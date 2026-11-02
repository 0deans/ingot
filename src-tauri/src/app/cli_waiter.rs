//! Local CLI waiter IPC so Steam shortcuts stay alive while a game runs on an already-open
//! Ingot instance, and forward Steam's "Stop" termination to that specific instance.

use serde::{Deserialize, Serialize};
use std::future::Future;
use std::io::{self, Write};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Manager, Runtime};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};

const MAX_MESSAGE_BYTES: usize = 16 * 1024;
const CONNECT_TIMEOUT: Duration = Duration::from_millis(500);
const RESERVATION_ACK_TIMEOUT: Duration = Duration::from_secs(15);

const ALLOWED_STEAM_ENV_KEYS: &[&str] = &[
    "SteamAppId",
    "SteamGameId",
    "SteamOverlayGameId",
    "SteamClientLaunch",
    "SteamEnv",
    "SteamVirtualGamepadInfo",
    "STEAM_GAME_ID",
    "STEAM_APP_ID",
];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct WaiterEndpointInfo {
    pub(crate) port: u16,
    pub(crate) token: String,
    pub(crate) pid: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct WaiterLaunchRequest {
    pub(crate) token: String,
    pub(crate) instance_id: String,
    pub(crate) steam_env: Vec<(String, String)>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "status", rename_all = "snake_case")]
pub(crate) enum WaiterLaunchResponse {
    Accepted { reservation_id: u64 },
    Finished { exit_code: i32 },
    Failed { message: String },
}

pub(crate) struct ReservedLaunch {
    pub(crate) reservation_id: u64,
    pub(crate) launch_result: tokio::sync::oneshot::Receiver<Result<u32, String>>,
}

pub(crate) trait WaiterSessionHost: Send + Sync + 'static {
    fn start_launch(
        &self,
        instance_id: String,
        steam_env: Vec<(String, String)>,
    ) -> impl Future<Output = Result<ReservedLaunch, String>> + Send;

    fn wait_for_reservation_end(
        &self,
        instance_id: String,
        reservation_id: u64,
    ) -> impl Future<Output = ()> + Send;

    fn cancel_or_kill_reservation(
        &self,
        instance_id: String,
        reservation_id: u64,
    ) -> impl Future<Output = ()> + Send;
}

/// Filters environment variables down to safe, non-injectable Steam game/overlay identifiers.
pub(crate) fn collect_steam_env<I>(vars: I) -> Vec<(String, String)>
where
    I: IntoIterator<Item = (String, String)>,
{
    vars.into_iter()
        .filter(|(key, value)| {
            ALLOWED_STEAM_ENV_KEYS.contains(&key.as_str())
                && !value.is_empty()
                && value.len() <= 1024
                && !value.contains('\0')
        })
        .collect()
}

pub(crate) fn write_endpoint_file(path: &Path, info: &WaiterEndpointInfo) -> io::Result<()> {
    let Some(parent) = path.parent() else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "endpoint file path has no parent directory",
        ));
    };
    std::fs::create_dir_all(parent)?;
    let encoded = serde_json::to_vec(info).map_err(io::Error::other)?;
    let mut staged = tempfile::NamedTempFile::new_in(parent)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = staged
            .as_file()
            .set_permissions(std::fs::Permissions::from_mode(0o600));
    }
    staged.write_all(&encoded)?;
    staged.flush()?;
    staged.persist(path).map_err(|err| err.error)?;
    Ok(())
}

pub(crate) fn cleanup_endpoint_file_at(path: &Path, expected_pid: u32) {
    let Ok(bytes) = std::fs::read(path) else {
        return;
    };
    let should_remove = match serde_json::from_slice::<WaiterEndpointInfo>(&bytes) {
        Ok(info) => info.pid == expected_pid,
        Err(_) => true,
    };
    if should_remove {
        let _ = std::fs::remove_file(path);
    }
}

