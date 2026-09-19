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
    time::{Duration, Instant},
};

#[path = "relaisdesk_meter_clock.rs"]
mod clock;

#[derive(Deserialize)]
struct Binding {
    endpoint: String,
    secret: String,
    peer: String,
}
#[derive(Serialize)]
struct Pulse<'a> {
    kind: &'a str,
    connection: &'a str,
    sequence: u64,
    cumulative_ms: u64,
    closed: bool,
}
#[derive(Default)]
struct State {
    clock: clock::ConnectedClock,
    finished: bool,
}

pub(crate) struct Meter {
    state: Arc<Mutex<State>>,
    failed: Arc<AtomicBool>,
    wake: Arc<tokio::sync::Notify>,
}
impl Meter {
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
        let path = parent.join("prestations").join(format!("{peer}.json"));
        let info = match tokio::fs::symlink_metadata(&path).await {
            Ok(info) => info,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e.into()),
        };
        if !info.is_file() || info.file_type().is_symlink() || info.len() > 4096 {
            bail!("Invalid prestation binding");
        }
        let binding: Binding = serde_json::from_slice(&tokio::fs::read(path).await?)?;
        let url = reqwest::Url::parse(&binding.endpoint)?;
        if binding.peer != peer
            || binding.secret.len() != 64
            || !binding.secret.bytes().all(|c| c.is_ascii_hexdigit())
            || url.scheme() != "http"
            || url.host_str() != Some("127.0.0.1")
            || url.port().is_none()
            || url.path() != "/meter"
            || !url.username().is_empty()
            || url.password().is_some()
            || url.query().is_some()
            || url.fragment().is_some()
        {
            bail!("Invalid prestation endpoint");
        }
        let client = reqwest::Client::builder()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(4))
            .build()?;
        let state = Arc::new(Mutex::new(State::default()));
        let failed = Arc::new(AtomicBool::new(false));
        let wake = Arc::new(tokio::sync::Notify::new());
        let result = Self {
            state: Arc::clone(&state),
            failed: Arc::clone(&failed),
            wake: Arc::clone(&wake),
        };
        tokio::spawn(async move {
            let connection = uuid::Uuid::new_v4().simple().to_string();
            let mut sequence = 0;
            let mut baseline = 0;
            loop {
                let (started, millis, finished) = {
                    let s = state.lock().unwrap();
                    (s.clock.started(), s.clock.millis(), s.finished)
                };
                if finished && sequence == 0 {
                    break;
                }
                if started {
                    sequence += 1;
                }
                if sequence == 1 {
                    baseline = millis;
                }
                let body = Pulse {
                    kind: if started { "pulse" } else { "ready" },
                    connection: &connection,
                    sequence,
                    cumulative_ms: millis.saturating_sub(baseline),
                    closed: finished && sequence > 1,
                };
                let sent = client
                    .post(&binding.endpoint)
                    .bearer_auth(&binding.secret)
                    .json(&body)
                    .send()
                    .await;
                if !matches!(sent, Ok(ref response) if response.status().is_success()) {
                    failed.store(true, Ordering::SeqCst);
                    log::warn!(
                        "RelaisDesk prestation metering unavailable; closing remote session"
                    );
                    break;
                }
                if finished {
                    break;
                }
                tokio::select! { _ = tokio::time::sleep(Duration::from_secs(2)) => {}, _ = wake.notified() => {} }
            }
        });
        Ok(Some(result))
    }
    pub(crate) fn confirm(&self) {
        let mut state = self.state.lock().unwrap();
        let first = !state.clock.started();
        state.clock.confirm(Instant::now());
        if first {
            self.wake.notify_one();
        }
    }
    pub(crate) fn failed(&self) -> bool {
        self.failed.load(Ordering::SeqCst)
    }
}
impl Drop for Meter {
    fn drop(&mut self) {
        self.state.lock().unwrap().finished = true;
        self.wake.notify_one();
    }
}
