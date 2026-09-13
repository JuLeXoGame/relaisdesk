use hbb_common::{
    anyhow::{anyhow, bail},
    base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _},
    message_proto::NetworkAuthorization,
    sodiumoxide::crypto::sign,
    ResultType,
};
use serde::Deserialize;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

#[derive(Deserialize)]
struct Claims {
    iss: String,
    aud: String,
    tenant: String,
    role: String,
    device_public_key: String,
    iat: i64,
    nbf: i64,
    exp: i64,
    #[serde(default)]
    verification_key: String,
    #[serde(default)]
    peer_auth_version: u32,
    #[serde(default)]
    folder_access: Vec<String>,
    #[serde(default)]
    folder_path: Vec<String>,
}

pub(crate) struct PeerLease {
    device_key: String,
    expires_at: i64,
    refreshed: Instant,
    pub(crate) restricted: bool,
}

impl PeerLease {
    pub(crate) fn is_current(&self) -> bool {
        self.refreshed.elapsed().as_secs() < 30 && now().map(|n| n < self.expires_at).unwrap_or(false)
    }
}

fn now() -> ResultType<i64> {
    Ok(SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() as i64)
}

fn decode(token: &str) -> ResultType<(Claims, String, Vec<u8>)> {
    if token.len() > 8192 { bail!("Invalid authorization size"); }
    let parts: Vec<_> = token.split('.').collect();
    if parts.len() != 3 || parts[0] != "rd1" { bail!("Invalid authorization format"); }
    let payload = URL_SAFE_NO_PAD.decode(parts[1])?;
    if payload.len() > 4096 { bail!("Invalid authorization payload"); }
    Ok((serde_json::from_slice(&payload)?, format!("rd1.{}", parts[1]), URL_SAFE_NO_PAD.decode(parts[2])?))
}

fn verify_token(token: &str, key: &sign::PublicKey, at: i64) -> ResultType<Claims> {
    let (claims, signed, signature) = decode(token)?;
    let signature = sign::Signature::from_bytes(&signature).map_err(|_| anyhow!("Invalid authorization signature"))?;
    if !sign::verify_detached(&signature, signed.as_bytes(), key)
        || claims.iss != "relaisdesk-api" || claims.aud != "rustdesk-network"
        || claims.tenant.is_empty() || claims.tenant.len() > 128
        || claims.iat > claims.nbf || claims.nbf > claims.exp
        || claims.exp.saturating_sub(claims.iat) > 900
        || at.saturating_add(30) < claims.nbf || at >= claims.exp
        || claims.folder_access.len() > 100 || claims.folder_path.len() > 32
    { bail!("Authorization rejected"); }
    Ok(claims)
}

