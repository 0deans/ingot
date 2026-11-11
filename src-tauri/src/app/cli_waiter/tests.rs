use super::endpoint::{cleanup_endpoint_file_at, write_endpoint_file};
use super::protocol::{
    collect_steam_env, read_bounded_line, send_message, validate_launch_request,
    RequestValidationError, WaiterEndpointInfo, WaiterLaunchRequest, WaiterLaunchResponse,
    MAX_MESSAGE_BYTES,
};
use super::transport::{
    delegate_launch_over_io, delegate_launch_with_file, handle_waiter_io, serve_waiter_listener,
    RESERVATION_ACK_TIMEOUT,
};
use super::{ReservedLaunch, WaiterSessionHost};
use std::io;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::io::{AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};

struct FakeHost {
    reservation_id: u64,
    launch_error: Option<String>,
    prep_error: Option<String>,
    start_calls: AtomicUsize,
    started_env: Mutex<Vec<(String, String)>>,
    waited_target: Mutex<Option<(String, u64)>>,
    wait_entered_tx: Mutex<Option<tokio::sync::oneshot::Sender<()>>>,
    finish_notify: Arc<tokio::sync::Notify>,
    cancelled_tx: Mutex<Option<tokio::sync::oneshot::Sender<(String, u64)>>>,
}

impl FakeHost {
    fn new(reservation_id: u64) -> Self {
        Self {
            reservation_id,
            launch_error: None,
            prep_error: None,
            start_calls: AtomicUsize::new(0),
            started_env: Mutex::new(Vec::new()),
            waited_target: Mutex::new(None),
            wait_entered_tx: Mutex::new(None),
            finish_notify: Arc::new(tokio::sync::Notify::new()),
            cancelled_tx: Mutex::new(None),
        }
    }
}

impl WaiterSessionHost for FakeHost {
    async fn start_launch(
        &self,
        _instance_id: String,
        steam_env: Vec<(String, String)>,
    ) -> Result<ReservedLaunch, String> {
        self.start_calls.fetch_add(1, Ordering::SeqCst);
        *self.started_env.lock().unwrap() = steam_env;
        if let Some(err) = &self.launch_error {
            return Err(err.clone());
        }
        let (tx, rx) = tokio::sync::oneshot::channel();
        if let Some(prep_err) = &self.prep_error {
            let _ = tx.send(Err(prep_err.clone()));
        } else {
            let _ = tx.send(Ok(1234));
        }
        Ok(ReservedLaunch {
            reservation_id: self.reservation_id,
            launch_result: rx,
        })
    }

    async fn wait_for_reservation_end(&self, instance_id: String, reservation_id: u64) {
        *self.waited_target.lock().unwrap() = Some((instance_id, reservation_id));
        if let Some(tx) = self.wait_entered_tx.lock().unwrap().take() {
            let _ = tx.send(());
        }
        self.finish_notify.notified().await;
    }

    async fn cancel_or_kill_reservation(&self, instance_id: String, reservation_id: u64) {
        if let Some(tx) = self.cancelled_tx.lock().unwrap().take() {
            let _ = tx.send((instance_id, reservation_id));
        }
    }
}

#[test]
fn steam_env_filter_retains_only_allowlisted_safe_entries() {
    let filtered = collect_steam_env(vec![
        ("SteamGameId".to_string(), "12345".to_string()),
        ("SteamOverlayGameId".to_string(), "12345".to_string()),
        ("LD_PRELOAD".to_string(), "/tmp/evil.so".to_string()),
        ("PATH".to_string(), "/usr/bin".to_string()),
        ("SteamAppId".to_string(), "".to_string()),
        ("SteamClientLaunch".to_string(), "a".repeat(1025)),
        ("SteamEnv".to_string(), "bad\0value".to_string()),
    ]);
    assert_eq!(
        filtered,
        vec![
            ("SteamGameId".to_string(), "12345".to_string()),
            ("SteamOverlayGameId".to_string(), "12345".to_string()),
        ]
    );
}