pub(crate) fn cleanup_endpoint_file<R: Runtime>(app: &AppHandle<R>) {
    if let Ok(data_dir) = app.path().app_data_dir() {
        let path = super::AppPaths::new(data_dir).cli_waiter_file();
        cleanup_endpoint_file_at(&path, std::process::id());
    }
}

struct NativeWaiterHost<R: Runtime> {
    app: AppHandle<R>,
}

impl<R: Runtime> WaiterSessionHost for NativeWaiterHost<R> {
    async fn start_launch(
        &self,
        instance_id: String,
        steam_env: Vec<(String, String)>,
    ) -> Result<ReservedLaunch, String> {
        let (reserved_tx, reserved_rx) = tokio::sync::oneshot::channel();
        let (result_tx, result_rx) = tokio::sync::oneshot::channel();
        let handle = self.app.clone();
        let id_for_launch = instance_id.clone();

        tokio::spawn(async move {
            let outcome = crate::app::games::launch_instance_with_options(
                handle.clone(),
                id_for_launch.clone(),
                crate::app::games::LaunchSessionOptions {
                    quick_play: None,
                    launched_from_cli: true,
                    extra_env: steam_env,
                    on_reserved: Some(reserved_tx),
                },
            )
            .await;

            match outcome {
                Ok(pid) => {
                    let _ = result_tx.send(Ok(pid));
                }
                Err(err) => {
                    eprintln!("[CLI] Failed to launch instance {id_for_launch}: {err}");
                    super::cli::restore_window_on_cli_failure(&handle, &err);
                    let _ = result_tx.send(Err(err.to_string()));
                }
            }
        });

        match reserved_rx.await {
            Ok(reservation_id) => Ok(ReservedLaunch {
                reservation_id,
                launch_result: result_rx,
            }),
            Err(_) => match result_rx.await {
                Ok(Err(message)) => Err(message),
                _ => Err("Instance launch ended before reservation completed".to_string()),
            },
        }
    }

    async fn wait_for_reservation_end(&self, instance_id: String, reservation_id: u64) {
        let pm = super::state(&self.app).games.clone();
        pm.wait_for_reservation_end(&instance_id, reservation_id)
            .await;
    }

    async fn cancel_or_kill_reservation(&self, instance_id: String, reservation_id: u64) {
        let pm = super::state(&self.app).games.clone();
        if let Err(err) = pm
            .cancel_or_kill_reservation(&instance_id, reservation_id)
            .await
        {
            eprintln!("[CLI] Failed to stop instance {instance_id} after waiter disconnect: {err}");
        }
    }
}

pub(crate) fn start_server<R: Runtime>(app: &AppHandle<R>) {
    let Ok(data_dir) = app.path().app_data_dir() else {
        return;
    };
    let endpoint_path = super::AppPaths::new(data_dir).cli_waiter_file();
    let host = Arc::new(NativeWaiterHost { app: app.clone() });

    tauri::async_runtime::spawn(async move {
        let bind_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0);
        let listener = match TcpListener::bind(bind_addr).await {
            Ok(listener) => listener,
            Err(err) => {
                eprintln!("[CLI] Failed to bind local CLI waiter listener: {err}");
                return;
            }
        };
        let port = match listener.local_addr() {
            Ok(addr) => addr.port(),
            Err(err) => {
                eprintln!("[CLI] Failed to read local CLI waiter address: {err}");
                return;
            }
        };
        let token = uuid::Uuid::new_v4().to_string();
        let info = WaiterEndpointInfo {
            port,
            token: token.clone(),
            pid: std::process::id(),
        };
        if let Err(err) = write_endpoint_file(&endpoint_path, &info) {
            eprintln!("[CLI] Failed to publish CLI waiter endpoint file: {err}");
            return;
        }
        serve_waiter_listener(listener, token, host).await;
    });
}

