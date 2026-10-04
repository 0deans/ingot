use super::super::{
    client::Endpoints,
    clock::{ProtocolClock, Sleep},
    MicrosoftDeviceCode, MicrosoftSession,
};
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    task::JoinHandle,
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
    base: String,
    task: JoinHandle<Vec<Request>>,
}
impl Server {
    pub async fn new(script: Vec<Reply>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let task = tokio::spawn(async move {
            let mut requests = Vec::new();
            for reply in script {
                let (mut stream, _) = listener.accept().await.unwrap();
                let mut bytes = Vec::new();
                let mut buffer = [0; 2048];
                let (headers, offset, length) = loop {
                    let count = stream.read(&mut buffer).await.unwrap();
                    assert!(count > 0);
                    bytes.extend_from_slice(&buffer[..count]);
                    if let Some(offset) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
                        let headers = String::from_utf8(bytes[..offset].to_vec()).unwrap();
                        let length = headers
                            .lines()
                            .find_map(|line| {
                                let (name, value) = line.split_once(':')?;
                                name.eq_ignore_ascii_case("content-length")
                                    .then(|| value.trim().parse::<usize>().unwrap())
                            })
                            .unwrap_or(0);
                        break (headers, offset + 4, length);
                    }
                };
                while bytes.len() < offset + length {
                    let count = stream.read(&mut buffer).await.unwrap();
                    assert!(count > 0);
                    bytes.extend_from_slice(&buffer[..count]);
                }
                assert!(
                    headers
                        .lines()
                        .next()
                        .unwrap()
                        .split_whitespace()
                        .nth(1)
                        .unwrap()
                        == reply.path
                );
                requests.push(Request {
                    headers,
                    body: String::from_utf8(bytes[offset..offset + length].to_vec()).unwrap(),
                });
                if let Some(effect) = reply.effect {
                    effect();
                }
                if reply.status == 0 {
                    drop(stream);
                    continue;
                }
                let response = format!("HTTP/1.1 {} Fixture\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", reply.status, reply.body.len(), reply.body);
                stream.write_all(response.as_bytes()).await.unwrap();
                stream.shutdown().await.unwrap();
            }
            requests
        });
        Self { base, task }
    }
    pub fn session(&self, clock: Arc<Clock>, client_id: Option<String>) -> MicrosoftSession {
        let endpoint = |path: &str| format!("{}{path}", self.base);
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
        tokio::time::timeout(Duration::from_secs(5), self.task)
            .await
            .unwrap()
            .unwrap()
    }
}