#[test]
fn validate_launch_request_accepts_valid_and_rejects_malformed_or_invalid_token() {
    assert_eq!(
        validate_launch_request("{not-json", "expected").err(),
        Some(RequestValidationError::Malformed)
    );
    assert_eq!(
        RequestValidationError::Malformed.into_response(),
        WaiterLaunchResponse::Failed {
            message: "Malformed waiter launch request".to_string(),
        }
    );

    let wrong_token = serde_json::to_string(&WaiterLaunchRequest {
        token: "wrong".to_string(),
        instance_id: "inst-1".to_string(),
        steam_env: Vec::new(),
    })
    .unwrap();
    assert_eq!(
        validate_launch_request(&wrong_token, "expected").err(),
        Some(RequestValidationError::InvalidToken)
    );
    assert_eq!(
        RequestValidationError::InvalidToken.into_response(),
        WaiterLaunchResponse::Failed {
            message: "Invalid waiter authentication token".to_string(),
        }
    );

    let empty_token = serde_json::to_string(&WaiterLaunchRequest {
        token: String::new(),
        instance_id: "inst-1".to_string(),
        steam_env: Vec::new(),
    })
    .unwrap();
    assert_eq!(
        validate_launch_request(&empty_token, "").err(),
        Some(RequestValidationError::InvalidToken)
    );

    let valid = serde_json::to_string(&WaiterLaunchRequest {
        token: "expected".to_string(),
        instance_id: "inst-1".to_string(),
        steam_env: vec![
            ("SteamAppId".to_string(), "77".to_string()),
            ("LD_PRELOAD".to_string(), "/bad".to_string()),
        ],
    })
    .unwrap();
    let validated = validate_launch_request(&valid, "expected").unwrap();
    assert_eq!(validated.instance_id, "inst-1");
    assert_eq!(
        validated.steam_env,
        vec![("SteamAppId".to_string(), "77".to_string())]
    );
}

#[tokio::test]
async fn read_bounded_line_enforces_16kib_limit_and_valid_utf8() {
    let mut empty_reader = BufReader::new(std::io::Cursor::new(Vec::<u8>::new()));
    assert_eq!(read_bounded_line(&mut empty_reader).await.unwrap(), None);

    let mut exact_payload = vec![b'a'; MAX_MESSAGE_BYTES - 1];
    exact_payload.push(b'\n');
    let mut exact_reader = BufReader::new(std::io::Cursor::new(exact_payload));
    let line = read_bounded_line(&mut exact_reader).await.unwrap().unwrap();
    assert_eq!(line.len(), MAX_MESSAGE_BYTES);

    let mut oversized_newline = vec![b'a'; MAX_MESSAGE_BYTES];
    oversized_newline.push(b'\n');
    let mut oversized_reader = BufReader::new(std::io::Cursor::new(oversized_newline));
    let err = read_bounded_line(&mut oversized_reader).await.unwrap_err();
    assert_eq!(err.kind(), io::ErrorKind::InvalidData);

    let oversized_unterminated = vec![b'b'; MAX_MESSAGE_BYTES + 1];
    let mut unterminated_reader = BufReader::new(std::io::Cursor::new(oversized_unterminated));
    let err = read_bounded_line(&mut unterminated_reader)
        .await
        .unwrap_err();
    assert_eq!(err.kind(), io::ErrorKind::InvalidData);

    let invalid_utf8 = vec![0xff, 0xfe, b'\n'];
    let mut utf8_reader = BufReader::new(std::io::Cursor::new(invalid_utf8));
    let err = read_bounded_line(&mut utf8_reader).await.unwrap_err();
    assert_eq!(err.kind(), io::ErrorKind::InvalidData);
}

#[test]
fn cleanup_endpoint_file_removes_own_or_corrupt_and_preserves_other_pid() {
    let temp = tempfile::tempdir().unwrap();
    let missing = temp.path().join("missing.json");
    cleanup_endpoint_file_at(&missing, 100);

    let path = temp.path().join("cli-waiter.json");
    write_endpoint_file(
        &path,
        &WaiterEndpointInfo {
            port: 4000,
            token: "tok".to_string(),
            pid: 200,
        },
    )
    .unwrap();

    cleanup_endpoint_file_at(&path, 100);
    assert!(path.exists(), "other PID endpoint file must be preserved");

    cleanup_endpoint_file_at(&path, 200);
    assert!(!path.exists(), "own PID endpoint file must be removed");

    std::fs::write(&path, b"{corrupt").unwrap();
    cleanup_endpoint_file_at(&path, 100);
    assert!(!path.exists(), "corrupt endpoint file must be removed");
}