// local_token comes only from the protected launcher/broker file, obtained over HTTPS.
pub(crate) fn verify_peer(local_token: &str, proof: Option<&NetworkAuthorization>, challenge: &str, previous: Option<&PeerLease>) -> ResultType<Option<PeerLease>> {
    let (local, _, _) = decode(local_token)?;
    if local.peer_auth_version == 0 && previous.is_none() { return Ok(None); }
    if local.peer_auth_version != 1 || local.role != "viewer" { bail!("Peer authorization required"); }
    let key_bytes = URL_SAFE_NO_PAD.decode(&local.verification_key)?;
    let key = sign::PublicKey::from_slice(&key_bytes).ok_or_else(|| anyhow!("Invalid trusted verification key"))?;
    let at = now()?;
    let local = verify_token(local_token, &key, at)?;
    let proof = proof.ok_or_else(|| anyhow!("Update RelaisDesk Technicien: peer authorization is required"))?;
    if challenge.len() < 16 || challenge.len() > 128 || proof.nonce.len() < 16 || proof.nonce.len() > 128
        || at.abs_diff(proof.timestamp) > 30 { bail!("Invalid peer authorization challenge"); }
    let peer = verify_token(&proof.token, &key, at)?;
    if peer.tenant != local.tenant || (peer.role != "technician" && peer.role != "folder_technician")
        || peer.device_public_key == local.device_public_key
        || (peer.role == "folder_technician" && !peer.folder_access.iter().any(|id| local.folder_path.contains(id)))
        || previous.map(|old| old.device_key != peer.device_public_key).unwrap_or(false)
    { bail!("This computer is outside the authorized folders"); }
    let device_bytes = URL_SAFE_NO_PAD.decode(&peer.device_public_key)?;
    let device_key = sign::PublicKey::from_slice(&device_bytes).ok_or_else(|| anyhow!("Invalid peer device key"))?;
    let signature = sign::Signature::from_bytes(proof.signature.as_ref()).map_err(|_| anyhow!("Invalid peer proof signature"))?;
    let action = format!("session:{challenge}");
    let message = format!("relaisdesk-proof-v1\n{action}\n{}\n{}\n{}", proof.timestamp, proof.nonce, proof.token);
    if !sign::verify_detached(&signature, message.as_bytes(), &device_key) { bail!("Invalid peer proof"); }
    Ok(Some(PeerLease { device_key: peer.device_public_key, expires_at: peer.exp.min(local.exp), refreshed: Instant::now(), restricted: peer.role == "folder_technician" }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn token(api: &sign::SecretKey, key: &sign::PublicKey, device: &sign::PublicKey, role: &str, tenant: &str, folders: Vec<&str>, expiry: i64) -> String {
        let at = now().unwrap();
        let payload = json!({"iss":"relaisdesk-api","aud":"rustdesk-network","tenant":tenant,"role":role,
            "device_public_key":URL_SAFE_NO_PAD.encode(device.as_ref()),"verification_key":URL_SAFE_NO_PAD.encode(key.as_ref()),
            "iat":at-1,"nbf":at-1,"exp":expiry,"peer_auth_version":1,"folder_path":["FLD-AAAA-BBBB"],"folder_access":folders});
        let signed = format!("rd1.{}",URL_SAFE_NO_PAD.encode(serde_json::to_vec(&payload).unwrap()));
        let sig = sign::sign_detached(signed.as_bytes(),api);
        format!("{signed}.{}",URL_SAFE_NO_PAD.encode(sig.to_bytes()))
    }
    fn proof(token: String, device: &sign::SecretKey, challenge: &str) -> NetworkAuthorization {
        let timestamp = now().unwrap();
        let nonce = "0123456789abcdef0123456789abcdef".to_owned();
        let signed = format!("relaisdesk-proof-v1\nsession:{challenge}\n{timestamp}\n{nonce}\n{token}");
        NetworkAuthorization { token, timestamp, nonce, signature: sign::sign_detached(signed.as_bytes(), device).to_bytes().to_vec().into(), ..Default::default() }
    }
    #[test]
    fn session_acl_proof_and_replay_are_enforced_on_the_viewer() {
        hbb_common::sodiumoxide::init().unwrap();
        let (key, api) = sign::gen_keypair();
        let (viewer, _) = sign::gen_keypair();
        let (tech, tech_secret) = sign::gen_keypair();
        let challenge="random-per-connection-challenge";
        let local=token(&api,&key,&viewer,"viewer","tenant",vec![],now().unwrap()+300);
        let authorized=token(&api,&key,&tech,"folder_technician","tenant",vec!["FLD-AAAA-BBBB"],now().unwrap()+60);
        let p=proof(authorized,&tech_secret,challenge);
        let lease=verify_peer(&local,Some(&p),challenge,None).unwrap().unwrap();
        assert!(lease.is_current());
        assert!(verify_peer(&local,Some(&p),"another-connection-challenge",None).is_err());
        assert!(verify_peer(&local,None,challenge,None).is_err());
        for (tenant,folders) in [("foreign",vec!["FLD-AAAA-BBBB"]),("tenant",vec![]),("tenant",vec!["FLD-CCCC-DDDD"])] {
            let p=proof(token(&api,&key,&tech,"folder_technician",tenant,folders,now().unwrap()+60),&tech_secret,challenge);
            assert!(verify_peer(&local,Some(&p),challenge,Some(&lease)).is_err());
        }
        let expired=proof(token(&api,&key,&tech,"folder_technician","tenant",vec!["FLD-AAAA-BBBB"],now().unwrap()-1),&tech_secret,challenge);
        assert!(verify_peer(&local,Some(&expired),challenge,None).is_err());
        let (_, attacker)=sign::gen_keypair();
        let forged=proof(token(&attacker,&key,&tech,"technician","tenant",vec![],now().unwrap()+60),&tech_secret,challenge);
        assert!(verify_peer(&local,Some(&forged),challenge,None).is_err());
    }
}
