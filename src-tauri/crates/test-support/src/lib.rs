//! Development-only loopback HTTP server fixture for offline workspace tests.

use std::collections::VecDeque;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::watch;
use tokio::task::{JoinHandle, JoinSet};

const MAX_HEADER_BYTES: usize = 64 * 1024;
const FINISH_TIMEOUT: Duration = Duration::from_secs(5);

/// A captured HTTP request received by [`LoopbackServer`].
#[derive(Debug, Clone)]
pub struct CapturedRequest {
    pub method: String,
    pub path: String,
    pub raw_headers: String,
    pub raw_request: String,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

impl CapturedRequest {
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }

    pub fn body_str(&self) -> std::borrow::Cow<'_, str> {
        String::from_utf8_lossy(&self.body)
    }
}

/// Callback executed when a loopback response handles a captured request.
pub type RequestEffect = Arc<dyn Fn(&CapturedRequest) + Send + Sync>;

/// Optional response timing or transport behavior for [`ResponseSpec`].
#[derive(Clone, Default)]
pub struct ResponseSpec {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
    pub close_without_response: bool,
    pub stall: bool,
    pub delay: Option<Duration>,
    pub chunked: Option<(usize, Duration)>,
    pub effect: Option<RequestEffect>,
}

impl ResponseSpec {
    pub fn json(status: u16, body: impl Into<Vec<u8>>) -> Self {
        Self {
            status,
            headers: vec![("Content-Type".into(), "application/json".into())],
            body: body.into(),
            ..Self::default()
        }
    }

    pub fn bytes(status: u16, body: impl Into<Vec<u8>>) -> Self {
        Self {
            status,
            headers: Vec::new(),
            body: body.into(),
            ..Self::default()
        }
    }

    pub fn text(status: u16, body: impl Into<String>) -> Self {
        Self::bytes(status, body.into().into_bytes())
    }

    pub fn status(status: u16) -> Self {
        Self::bytes(status, Vec::new())
    }

    /// Drops the TCP connection immediately after capturing the request.
    pub fn close_connection() -> Self {
        Self {
            close_without_response: true,
            ..Self::default()
        }
    }

    /// Keeps the TCP socket open without writing headers until server drop/cancellation.
    pub fn stall() -> Self {
        Self {
            stall: true,
            ..Self::default()
        }
    }

    pub fn with_header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.push((name.into(), value.into()));
        self
    }

    pub fn with_delay(mut self, delay: Duration) -> Self {
        self.delay = Some(delay);
        self
    }

    pub fn with_chunks(mut self, chunk_size: usize, interval: Duration) -> Self {
        self.chunked = Some((chunk_size.max(1), interval));
        self
    }

    pub fn with_effect<F>(mut self, effect: F) -> Self
    where
        F: Fn(&CapturedRequest) + Send + Sync + 'static,
    {
        self.effect = Some(Arc::new(effect));
        self
    }
}

impl From<(u16, String)> for ResponseSpec {
    fn from((status, body): (u16, String)) -> Self {
        Self::json(status, body.into_bytes())
    }
}