#[cfg(unix)]
#[test]
fn write_endpoint_file_sets_0600_mode() {
    use std::os::unix::fs::PermissionsExt;

    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("cli-waiter.json");
    write_endpoint_file(
        &path,
        &WaiterEndpointInfo {
            port: 4001,
            token: "secret".to_string(),
            pid: 42,
        },
    )
    .unwrap();
    let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o600);
}

#[tokio::test]
async fn normal_waiter_lifecycle_waits_for_game_exit_and_returns_zero() {
    let temp = tempfile::tempdir().unwrap();
    let endpoint_file = temp.path().join("cli-waiter.json");
    let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let token = "valid-token".to_string();

    write_endpoint_file(
        &endpoint_file,
        &WaiterEndpointInfo {
            port,
            token: token.clone(),
            pid: std::process::id().wrapping_add(1),
        },
    )
    .unwrap();

    let (wait_entered_tx, wait_entered_rx) = tokio::sync::oneshot::channel();
    let mut host = FakeHost::new(7);
    host.wait_entered_tx = Mutex::new(Some(wait_entered_tx));
    let host = Arc::new(host);
    let finish_notify = Arc::clone(&host.finish_notify);

    let host_clone = Arc::clone(&host);
    let server_task = tokio::spawn(async move {
        serve_waiter_listener(listener, token, host_clone).await;
    });

    let path_clone = endpoint_file.clone();
    let client_task = tokio::spawn(async move {
        delegate_launch_with_file(
            &path_clone,
            "inst-1",
            vec![("SteamGameId".to_string(), "99".to_string())],
        )
        .await
    });

    wait_entered_rx.await.unwrap();
    tokio::task::yield_now().await;
    assert!(!client_task.is_finished());

    finish_notify.notify_one();
    let exit_code = client_task.await.unwrap();
    assert_eq!(exit_code, Some(0));
    assert_eq!(
        *host.started_env.lock().unwrap(),
        vec![("SteamGameId".to_string(), "99".to_string())]
    );
    assert_eq!(
        *host.waited_target.lock().unwrap(),
        Some(("inst-1".to_string(), 7))
    );
    server_task.abort();
}

#[tokio::test]
async fn waiter_disconnect_triggers_reservation_cancellation_or_kill() {
    let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
    let addr = listener.local_addr().unwrap();
    let token = "stop-token".to_string();
    let (cancel_tx, cancel_rx) = tokio::sync::oneshot::channel();

    let mut host = FakeHost::new(42);
    host.cancelled_tx = Mutex::new(Some(cancel_tx));
    let host = Arc::new(host);

    let server_task = tokio::spawn(async move {
        serve_waiter_listener(listener, token, host).await;
    });

    let stream = TcpStream::connect(addr).await.unwrap();
    let (read_half, mut write_half) = stream.into_split();
    let mut reader = BufReader::new(read_half);

    let req = WaiterLaunchRequest {
        token: "stop-token".to_string(),
        instance_id: "inst-steam-stop".to_string(),
        steam_env: Vec::new(),
    };
    send_message(&mut write_half, &req).await.unwrap();

    let ack_line = read_bounded_line(&mut reader).await.unwrap().unwrap();
    let ack: WaiterLaunchResponse = serde_json::from_str(&ack_line).unwrap();
    assert_eq!(ack, WaiterLaunchResponse::Accepted { reservation_id: 42 });

    // Simulate Steam clicking "Stop" and terminating the waiting CLI process.
    drop(reader);
    drop(write_half);

    let cancelled = cancel_rx.await.unwrap();
    assert_eq!(cancelled, ("inst-steam-stop".to_string(), 42));
    server_task.abort();
}