pub(crate) async fn serve_waiter_listener<H: WaiterSessionHost>(
    listener: TcpListener,
    expected_token: String,
    host: Arc<H>,
) {
    loop {
        let Ok((stream, _)) = listener.accept().await else {
            break;
        };
        let token = expected_token.clone();
        let host = host.clone();
        tokio::spawn(async move {
            let _ = handle_waiter_connection(stream, &token, host.as_ref()).await;
        });
    }
}

async fn handle_waiter_connection<H: WaiterSessionHost>(
    stream: TcpStream,
    expected_token: &str,
    host: &H,
) -> io::Result<()> {
    let (read_half, mut write_half) = stream.into_split();
    let mut reader = BufReader::new(read_half);

    let Some(request_line) = read_bounded_line(&mut reader).await? else {
        return Ok(());
    };

    let request: WaiterLaunchRequest = match serde_json::from_str(&request_line) {
        Ok(req) => req,
        Err(_) => {
            send_response(
                &mut write_half,
                &WaiterLaunchResponse::Failed {
                    message: "Malformed waiter launch request".to_string(),
                },
            )
            .await?;
            return Ok(());
        }
    };

    if request.token != expected_token {
        send_response(
            &mut write_half,
            &WaiterLaunchResponse::Failed {
                message: "Invalid waiter authentication token".to_string(),
            },
        )
        .await?;
        return Ok(());
    }

    let instance_id = request.instance_id;
    let sanitized_env = collect_steam_env(request.steam_env);

    let reserved = match host.start_launch(instance_id.clone(), sanitized_env).await {
        Ok(reserved) => reserved,
        Err(message) => {
            send_response(&mut write_half, &WaiterLaunchResponse::Failed { message }).await?;
            return Ok(());
        }
    };

    let reservation_id = reserved.reservation_id;
    send_response(
        &mut write_half,
        &WaiterLaunchResponse::Accepted { reservation_id },
    )
    .await?;

    let reservation_done = host.wait_for_reservation_end(instance_id.clone(), reservation_id);
    let client_disconnected = wait_for_client_disconnect(&mut reader);

    tokio::select! {
        () = reservation_done => {
            let response = match reserved.launch_result.await {
                Ok(Ok(_pid)) => WaiterLaunchResponse::Finished { exit_code: 0 },
                Ok(Err(message)) => WaiterLaunchResponse::Failed { message },
                Err(_) => WaiterLaunchResponse::Finished { exit_code: 0 },
            };
            send_response(&mut write_half, &response).await?;
        }
        () = client_disconnected => {
            host.cancel_or_kill_reservation(instance_id, reservation_id).await;
        }
    }

    Ok(())
}

async fn wait_for_client_disconnect<R: tokio::io::AsyncRead + Unpin>(reader: &mut R) {
    let mut scratch = [0_u8; 64];
    loop {
        match reader.read(&mut scratch).await {
            Ok(0) | Err(_) => return,
            Ok(_) => {}
        }
    }
}

async fn read_bounded_line<R: tokio::io::AsyncBufRead + Unpin>(
    reader: &mut R,
) -> io::Result<Option<String>> {
    let mut buf = Vec::new();
    let bytes_read = reader
        .take((MAX_MESSAGE_BYTES + 1) as u64)
        .read_until(b'\n', &mut buf)
        .await?;
    if bytes_read == 0 {
        return Ok(None);
    }
    if buf.len() > MAX_MESSAGE_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "waiter message exceeded maximum length",
        ));
    }
    let line =
        String::from_utf8(buf).map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
    Ok(Some(line))
}

async fn send_response<W: tokio::io::AsyncWrite + Unpin>(
    writer: &mut W,
    response: &WaiterLaunchResponse,
) -> io::Result<()> {
    let mut encoded = serde_json::to_vec(response).map_err(io::Error::other)?;
    encoded.push(b'\n');
    writer.write_all(&encoded).await?;
    writer.flush().await
}