impl From<(u16, &'static str)> for ResponseSpec {
    fn from((status, body): (u16, &'static str)) -> Self {
        Self::json(status, body.as_bytes().to_vec())
    }
}

impl From<(u16, Vec<(String, String)>, Vec<u8>)> for ResponseSpec {
    fn from((status, headers, body): (u16, Vec<(String, String)>, Vec<u8>)) -> Self {
        Self {
            status,
            headers,
            body,
            ..Self::default()
        }
    }
}

/// One expected exchange in a strict sequential loopback script.
pub struct ScriptedExchange {
    pub expected_path: Option<String>,
    pub response: ResponseSpec,
}

impl ScriptedExchange {
    pub fn any_path(response: impl Into<ResponseSpec>) -> Self {
        Self {
            expected_path: None,
            response: response.into(),
        }
    }

    pub fn expect_path(path: impl Into<String>, response: impl Into<ResponseSpec>) -> Self {
        Self {
            expected_path: Some(path.into()),
            response: response.into(),
        }
    }
}

type RequestHandler = Arc<dyn Fn(&CapturedRequest) -> ResponseSpec + Send + Sync>;

/// Deterministic loopback HTTP server with request capture, failure surfacing, and drop cleanup.
pub struct LoopbackServer {
    addr: SocketAddr,
    base_url: String,
    requests: Arc<Mutex<Vec<CapturedRequest>>>,
    errors: Arc<Mutex<Vec<String>>>,
    pending_script: Option<Arc<Mutex<VecDeque<ScriptedExchange>>>>,
    shutdown_tx: watch::Sender<bool>,
    server_task: Option<JoinHandle<()>>,
}

impl Drop for LoopbackServer {
    fn drop(&mut self) {
        let _ = self.shutdown_tx.send(true);
        if let Some(task) = self.server_task.take() {
            task.abort();
        }
    }
}

impl LoopbackServer {
    /// Starts a server that consumes scripted `(status, json_body)` responses sequentially
    /// and responds with 404 after the queue empties.
    pub async fn scripted(responses: Vec<(u16, String)>) -> Self {
        let queue = Arc::new(Mutex::new(VecDeque::from(responses)));
        Self::with_handler(move |_| {
            let mut guard = lock_unpoisoned(&queue);
            if let Some((status, body)) = guard.pop_front() {
                ResponseSpec::json(status, body.into_bytes())
            } else {
                ResponseSpec::text(404, "Not Found")
            }
        })
        .await
    }

    /// Starts a server that strictly verifies each expected exchange in sequence and ensures all
    /// scripted exchanges were consumed when [`LoopbackServer::finish`] runs.
    pub async fn exact_script(exchanges: Vec<ScriptedExchange>) -> Self {
        let queue = Arc::new(Mutex::new(VecDeque::from(exchanges)));
        let pending = Arc::clone(&queue);
        let errors = Arc::new(Mutex::new(Vec::new()));
        let error_log = Arc::clone(&errors);

        let handler: RequestHandler = Arc::new(move |request| {
            let next = lock_unpoisoned(&queue).pop_front();
            match next {
                Some(exchange) => {
                    if let Some(expected) = exchange.expected_path {
                        if request.path != expected {
                            record_error(
                                &error_log,
                                format!(
                                    "unexpected request path: expected `{expected}`, got `{}`",
                                    request.path
                                ),
                            );
                        }
                    }
                    exchange.response
                }
                None => {
                    record_error(
                        &error_log,
                        format!(
                            "unexpected extra request: {} {}",
                            request.method, request.path
                        ),
                    );
                    ResponseSpec::text(500, "unexpected extra request")
                }
            }
        });

        Self::spawn_with(handler, errors, Some(pending)).await
    }

    /// Starts a server that returns a uniform status code and empty body for every request.
    pub async fn status_code(status: u16) -> Self {
        Self::with_handler(move |_| ResponseSpec::status(status)).await
    }

    /// Starts a server that delegates each captured request to `handler`.
    pub async fn with_handler<F, R>(handler: F) -> Self
    where
        F: Fn(&CapturedRequest) -> R + Send + Sync + 'static,
        R: Into<ResponseSpec>,
    {
        let handler: RequestHandler = Arc::new(move |req| handler(req).into());
        Self::spawn_with(handler, Arc::new(Mutex::new(Vec::new())), None).await
    }

    async fn spawn_with(
        handler: RequestHandler,
        errors: Arc<Mutex<Vec<String>>>,
        pending_script: Option<Arc<Mutex<VecDeque<ScriptedExchange>>>>,
    ) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .unwrap_or_else(|error| panic!("bind loopback fixture socket: {error}"));
        let addr = listener
            .local_addr()
            .unwrap_or_else(|error| panic!("read loopback fixture socket address: {error}"));
        let base_url = format!("http://{addr}");
        let requests = Arc::new(Mutex::new(Vec::new()));
        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        let context = AcceptContext {
            listener,
            requests: Arc::clone(&requests),
            errors: Arc::clone(&errors),
            handler,
            pending_script: pending_script.clone(),
            shutdown_rx,
        };
        let server_task = tokio::spawn(run_accept_loop(context));

        Self {
            addr,
            base_url,
            requests,
            errors,
            pending_script,
            shutdown_tx,
            server_task: Some(server_task),
        }
    }

    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    pub fn url(&self, path: &str) -> String {
        let trimmed = path.trim_start_matches('/');
        format!("{}/{}", self.base_url, trimmed)
    }

    pub fn captured_requests(&self) -> Vec<CapturedRequest> {
        lock_unpoisoned(&self.requests).clone()
    }

    /// Signals shutdown, drains in-flight connection tasks, surfaces any recorded handler/task
    /// failures, and verifies all strict-script exchanges were consumed.
    pub async fn finish(mut self) -> Vec<CapturedRequest> {
        let _ = self.shutdown_tx.send(true);
        if let Some(task) = self.server_task.take() {
            match tokio::time::timeout(FINISH_TIMEOUT, task).await {
                Ok(Ok(())) => {}
                Ok(Err(join_error)) if join_error.is_panic() => {
                    std::panic::resume_unwind(join_error.into_panic());
                }
                Ok(Err(join_error)) => {
                    record_error(
                        &self.errors,
                        format!("loopback server task failed: {join_error}"),
                    );
                }
                Err(_) => {
                    record_error(
                        &self.errors,
                        "timed out waiting for loopback server connections to finish".to_string(),
                    );
                }
            }
        }

        if let Some(pending) = &self.pending_script {
            let remaining = lock_unpoisoned(pending).len();
            if remaining > 0 {
                record_error(
                    &self.errors,
                    format!("{remaining} scripted HTTP exchange(s) were not consumed"),
                );
            }
        }

        let failures = lock_unpoisoned(&self.errors).clone();
        if !failures.is_empty() {
            panic!(
                "LoopbackServer recorded failure(s):\n{}",
                failures.join("\n")
            );
        }

        self.captured_requests()
    }
}