#[tokio::test]
async fn failed_accept_write_and_aborted_session_cancel_active_reservation() {
    let (mut client_stream, server_stream) = tokio::io::duplex(4096);
    let (server_read, server_write) = tokio::io::split(server_stream);
    let (cancel_tx, cancel_rx) = tokio::sync::oneshot::channel();
    let mut host = FakeHost::new(77);
    host.cancelled_tx = Mutex::new(Some(cancel_tx));
    let host = Arc::new(host);

    let req = WaiterLaunchRequest {
        token: "tok".to_string(),
        instance_id: "inst-drop-before-ack".to_string(),
        steam_env: Vec::new(),
    };
    send_message(&mut client_stream, &req).await.unwrap();
    drop(client_stream);

    let err = handle_waiter_io(server_read, server_write, "tok", host)
        .await
        .unwrap_err();
    assert_eq!(err.kind(), io::ErrorKind::BrokenPipe);
    assert_eq!(
        cancel_rx.await.unwrap(),
        ("inst-drop-before-ack".to_string(), 77)
    );

    let (client_stream, server_stream) = tokio::io::duplex(4096);
    let (client_read, mut client_write) = tokio::io::split(client_stream);
    let (server_read, server_write) = tokio::io::split(server_stream);
    let mut client_reader = BufReader::new(client_read);
    let (wait_entered_tx, wait_entered_rx) = tokio::sync::oneshot::channel();
    let (abort_cancel_tx, abort_cancel_rx) = tokio::sync::oneshot::channel();
    let mut abort_host = FakeHost::new(88);
    abort_host.wait_entered_tx = Mutex::new(Some(wait_entered_tx));
    abort_host.cancelled_tx = Mutex::new(Some(abort_cancel_tx));
    let abort_host = Arc::new(abort_host);

    let server_task = tokio::spawn(async move {
        handle_waiter_io(server_read, server_write, "tok", abort_host).await
    });

    send_message(
        &mut client_write,
        &WaiterLaunchRequest {
            token: "tok".to_string(),
            instance_id: "inst-abort".to_string(),
            steam_env: Vec::new(),
        },
    )
    .await
    .unwrap();
    let ack_line = read_bounded_line(&mut client_reader)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        serde_json::from_str::<WaiterLaunchResponse>(&ack_line).unwrap(),
        WaiterLaunchResponse::Accepted { reservation_id: 88 }
    );
    wait_entered_rx.await.unwrap();

    server_task.abort();
    assert!(server_task.await.unwrap_err().is_cancelled());
    assert_eq!(
        abort_cancel_rx.await.unwrap(),
        ("inst-abort".to_string(), 88)
    );
    drop(client_write);
    drop(client_reader);
}

#[tokio::test]
async fn launch_failures_return_nonzero_and_stale_or_corrupt_files_return_none() {
    let temp = tempfile::tempdir().unwrap();
    let missing_file = temp.path().join("missing.json");
    assert_eq!(
        delegate_launch_with_file(&missing_file, "inst-1", Vec::new()).await,
        None
    );

    let corrupt_file = temp.path().join("corrupt.json");
    std::fs::write(&corrupt_file, b"{corrupt").unwrap();
    assert_eq!(
        delegate_launch_with_file(&corrupt_file, "inst-1", Vec::new()).await,
        None
    );
    assert!(corrupt_file.exists());

    let unused_listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
    let unused_port = unused_listener.local_addr().unwrap().port();
    drop(unused_listener);

    let stale_file = temp.path().join("stale.json");
    write_endpoint_file(
        &stale_file,
        &WaiterEndpointInfo {
            port: unused_port,
            token: "stale".to_string(),
            pid: std::process::id().wrapping_add(1),
        },
    )
    .unwrap();
    assert_eq!(
        delegate_launch_with_file(&stale_file, "inst-1", Vec::new()).await,
        None
    );
    assert!(stale_file.exists());

    let active_file = temp.path().join("active.json");
    let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
    let port = listener.local_addr().unwrap().port();
    write_endpoint_file(
        &active_file,
        &WaiterEndpointInfo {
            port,
            token: "token".to_string(),
            pid: std::process::id().wrapping_add(1),
        },
    )
    .unwrap();

    let mut pre_fail_host = FakeHost::new(1);
    pre_fail_host.launch_error = Some("Instance is already running".to_string());
    let server_task = tokio::spawn(serve_waiter_listener(
        listener,
        "token".to_string(),
        Arc::new(pre_fail_host),
    ));

    let code = delegate_launch_with_file(&active_file, "inst-1", Vec::new()).await;
    assert_eq!(code, Some(1));
    server_task.abort();

    let post_listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
    let post_port = post_listener.local_addr().unwrap().port();
    let post_file = temp.path().join("post-fail.json");
    write_endpoint_file(
        &post_file,
        &WaiterEndpointInfo {
            port: post_port,
            token: "token".to_string(),
            pid: std::process::id().wrapping_add(1),
        },
    )
    .unwrap();

    let (post_wait_tx, post_wait_rx) = tokio::sync::oneshot::channel();
    let mut post_fail_host = FakeHost::new(2);
    post_fail_host.prep_error = Some("Missing runtime asset".to_string());
    post_fail_host.wait_entered_tx = Mutex::new(Some(post_wait_tx));
    let post_fail_host = Arc::new(post_fail_host);
    let finish_notify = Arc::clone(&post_fail_host.finish_notify);
    let post_server = tokio::spawn(serve_waiter_listener(
        post_listener,
        "token".to_string(),
        post_fail_host,
    ));

    let post_client =
        tokio::spawn(
            async move { delegate_launch_with_file(&post_file, "inst-2", Vec::new()).await },
        );
    post_wait_rx.await.unwrap();
    tokio::task::yield_now().await;
    assert!(!post_client.is_finished());

    finish_notify.notify_one();
    assert_eq!(post_client.await.unwrap(), Some(1));
    post_server.abort();
}