/// Checks if `--launch-instance <id>` was requested and an existing Ingot instance is listening.
/// When delegated, blocks until the game finishes and returns `Some(exit_code)`.
/// Returns `None` if not a CLI launch or if no running Ingot instance is reachable.
pub(crate) async fn try_delegate_to_running_instance(args: &[String]) -> Option<i32> {
    let instance_id = super::cli::parse_cli_launch_instance(args)?;
    let endpoint_path = super::paths::default_cli_waiter_file()?;
    let steam_env = collect_steam_env(std::env::vars());
    delegate_launch_with_file(&endpoint_path, &instance_id, steam_env).await
}

pub(crate) async fn delegate_launch_with_file(
    endpoint_path: &Path,
    instance_id: &str,
    steam_env: Vec<(String, String)>,
) -> Option<i32> {
    let bytes = std::fs::read(endpoint_path).ok()?;
    let info: WaiterEndpointInfo = serde_json::from_slice(&bytes).ok()?;
    if info.pid == std::process::id() {
        return None;
    }

    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), info.port);
    let stream = match tokio::time::timeout(CONNECT_TIMEOUT, TcpStream::connect(addr)).await {
        Ok(Ok(stream)) => stream,
        _ => return None,
    };

    let (read_half, mut write_half) = stream.into_split();
    let mut reader = BufReader::new(read_half);

    let request = WaiterLaunchRequest {
        token: info.token,
        instance_id: instance_id.to_string(),
        steam_env,
    };
    let mut encoded = serde_json::to_vec(&request).ok()?;
    encoded.push(b'\n');
    if write_half.write_all(&encoded).await.is_err() || write_half.flush().await.is_err() {
        return None;
    }

    let first_line =
        match tokio::time::timeout(RESERVATION_ACK_TIMEOUT, read_bounded_line(&mut reader)).await {
            Ok(Ok(Some(line))) => line,
            _ => return None,
        };

    let first_response: WaiterLaunchResponse = serde_json::from_str(&first_line).ok()?;
    match first_response {
        WaiterLaunchResponse::Accepted { .. } => {}
        WaiterLaunchResponse::Finished { exit_code } => return Some(exit_code),
        WaiterLaunchResponse::Failed { message } => {
            eprintln!("[CLI] {message}");
            return Some(1);
        }
    }

    match read_bounded_line(&mut reader).await {
        Ok(Some(second_line)) => match serde_json::from_str::<WaiterLaunchResponse>(&second_line) {
            Ok(WaiterLaunchResponse::Finished { exit_code }) => Some(exit_code),
            Ok(WaiterLaunchResponse::Failed { message }) => {
                eprintln!("[CLI] {message}");
                Some(1)
            }
            _ => Some(0),
        },
        Ok(None) | Err(_) => Some(0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    struct FakeHost {
        reservation_id: u64,
        launch_error: Option<String>,
        prep_error: Option<String>,
        started_env: Mutex<Vec<(String, String)>>,
        finish_notify: Arc<tokio::sync::Notify>,
        cancelled_tx: Mutex<Option<tokio::sync::oneshot::Sender<(String, u64)>>>,
    }

    impl WaiterSessionHost for FakeHost {
        async fn start_launch(
            &self,
            _instance_id: String,
            steam_env: Vec<(String, String)>,
        ) -> Result<ReservedLaunch, String> {
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

        async fn wait_for_reservation_end(&self, _instance_id: String, _reservation_id: u64) {
            self.finish_notify.notified().await;
        }

        async fn cancel_or_kill_reservation(&self, instance_id: String, reservation_id: u64) {
            if let Some(tx) = self.cancelled_tx.lock().unwrap().take() {
                let _ = tx.send((instance_id, reservation_id));
            }
        }
    }

    #[test]
    fn steam_env_filter_retains_only_allowlisted_keys() {
        let filtered = collect_steam_env(vec![
            ("SteamGameId".to_string(), "12345".to_string()),
            ("SteamOverlayGameId".to_string(), "12345".to_string()),
            ("LD_PRELOAD".to_string(), "/tmp/evil.so".to_string()),
            ("PATH".to_string(), "/usr/bin".to_string()),
            ("SteamAppId".to_string(), "".to_string()),
        ]);
        assert_eq!(
            filtered,
            vec![
                ("SteamGameId".to_string(), "12345".to_string()),
                ("SteamOverlayGameId".to_string(), "12345".to_string()),
            ]
        );
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
                pid: 999_999,
            },
        )
        .unwrap();

        let finish_notify = Arc::new(tokio::sync::Notify::new());
        let host = Arc::new(FakeHost {
            reservation_id: 7,
            launch_error: None,
            prep_error: None,
            started_env: Mutex::new(Vec::new()),
            finish_notify: finish_notify.clone(),
            cancelled_tx: Mutex::new(None),
        });

        let host_clone = host.clone();
        tokio::spawn(async move {
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

        tokio::time::sleep(Duration::from_millis(30)).await;
        assert!(!client_task.is_finished());

        finish_notify.notify_one();
        let exit_code = client_task.await.unwrap();
        assert_eq!(exit_code, Some(0));
        assert_eq!(
            *host.started_env.lock().unwrap(),
            vec![("SteamGameId".to_string(), "99".to_string())]
        );
    }

    #[tokio::test]
    async fn waiter_disconnect_triggers_reservation_cancellation_or_kill() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
        let addr = listener.local_addr().unwrap();
        let token = "stop-token".to_string();
        let (cancel_tx, cancel_rx) = tokio::sync::oneshot::channel();

        let host = Arc::new(FakeHost {
            reservation_id: 42,
            launch_error: None,
            prep_error: None,
            started_env: Mutex::new(Vec::new()),
            finish_notify: Arc::new(tokio::sync::Notify::new()),
            cancelled_tx: Mutex::new(Some(cancel_tx)),
        });

        tokio::spawn(async move {
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
        let mut payload = serde_json::to_vec(&req).unwrap();
        payload.push(b'\n');
        write_half.write_all(&payload).await.unwrap();
        write_half.flush().await.unwrap();

        let ack_line = read_bounded_line(&mut reader).await.unwrap().unwrap();
        let ack: WaiterLaunchResponse = serde_json::from_str(&ack_line).unwrap();
        assert_eq!(ack, WaiterLaunchResponse::Accepted { reservation_id: 42 });

        // Simulate Steam clicking "Stop" and terminating the waiting CLI process
        drop(reader);
        drop(write_half);

        let cancelled = cancel_rx.await.unwrap();
        assert_eq!(cancelled, ("inst-steam-stop".to_string(), 42));
    }

    #[tokio::test]
    async fn launch_failure_returns_nonzero_exit_code_and_stale_file_returns_none() {
        let temp = tempfile::tempdir().unwrap();
        let stale_file = temp.path().join("stale.json");
        write_endpoint_file(
            &stale_file,
            &WaiterEndpointInfo {
                port: 1,
                token: "stale".to_string(),
                pid: 999_999,
            },
        )
        .unwrap();
        assert_eq!(
            delegate_launch_with_file(&stale_file, "inst-1", Vec::new()).await,
            None
        );

        let active_file = temp.path().join("active.json");
        let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
        let port = listener.local_addr().unwrap().port();
        write_endpoint_file(
            &active_file,
            &WaiterEndpointInfo {
                port,
                token: "token".to_string(),
                pid: 999_999,
            },
        )
        .unwrap();

        let host = Arc::new(FakeHost {
            reservation_id: 1,
            launch_error: Some("Instance is already running".to_string()),
            prep_error: None,
            started_env: Mutex::new(Vec::new()),
            finish_notify: Arc::new(tokio::sync::Notify::new()),
            cancelled_tx: Mutex::new(None),
        });
        tokio::spawn(async move {
            serve_waiter_listener(listener, "token".to_string(), host).await;
        });

        let code = delegate_launch_with_file(&active_file, "inst-1", Vec::new()).await;
        assert_eq!(code, Some(1));
    }
}