struct AcceptContext {
    listener: TcpListener,
    requests: Arc<Mutex<Vec<CapturedRequest>>>,
    errors: Arc<Mutex<Vec<String>>>,
    handler: RequestHandler,
    pending_script: Option<Arc<Mutex<VecDeque<ScriptedExchange>>>>,
    shutdown_rx: watch::Receiver<bool>,
}

async fn run_accept_loop(mut ctx: AcceptContext) {
    let mut workers = JoinSet::new();

    loop {
        if let Some(pending) = &ctx.pending_script {
            if lock_unpoisoned(pending).is_empty() {
                break;
            }
        }

        tokio::select! {
            biased;
            changed = ctx.shutdown_rx.changed() => {
                if changed.is_err() || *ctx.shutdown_rx.borrow() {
                    break;
                }
            }
            accepted = ctx.listener.accept() => {
                let (stream, _) = match accepted {
                    Ok(pair) => pair,
                    Err(error) => {
                        record_error(&ctx.errors, format!("loopback accept failed: {error}"));
                        break;
                    }
                };

                if ctx.pending_script.is_some() {
                    handle_connection(
                        stream,
                        &ctx.requests,
                        &ctx.errors,
                        &ctx.handler,
                        ctx.shutdown_rx.clone(),
                    )
                    .await;
                } else {
                    let requests = Arc::clone(&ctx.requests);
                    let errors = Arc::clone(&ctx.errors);
                    let handler = Arc::clone(&ctx.handler);
                    let shutdown_rx = ctx.shutdown_rx.clone();
                    workers.spawn(async move {
                        handle_connection(stream, &requests, &errors, &handler, shutdown_rx).await;
                    });
                }
            }
        }
    }

    drain_workers(&mut workers, &ctx.errors).await;
}