#[tokio::test]
async fn same_pid_endpoint_file_skips_delegation_and_is_preserved() {
    let temp = tempfile::tempdir().unwrap();
    let endpoint_file = temp.path().join("self.json");
    let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
    let port = listener.local_addr().unwrap().port();

    write_endpoint_file(
        &endpoint_file,
        &WaiterEndpointInfo {
            port,
            token: "self-token".to_string(),
            pid: std::process::id(),
        },
    )
    .unwrap();
    let before = std::fs::read(&endpoint_file).unwrap();

    let host = Arc::new(FakeHost::new(11));
    let server_task = tokio::spawn(serve_waiter_listener(
        listener,
        "self-token".to_string(),
        Arc::clone(&host),
    ));

    let result = delegate_launch_with_file(&endpoint_file, "inst-self", Vec::new()).await;
    assert_eq!(result, None);
    assert_eq!(host.start_calls.load(Ordering::SeqCst), 0);
    assert_eq!(std::fs::read(&endpoint_file).unwrap(), before);
    server_task.abort();
}

#[tokio::test]
async fn server_rejects_malformed_invalid_token_and_oversized_requests() {
    let (client_stream, server_stream) = tokio::io::duplex(4096);
    let (client_read, mut client_write) = tokio::io::split(client_stream);
    let (server_read, server_write) = tokio::io::split(server_stream);
    let mut client_reader = BufReader::new(client_read);
    let host = Arc::new(FakeHost::new(1));

    let server_task = tokio::spawn(handle_waiter_io(
        server_read,
        server_write,
        "expected-token",
        Arc::clone(&host),
    ));
    client_write.write_all(b"{not-valid-json\n").await.unwrap();
    client_write.flush().await.unwrap();
    let line = read_bounded_line(&mut client_reader)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        serde_json::from_str::<WaiterLaunchResponse>(&line).unwrap(),
        WaiterLaunchResponse::Failed {
            message: "Malformed waiter launch request".to_string(),
        }
    );
    server_task.await.unwrap().unwrap();
    assert_eq!(host.start_calls.load(Ordering::SeqCst), 0);

    let (client_stream, server_stream) = tokio::io::duplex(4096);
    let (client_read, mut client_write) = tokio::io::split(client_stream);
    let (server_read, server_write) = tokio::io::split(server_stream);
    let mut client_reader = BufReader::new(client_read);

    let server_task = tokio::spawn(handle_waiter_io(
        server_read,
        server_write,
        "expected-token",
        Arc::clone(&host),
    ));
    send_message(
        &mut client_write,
        &WaiterLaunchRequest {
            token: "wrong-token".to_string(),
            instance_id: "inst-1".to_string(),
            steam_env: Vec::new(),
        },
    )
    .await
    .unwrap();
    let line = read_bounded_line(&mut client_reader)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        serde_json::from_str::<WaiterLaunchResponse>(&line).unwrap(),
        WaiterLaunchResponse::Failed {
            message: "Invalid waiter authentication token".to_string(),
        }
    );
    server_task.await.unwrap().unwrap();
    assert_eq!(host.start_calls.load(Ordering::SeqCst), 0);

    let (client_stream, server_stream) = tokio::io::duplex(MAX_MESSAGE_BYTES * 2);
    let (_client_read, mut client_write) = tokio::io::split(client_stream);
    let (server_read, server_write) = tokio::io::split(server_stream);

    let mut oversized = vec![b'x'; MAX_MESSAGE_BYTES + 64];
    oversized.push(b'\n');
    client_write.write_all(&oversized).await.unwrap();
    client_write.flush().await.unwrap();

    let err = handle_waiter_io(
        server_read,
        server_write,
        "expected-token",
        Arc::clone(&host),
    )
    .await
    .unwrap_err();
    assert_eq!(err.kind(), io::ErrorKind::InvalidData);
    assert_eq!(host.start_calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn client_rejects_malformed_and_oversized_ack_responses() {
    let (client_stream, server_stream) = tokio::io::duplex(4096);
    let (client_read, client_write) = tokio::io::split(client_stream);
    let (server_read, mut server_write) = tokio::io::split(server_stream);
    let mut server_reader = BufReader::new(server_read);

    let client_task = tokio::spawn(delegate_launch_over_io(
        client_read,
        client_write,
        "tok".to_string(),
        "inst-malformed-ack",
        Vec::new(),
    ));
    let _ = read_bounded_line(&mut server_reader)
        .await
        .unwrap()
        .unwrap();
    server_write.write_all(b"{bad-response\n").await.unwrap();
    server_write.flush().await.unwrap();
    assert_eq!(client_task.await.unwrap(), None);

    let (client_stream, server_stream) = tokio::io::duplex(MAX_MESSAGE_BYTES * 2);
    let (client_read, client_write) = tokio::io::split(client_stream);
    let (server_read, mut server_write) = tokio::io::split(server_stream);
    let mut server_reader = BufReader::new(server_read);

    let client_task = tokio::spawn(delegate_launch_over_io(
        client_read,
        client_write,
        "tok".to_string(),
        "inst-oversized-ack",
        Vec::new(),
    ));
    let _ = read_bounded_line(&mut server_reader)
        .await
        .unwrap()
        .unwrap();
    let mut oversized = vec![b'y'; MAX_MESSAGE_BYTES + 64];
    oversized.push(b'\n');
    server_write.write_all(&oversized).await.unwrap();
    server_write.flush().await.unwrap();
    assert_eq!(client_task.await.unwrap(), None);

    let (client_stream, server_stream) = tokio::io::duplex(4096);
    let (client_read, client_write) = tokio::io::split(client_stream);
    let (server_read, mut server_write) = tokio::io::split(server_stream);
    let mut server_reader = BufReader::new(server_read);

    let client_task = tokio::spawn(delegate_launch_over_io(
        client_read,
        client_write,
        "tok".to_string(),
        "inst-immediate-finish",
        Vec::new(),
    ));
    let _ = read_bounded_line(&mut server_reader)
        .await
        .unwrap()
        .unwrap();
    send_message(
        &mut server_write,
        &WaiterLaunchResponse::Finished { exit_code: 3 },
    )
    .await
    .unwrap();
    assert_eq!(client_task.await.unwrap(), Some(3));
}

#[tokio::test(start_paused = true)]
async fn client_returns_none_after_reservation_ack_timeout() {
    let (client_stream, server_stream) = tokio::io::duplex(4096);
    let (client_read, client_write) = tokio::io::split(client_stream);
    let (server_read, _server_write) = tokio::io::split(server_stream);
    let mut server_reader = BufReader::new(server_read);

    let client_task = tokio::spawn(delegate_launch_over_io(
        client_read,
        client_write,
        "token".to_string(),
        "inst-timeout",
        Vec::new(),
    ));

    let request_line = read_bounded_line(&mut server_reader)
        .await
        .unwrap()
        .unwrap();
    let request: WaiterLaunchRequest = serde_json::from_str(&request_line).unwrap();
    assert_eq!(request.instance_id, "inst-timeout");

    tokio::time::advance(RESERVATION_ACK_TIMEOUT - Duration::from_millis(1)).await;
    tokio::task::yield_now().await;
    assert!(!client_task.is_finished());

    tokio::time::advance(Duration::from_millis(2)).await;
    assert_eq!(client_task.await.unwrap(), None);
}
