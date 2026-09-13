use super::AuthorizationProof;
use hbb_common::{
    anyhow::{anyhow, bail},
    base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _},
    config::Config,
    ResultType,
};
use std::{
    fs,
    io::{BufRead, BufReader, Read, Write},
    os::unix::{
        fs::{FileTypeExt, MetadataExt},
        net::UnixStream,
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};

const SOCKET: &str = "/run/relaisdesk-fleet/proof.sock";
const PROTOCOL: &str = "relaisdesk-fleet-proof-v1";

#[derive(serde::Deserialize)]
struct Response {
    token: String,
    timestamp: i64,
    nonce: String,
    signature: String,
}

pub(super) fn authorization(action: &str) -> ResultType<AuthorizationProof> {
    if Config::get_option("relaisdesk-proof-socket") != SOCKET
        || action.is_empty()
        || action.len() > 512
        || action.contains(['\r', '\n', '\0'])
    {
        bail!("Invalid RelaisDesk fleet authorization request");
    }
    for path in ["/run", "/run/relaisdesk-fleet"] {
        let meta = fs::symlink_metadata(path)?;
        if !meta.is_dir() || meta.uid() != 0 || meta.mode() & 0o022 != 0 {
            bail!("Untrusted RelaisDesk fleet socket directory");
        }
    }
    let meta = fs::symlink_metadata(SOCKET)?;
    if !meta.file_type().is_socket() || meta.uid() != 0 {
        bail!("Untrusted RelaisDesk fleet socket");
    }
    let mut stream = UnixStream::connect(SOCKET)?;
    stream.set_read_timeout(Some(Duration::from_secs(2)))?;
    stream.set_write_timeout(Some(Duration::from_secs(2)))?;
    let request = serde_json::json!({"protocol": PROTOCOL, "action": action});
    serde_json::to_writer(&mut stream, &request)?;
    stream.write_all(b"\n")?;
    let mut response = String::new();
    BufReader::new(stream.take(16 * 1024)).read_line(&mut response)?;
    if !response.ends_with('\n') || response.len() >= 16 * 1024 {
        bail!("Invalid RelaisDesk fleet response size");
    }
    let response: Response = serde_json::from_str(&response)?;
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() as i64;
    if response.token.len() > 8192
        || !response.token.starts_with("rd1.")
        || response.token.chars().any(char::is_whitespace)
        || response.timestamp < now - 30
        || response.timestamp > now + 30
        || response.nonce.len() != 32
        || !response.nonce.bytes().all(|b| b.is_ascii_hexdigit())
    {
        bail!("Invalid RelaisDesk fleet proof");
    }
    let signature = URL_SAFE_NO_PAD
        .decode(response.signature.as_bytes())
        .map_err(|_| anyhow!("Invalid RelaisDesk fleet signature"))?;
    if signature.len() != 64 {
        bail!("Invalid RelaisDesk fleet signature length");
    }
    Ok(AuthorizationProof {
        token: response.token,
        timestamp: response.timestamp,
        nonce: response.nonce,
        signature,
    })
}