async fn drain_workers(workers: &mut JoinSet<()>, errors: &Arc<Mutex<Vec<String>>>) {
    let drain = async {
        while let Some(result) = workers.join_next().await {
            if let Err(join_error) = result {
                record_error(errors, format!("loopback worker panicked: {join_error}"));
            }
        }
    };
    if tokio::time::timeout(FINISH_TIMEOUT, drain).await.is_err() {
        workers.abort_all();
    }
}

async fn handle_connection(
    mut stream: TcpStream,
    requests: &Arc<Mutex<Vec<CapturedRequest>>>,
    errors: &Arc<Mutex<Vec<String>>>,
    handler: &RequestHandler,
    mut shutdown_rx: watch::Receiver<bool>,
) {
    let Some(request) = read_http_request(&mut stream, errors).await else {
        return;
    };

    let response =
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| handler(&request))) {
            Ok(spec) => spec,
            Err(_) => {
                record_error(
                    errors,
                    format!("handler panicked on {} {}", request.method, request.path),
                );
                ResponseSpec::text(500, "loopback handler panicked")
            }
        };

    if let Some(effect) = &response.effect {
        if std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| effect(&request))).is_err() {
            record_error(
                errors,
                format!("effect panicked on {} {}", request.method, request.path),
            );
        }
    }

    lock_unpoisoned(requests).push(request);

    if response.stall {
        let _ = shutdown_rx.wait_for(|flag| *flag).await;
        return;
    }

    if response.close_without_response {
        drop(stream);
        return;
    }

    if let Some(delay) = response.delay {
        tokio::select! {
            _ = tokio::time::sleep(delay) => {}
            _ = shutdown_rx.wait_for(|flag| *flag) => return,
        }
    }

    write_http_response(&mut stream, &response).await;
}

async fn read_http_request(
    stream: &mut TcpStream,
    errors: &Arc<Mutex<Vec<String>>>,
) -> Option<CapturedRequest> {
    let mut bytes = Vec::new();
    let mut buffer = [0u8; 4096];
    let mut header_end = None;
    let mut content_length = 0usize;

    loop {
        let read = match stream.read(&mut buffer).await {
            Ok(0) => break,
            Ok(n) => n,
            Err(error) => {
                if !is_expected_disconnect(&error) {
                    record_error(errors, format!("loopback read failed: {error}"));
                }
                return None;
            }
        };
        bytes.extend_from_slice(&buffer[..read]);

        if header_end.is_none() {
            if let Some(pos) = bytes.windows(4).position(|window| window == b"\r\n\r\n") {
                header_end = Some(pos);
                let headers_str = String::from_utf8_lossy(&bytes[..pos]);
                content_length = headers_str
                    .lines()
                    .find_map(|line| {
                        let (name, val) = line.split_once(':')?;
                        name.trim()
                            .eq_ignore_ascii_case("content-length")
                            .then(|| val.trim().parse::<usize>().ok())?
                    })
                    .unwrap_or(0);
            } else if bytes.len() > MAX_HEADER_BYTES {
                record_error(
                    errors,
                    "loopback request exceeded maximum header size".to_string(),
                );
                return None;
            }
        }

        if let Some(pos) = header_end {
            if bytes.len() >= pos + 4 + content_length {
                break;
            }
        }
    }

    let pos = header_end?;
    let body_end = (pos + 4 + content_length).min(bytes.len());
    let header_str = String::from_utf8_lossy(&bytes[..pos]).to_string();
    let body = bytes[pos + 4..body_end].to_vec();

    let mut lines = header_str.lines();
    let first_line = lines.next().unwrap_or("");
    let mut parts = first_line.split_whitespace();
    let method = parts.next().unwrap_or("GET").to_string();
    let path = parts.next().unwrap_or("/").to_string();

    let headers = lines
        .filter_map(|line| {
            let (key, value) = line.split_once(':')?;
            Some((key.trim().to_string(), value.trim().to_string()))
        })
        .collect();

    Some(CapturedRequest {
        method,
        path,
        raw_headers: header_str,
        raw_request: String::from_utf8_lossy(&bytes[..body_end]).to_string(),
        headers,
        body,
    })
}

