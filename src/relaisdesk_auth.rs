use hbb_common::{
    anyhow::{anyhow, bail},
    base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _},
    config::Config,
    sodiumoxide::crypto::sign,
    ResultType,
};
use std::{fs, path::Path, time::SystemTime};
use uuid::Uuid;

#[cfg(target_os = "linux")]
#[path = "relaisdesk_fleet_linux.rs"]
mod fleet_linux;

const TOKEN_FILE_OPTION: &str = "relaisdesk-token-file";
const PROOF_KEY_FILE_OPTION: &str = "relaisdesk-proof-key-file";
const MAX_TOKEN_BYTES: u64 = 8 * 1024;
const MAX_KEY_FILE_BYTES: u64 = 256;

#[path = "relaisdesk_peer_auth.rs"]
mod peer_auth;
pub(crate) use peer_auth::PeerLease;

pub(crate) fn session_proof(challenge: &str) -> ResultType<Option<hbb_common::message_proto::NetworkAuthorization>> {
    if challenge.is_empty() { return Ok(None); }
    Ok(authorization(&format!("session:{challenge}"))?.map(|auth| hbb_common::message_proto::NetworkAuthorization {
        token: auth.token, timestamp: auth.timestamp, nonce: auth.nonce, signature: auth.signature.into(), ..Default::default()
    }))
}

pub(crate) fn verify_peer(proof: Option<&hbb_common::message_proto::NetworkAuthorization>, challenge: &str, previous: Option<&PeerLease>) -> ResultType<Option<PeerLease>> {
    match authorization("peer-policy")? {
        Some(local) => peer_auth::verify_peer(&local.token, proof, challenge, previous),
        None if previous.is_none() => Ok(None),
        None => bail!("Local RelaisDesk authorization disappeared"),
    }
}

pub(crate) struct AuthorizationProof {
    pub(crate) token: String,
    pub(crate) timestamp: i64,
    pub(crate) nonce: String,
    pub(crate) signature: Vec<u8>,
}

pub(crate) fn is_configured() -> bool {
    #[cfg(target_os = "linux")]
    if !Config::get_option("relaisdesk-proof-socket").trim().is_empty() {
        return true;
    }
    !Config::get_option(TOKEN_FILE_OPTION).trim().is_empty()
        || !Config::get_option(PROOF_KEY_FILE_OPTION).trim().is_empty()
}

pub(crate) fn authorization(action: &str) -> ResultType<Option<AuthorizationProof>> {
    #[cfg(target_os = "linux")]
    if !Config::get_option("relaisdesk-proof-socket").trim().is_empty() {
        return fleet_linux::authorization(action).map(Some);
    }
    let token_path = Config::get_option(TOKEN_FILE_OPTION);
    let key_path = Config::get_option(PROOF_KEY_FILE_OPTION);
    if token_path.trim().is_empty() && key_path.trim().is_empty() {
        return Ok(None);
    }
    if token_path.trim().is_empty() || key_path.trim().is_empty() {
        bail!("RelaisDesk authorization configuration is incomplete");
    }
    if action.is_empty() || action.len() > 512 || action.contains('\n') {
        bail!("Invalid RelaisDesk authorization action");
    }

    let token = read_secret(Path::new(token_path.trim()), MAX_TOKEN_BYTES, "token")?;
    if token.is_empty() || token.chars().any(char::is_whitespace) {
        bail!("Invalid RelaisDesk authorization token");
    }

    let encoded_key = read_secret(Path::new(key_path.trim()), MAX_KEY_FILE_BYTES, "proof key")?;
    let key = URL_SAFE_NO_PAD
        .decode(encoded_key.as_bytes())
        .map_err(|_| anyhow!("Invalid RelaisDesk proof key encoding"))?;
    let secret_key = sign::SecretKey::from_slice(&key)
        .ok_or_else(|| anyhow!("Invalid RelaisDesk proof key length"))?;

    let timestamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_err(|_| anyhow!("System clock is before the Unix epoch"))?
        .as_secs() as i64;
    let nonce = Uuid::new_v4().simple().to_string();
    let signed = proof_message(action, &token, timestamp, &nonce);
    let signature = sign::sign_detached(signed.as_bytes(), &secret_key)
        .to_bytes()
        .to_vec();

    Ok(Some(AuthorizationProof {
        token,
        timestamp,
        nonce,
        signature,
    }))
}

fn proof_message(action: &str, token: &str, timestamp: i64, nonce: &str) -> String {
    format!("relaisdesk-proof-v1\n{action}\n{timestamp}\n{nonce}\n{token}")
}

fn read_secret(path: &Path, max_bytes: u64, label: &str) -> ResultType<String> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|_| anyhow!("RelaisDesk {label} file is unavailable"))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() || metadata.len() > max_bytes {
        bail!("Invalid RelaisDesk {label} file");
    }
    let value =
        fs::read_to_string(path).map_err(|_| anyhow!("RelaisDesk {label} file is unreadable"))?;
    Ok(value.trim().to_owned())
}

#[cfg(test)]
mod tests {
    use super::proof_message;

    #[test]
    fn proof_message_is_unambiguous() {
        assert_eq!(
            proof_message("punch:123", "rd1.payload.signature", 42, "nonce"),
            "relaisdesk-proof-v1\npunch:123\n42\nnonce\nrd1.payload.signature"
        );
    }
}
