use hbb_common::{
    anyhow::{anyhow, bail},
    config::Config,
    log, tokio, ResultType,
};
use serde::{Deserialize, Serialize};
use std::{
    path::Path,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};

#[derive(Deserialize)]
struct Binding {
    endpoint: String,
    secret: String,
    peer: String,
}

#[derive(Serialize)]
struct Event<'a> {
    event: &'a str,
}

#[derive(Default)]
struct State {
    connected: bool,
    finished: bool,
}

// Tracker reports only a successfully established control session and its
// closure. It deliberately has no access to the API token; a local launcher
// bridge owns that token and authorizes the final server request.
pub(crate) struct Tracker {
    state: Arc<Mutex<State>>,
    failed: Arc<AtomicBool>,
    wake: Arc<tokio::sync::Notify>,
    task: Option<tokio::task::JoinHandle<()>>,
}

impl Tracker {
    pub(crate) async fn open(peer: &str) -> ResultType<Option<Self>> {
        if !(6..=16).contains(&peer.len()) || !peer.bytes().all(|c| c.is_ascii_digit()) {
            return Ok(None);
        }
        let token = Config::get_option("relaisdesk-token-file");
        if token.is_empty() {
            return Ok(None);
        }
        let parent = Path::new(&token)
            .parent()
            .ok_or_else(|| anyhow!("Invalid token directory"))?;
        let path = parent.join("interventions").join(format!("{peer}.json"));
        let info = match tokio::fs::symlink_metadata(&path).await {
            Ok(info) => info,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e.into()),
        };
        if !info.is_file() || info.file_type().is_symlink() || info.len() > 4096 {
            bail!("Invalid intervention binding");
        }
        let binding: Binding = serde_json::from_slice(&tokio::fs::read(path).await?)?;
        let url = reqwest::Url::parse(&binding.endpoint)?;
        if binding.peer != peer
            || binding.secret.len() != 64
            || !binding.secret.bytes().all(|c| c.is_ascii_hexdigit())
            || url.scheme() != "http"
            || url.host_str() != Some("127.0.0.1")
            || url.port().is_none()
            || url.path() != "/intervention"
            || !url.username().is_empty()
            || url.password().is_some()
            || url.query().is_some()
            || url.fragment().is_some()
        {
            bail!("Invalid intervention endpoint");
        }
        Ok(Some(spawn_tracker(binding)?))
    }

    pub(crate) fn confirm(&self) {
        let mut state = self.state.lock().unwrap();
        if !state.connected {
            state.connected = true;
            self.wake.notify_one();
        }
    }

    /// Signal session end and wait (bounded) for the final event to reach the
    /// bridge, so a disconnect closes the intervention even when the engine
    /// process exits right after. Sessions that never connected report
    /// nothing; the bridge settles them when it stops.
    pub(crate) async fn close(mut self) {
        let connected = {
            let mut state = self.state.lock().unwrap();
            state.finished = true;
            state.connected
        };
        self.wake.notify_one();
        if connected {
            if let Some(handle) = self.task.take() {
                let _ = tokio::time::timeout(Duration::from_secs(5), handle).await;
            }
        }
    }

    pub(crate) fn failed(&self) -> bool {
        self.failed.load(Ordering::SeqCst)
    }
}