async fn write_http_response(stream: &mut TcpStream, response: &ResponseSpec) {
    let mut has_content_length = false;
    let mut extra_headers = String::new();
    for (key, value) in &response.headers {
        if key.eq_ignore_ascii_case("content-length") {
            has_content_length = true;
        }
        extra_headers.push_str(&format!("{key}: {value}\r\n"));
    }

    let mut head = format!(
        "HTTP/1.1 {} Fixture\r\nConnection: close\r\n",
        response.status
    );
    if !has_content_length {
        head.push_str(&format!("Content-Length: {}\r\n", response.body.len()));
    }
    head.push_str(&extra_headers);
    head.push_str("\r\n");

    if stream.write_all(head.as_bytes()).await.is_err() {
        return;
    }

    if let Some((chunk_size, interval)) = response.chunked {
        for chunk in response.body.chunks(chunk_size) {
            if stream.write_all(chunk).await.is_err() {
                return;
            }
            tokio::time::sleep(interval).await;
        }
    } else if !response.body.is_empty() {
        let _ = stream.write_all(&response.body).await;
    }

    let _ = stream.shutdown().await;
}

fn is_expected_disconnect(error: &std::io::Error) -> bool {
    matches!(
        error.kind(),
        std::io::ErrorKind::ConnectionReset
            | std::io::ErrorKind::ConnectionAborted
            | std::io::ErrorKind::BrokenPipe
    )
}

fn record_error(errors: &Arc<Mutex<Vec<String>>>, message: String) {
    lock_unpoisoned(errors).push(message);
}

fn lock_unpoisoned<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    match mutex.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    async fn send_raw_request(addr: SocketAddr, path: &str) -> Vec<u8> {
        let mut stream = TcpStream::connect(addr).await.unwrap();
        let request =
            format!("GET {path} HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n");
        stream.write_all(request.as_bytes()).await.unwrap();
        let mut response = Vec::new();
        let _ = stream.read_to_end(&mut response).await;
        response
    }

    #[test]
    #[should_panic(expected = "1 scripted HTTP exchange(s) were not consumed")]
    fn finish_panics_when_exact_script_has_unconsumed_exchanges() {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        runtime.block_on(async {
            let server = LoopbackServer::exact_script(vec![ScriptedExchange::expect_path(
                "/unused",
                ResponseSpec::status(200),
            )])
            .await;
            let _ = server.finish().await;
        });
    }

    #[tokio::test]
    async fn close_connection_drops_stream_without_sending_headers() {
        let server = LoopbackServer::exact_script(vec![ScriptedExchange::expect_path(
            "/drop",
            ResponseSpec::close_connection(),
        )])
        .await;
        let response = send_raw_request(server.addr(), "/drop").await;
        assert!(response.is_empty());
        let requests = server.finish().await;
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].method, "GET");
        assert_eq!(requests[0].path, "/drop");
    }

    #[test]
    #[should_panic(expected = "handler panicked on GET /boom")]
    fn finish_surfaces_handler_panic() {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        runtime.block_on(async {
            let server = LoopbackServer::with_handler(|_| -> ResponseSpec { panic!("boom") }).await;
            let _ = send_raw_request(server.addr(), "/boom").await;
            let _ = server.finish().await;
        });
    }

    #[test]
    #[should_panic(expected = "unexpected request path: expected `/expected`, got `/wrong`")]
    fn finish_surfaces_unexpected_request_path() {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        runtime.block_on(async {
            let server = LoopbackServer::exact_script(vec![ScriptedExchange::expect_path(
                "/expected",
                ResponseSpec::status(200),
            )])
            .await;
            let _ = send_raw_request(server.addr(), "/wrong").await;
            let _ = server.finish().await;
        });
    }
}
