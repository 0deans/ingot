use super::super::{
    client::Endpoints,
    clock::{ProtocolClock, Sleep},
    MicrosoftDeviceCode, MicrosoftSession,
};
use ingot_test_support::{LoopbackServer, ResponseSpec, ScriptedExchange};
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};

pub(super) const DEVICE: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/microsoft-device-code.json"
));
pub(super) const TOKEN: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/microsoft-token.json"
));
pub(super) const XBOX: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/microsoft-xbox.json"
));
pub(super) const MINECRAFT: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/microsoft-minecraft-token.json"
));
pub(super) const PROFILE: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/microsoft-profile.json"
));

#[derive(Default)]
struct Time {
    elapsed: Duration,
    waits: Vec<Duration>,
}
#[derive(Default)]
pub(super) struct Clock {
    time: Mutex<Time>,
    pub cancel_on_sleep: Mutex<Option<Arc<AtomicBool>>>,
}
impl Clock {
    pub fn waits(&self) -> Vec<Duration> {
        self.time.lock().unwrap().waits.clone()
    }
    pub fn advance(&self, duration: Duration) {
        self.time.lock().unwrap().elapsed += duration;
    }
}
impl ProtocolClock for Clock {
    fn elapsed(&self) -> Duration {
        self.time.lock().unwrap().elapsed
    }
    fn unix_seconds(&self) -> u64 {
        1000 + self.elapsed().as_secs()
    }
    fn sleep(&self, duration: Duration) -> Sleep<'_> {
        Box::pin(async move {
            let mut time = self.time.lock().unwrap();
            time.elapsed += duration;
            time.waits.push(duration);
            if let Some(flag) = self.cancel_on_sleep.lock().unwrap().as_ref() {
                flag.store(true, Ordering::SeqCst);
            }
        })
    }
}

pub(super) struct Reply {
    pub path: &'static str,
    pub status: u16,
    pub body: String,
    pub effect: Option<Box<dyn FnOnce() + Send>>,
}
pub(super) fn reply(path: &'static str, status: u16, body: &str) -> Reply {
    Reply {
        path,
        status,
        body: body.to_owned(),
        effect: None,
    }
}
pub(super) fn chain() -> Vec<Reply> {
    vec![
        reply("/token", 200, TOKEN),
        reply("/xbox", 200, XBOX),
        reply("/xsts", 200, XBOX),
        reply("/minecraft", 200, MINECRAFT),
        reply("/profile", 200, PROFILE),
    ]
}
pub(super) fn code() -> MicrosoftDeviceCode {
    MicrosoftDeviceCode {
        user_code: "FIXTURE".to_owned(),
        verification_uri: "https://fixture.invalid/link".to_owned(),
        device_code: "fixture-device".to_owned(),
        interval: 2,
        expires_in: 900,
    }
}

pub(super) struct Request {
    pub headers: String,
    pub body: String,
}
pub(super) struct Server {
    inner: LoopbackServer,
}
impl Server {
    pub async fn new(script: Vec<Reply>) -> Self {
        let exchanges = script
            .into_iter()
            .map(|reply| {
                let mut spec = if reply.status == 0 {
                    ResponseSpec::close_connection()
                } else {
                    ResponseSpec::json(reply.status, reply.body.into_bytes())
                };
                if let Some(effect) = reply.effect {
                    let slot = Arc::new(Mutex::new(Some(effect)));
                    spec = spec.with_effect(move |_| {
                        if let Some(callback) = slot.lock().unwrap().take() {
                            callback();
                        }
                    });
                }
                ScriptedExchange::expect_path(reply.path, spec)
            })
            .collect();
        Self {
            inner: LoopbackServer::exact_script(exchanges).await,
        }
    }
    pub fn session(&self, clock: Arc<Clock>, client_id: Option<String>) -> MicrosoftSession {
        let endpoint = |path: &str| self.inner.url(path);
        let endpoints = Endpoints {
            device_code: endpoint("/device"),
            token: endpoint("/token"),
            xbox: endpoint("/xbox"),
            xsts: endpoint("/xsts"),
            minecraft_login: endpoint("/minecraft"),
            profile: endpoint("/profile"),
        };
        let client = reqwest::Client::builder().no_proxy().build().unwrap();
        MicrosoftSession::new(client, endpoints, client_id, clock)
    }
    pub async fn requests(self) -> Vec<Request> {
        self.inner
            .finish()
            .await
            .into_iter()
            .map(|captured| Request {
                headers: captured.raw_headers.clone(),
                body: captured.body_str().into_owned(),
            })
            .collect()
    }
}