fn spawn_tracker(binding: Binding) -> ResultType<Tracker> {
    let client = reqwest::Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(4))
        .build()?;
    let state = Arc::new(Mutex::new(State::default()));
    let failed = Arc::new(AtomicBool::new(false));
    let wake = Arc::new(tokio::sync::Notify::new());
    let task = tokio::spawn({
        let state = Arc::clone(&state);
        let failed = Arc::clone(&failed);
        let wake = Arc::clone(&wake);
        async move {
            let mut sent_connected = false;
            loop {
                let (connected, finished) = {
                    let state = state.lock().unwrap();
                    (state.connected, state.finished)
                };
                if finished && !connected {
                    // Never connected: nothing to report. The bridge settles
                    // unconnected interventions when it stops.
                    break;
                }
                let event = if finished {
                    "closed"
                } else if connected && !sent_connected {
                    "connected"
                } else {
                    wake.notified().await;
                    continue;
                };
                let mut delivered = false;
                for attempt in 0..3 {
                    let response = client
                        .post(&binding.endpoint)
                        .bearer_auth(&binding.secret)
                        .json(&Event { event })
                        .send()
                        .await;
                    if matches!(response, Ok(ref response) if response.status().is_success()) {
                        delivered = true;
                        break;
                    }
                    if attempt < 2 {
                        tokio::time::sleep(Duration::from_millis(250)).await;
                    }
                }
                if !delivered {
                    failed.store(true, Ordering::SeqCst);
                    log::warn!(
                        "RelaisDesk intervention tracking unavailable; closing remote session"
                    );
                    break;
                }
                if event == "connected" {
                    sent_connected = true;
                }
                if finished {
                    break;
                }
            }
        }
    });
    Ok(Tracker {
        state,
        failed,
        wake,
        task: Some(task),
    })
}

impl Drop for Tracker {
    fn drop(&mut self) {
        self.state.lock().unwrap().finished = true;
        self.wake.notify_one();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{BufRead, BufReader, Read, Write};
    use std::sync::mpsc;

    // Minimal stub of the launcher bridge: records event bodies, always 200.
    fn stub_bridge() -> (String, mpsc::Receiver<String>) {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let endpoint = format!("http://{}/intervention", listener.local_addr().unwrap());
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                let mut reader = BufReader::new(stream);
                let mut content_length = 0usize;
                loop {
                    let mut line = String::new();
                    if reader.read_line(&mut line).unwrap_or(0) == 0 {
                        break;
                    }
                    if line.trim().is_empty() {
                        break;
                    }
                    if let Some(value) = line
                        .trim()
                        .strip_prefix("Content-Length:")
                        .or_else(|| line.trim().strip_prefix("content-length:"))
                    {
                        content_length = value.trim().parse().unwrap_or(0);
                    }
                }
                let mut body = vec![0u8; content_length];
                if reader.read_exact(&mut body).is_err() {
                    continue;
                }
                let _ = tx.send(String::from_utf8(body).unwrap_or_default());
                let _ = reader
                    .get_mut()
                    .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
            }
        });
        (endpoint, rx)
    }

    fn test_binding(endpoint: String) -> Binding {
        Binding {
            endpoint,
            secret: "00".repeat(32),
            peer: "123456789".to_owned(),
        }
    }

    // Blocking channel receives run off the single-threaded test runtime.
    async fn recv_event(
        rx: &Arc<std::sync::Mutex<mpsc::Receiver<String>>>,
        timeout: Duration,
    ) -> Option<String> {
        let rx = Arc::clone(rx);
        tokio::task::spawn_blocking(move || rx.lock().unwrap().recv_timeout(timeout).ok())
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn close_after_confirm_reports_connected_then_closed() {
        let (endpoint, rx) = stub_bridge();
        let rx = Arc::new(std::sync::Mutex::new(rx));
        let tracker = spawn_tracker(test_binding(endpoint)).unwrap();
        tracker.confirm();
        assert_eq!(
            recv_event(&rx, Duration::from_secs(10)).await.as_deref(),
            Some(r#"{"event":"connected"}"#)
        );
        tracker.close().await;
        assert_eq!(
            recv_event(&rx, Duration::from_secs(10)).await.as_deref(),
            Some(r#"{"event":"closed"}"#)
        );
    }

    #[tokio::test]
    async fn close_without_confirm_reports_nothing() {
        let (endpoint, rx) = stub_bridge();
        let rx = Arc::new(std::sync::Mutex::new(rx));
        let tracker = spawn_tracker(test_binding(endpoint)).unwrap();
        tracker.close().await;
        assert!(recv_event(&rx, Duration::from_secs(1)).await.is_none());
    }
}
