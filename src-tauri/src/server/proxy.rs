pub mod protocol;
#[cfg(test)]
mod tests;

pub use protocol::*;

use std::io;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::broadcast;

/// Shared proxy state configuration
#[derive(Clone)]
pub struct ProxyConfig {
    pub server_id: String,
    pub server_name: String,
    pub public_port: u16,
    pub internal_port: u16,
    pub max_players: u32,
    pub sleep_enabled: bool,
}

/// Handle a single incoming client connection to the proxy
pub async fn handle_proxy_client<FWake, FIsRunning, FIsSleeping>(
    mut client_stream: TcpStream,
    config: ProxyConfig,
    active_conns: Arc<AtomicUsize>,
    wake_fn: Arc<FWake>,
    is_running_fn: Arc<FIsRunning>,
    is_sleeping_fn: Arc<FIsSleeping>,
) -> io::Result<()>
where
    FWake: Fn() -> tokio::sync::oneshot::Receiver<bool> + Send + Sync + 'static,
    FIsRunning: Fn() -> bool + Send + Sync + 'static,
    FIsSleeping: Fn() -> bool + Send + Sync + 'static,
{
    let handshake = read_handshake_packet(&mut client_stream).await?;

    match handshake.next_state {
        1 => {
            // NextState = 1: Status (MOTD Server List Ping)
            if is_sleeping_fn() {
                handle_synthetic_status_query(
                    &mut client_stream,
                    &config.server_name,
                    config.max_players,
                    handshake.protocol_version,
                )
                .await?;
            } else if !is_running_fn() {
                handle_synthetic_booting_status(
                    &mut client_stream,
                    &config.server_name,
                    config.max_players,
                    handshake.protocol_version,
                )
                .await?;
            } else {
                if let Ok(mut server_stream) =
                    TcpStream::connect(format!("127.0.0.1:{}", config.internal_port)).await
                {
                    server_stream.write_all(&handshake.raw_bytes).await?;
                    let _ =
                        tokio::io::copy_bidirectional(&mut client_stream, &mut server_stream).await;
                }
            }
        }
        2 => {
            // NextState = 2: Login (Player connecting to play)
            if is_sleeping_fn() {
                let wake_rx = wake_fn();
                let wake_result = tokio::time::timeout(Duration::from_secs(25), wake_rx).await;
                let is_ready = matches!(wake_result, Ok(Ok(true)));

                if is_ready {
                    if let Ok(mut server_stream) =
                        TcpStream::connect(format!("127.0.0.1:{}", config.internal_port)).await
                    {
                        server_stream.write_all(&handshake.raw_bytes).await?;
                        active_conns.fetch_add(1, Ordering::SeqCst);
                        let _ =
                            tokio::io::copy_bidirectional(&mut client_stream, &mut server_stream)
                                .await;
                        active_conns.fetch_sub(1, Ordering::SeqCst);
                        return Ok(());
                    }
                }

                let disconnect_pkt = make_login_disconnect_packet(
                    "§6[Ingot Mobile] §eServer is waking up from sleep!\n§aPlease reconnect in 10 seconds.",
                );
                let _ = client_stream.write_all(&disconnect_pkt).await;
                let _ = client_stream.flush().await;
            } else if is_running_fn() {
                let mut server_stream =
                    TcpStream::connect(format!("127.0.0.1:{}", config.internal_port))
                        .await
                        .map_err(|e| {
                            io::Error::new(
                                io::ErrorKind::ConnectionRefused,
                                format!(
                                    "Could not connect to internal server port {}: {e}",
                                    config.internal_port
                                ),
                            )
                        })?;

                server_stream.write_all(&handshake.raw_bytes).await?;
                active_conns.fetch_add(1, Ordering::SeqCst);
                let _ = tokio::io::copy_bidirectional(&mut client_stream, &mut server_stream).await;
                active_conns.fetch_sub(1, Ordering::SeqCst);
            } else {
                for _ in 0..40 {
                    tokio::time::sleep(Duration::from_millis(500)).await;
                    if let Ok(mut server_stream) =
                        TcpStream::connect(format!("127.0.0.1:{}", config.internal_port)).await
                    {
                        server_stream.write_all(&handshake.raw_bytes).await?;
                        active_conns.fetch_add(1, Ordering::SeqCst);
                        let _ =
                            tokio::io::copy_bidirectional(&mut client_stream, &mut server_stream)
                                .await;
                        active_conns.fetch_sub(1, Ordering::SeqCst);
                        return Ok(());
                    }
                }

                let disconnect_pkt = make_login_disconnect_packet(
                    "§6[Ingot Mobile] §eServer is still starting up. Please reconnect in a moment!",
                );
                let _ = client_stream.write_all(&disconnect_pkt).await;
                let _ = client_stream.flush().await;
            }
        }
        _ => {}
    }

    Ok(())
}

async fn handle_synthetic_status_query<S: AsyncRead + AsyncWrite + Unpin>(
    stream: &mut S,
    server_name: &str,
    max_players: u32,
    protocol: i32,
) -> io::Result<()> {
    let _req_len = read_varint(stream).await?;
    let req_id = read_varint(stream).await?;
    if req_id != 0x00 {
        return Ok(());
    }

    let json_resp = make_sleeping_status_json(server_name, max_players, protocol);
    let mut payload = Vec::new();
    encode_varint(json_resp.len() as i32, &mut payload);
    payload.extend_from_slice(json_resp.as_bytes());

    let packet = encode_packet(0x00, &payload);
    stream.write_all(&packet).await?;
    stream.flush().await?;

    if let Ok(_ping_len) = read_varint(stream).await {
        if let Ok(ping_id) = read_varint(stream).await {
            if ping_id == 0x01 {
                let mut ping_payload = [0u8; 8];
                if stream.read_exact(&mut ping_payload).await.is_ok() {
                    let pong_packet = encode_packet(0x01, &ping_payload);
                    let _ = stream.write_all(&pong_packet).await;
                    let _ = stream.flush().await;
                }
            }
        }
    }

    Ok(())
}

async fn handle_synthetic_booting_status<S: AsyncRead + AsyncWrite + Unpin>(
    stream: &mut S,
    server_name: &str,
    max_players: u32,
    protocol: i32,
) -> io::Result<()> {
    let _req_len = read_varint(stream).await?;
    let req_id = read_varint(stream).await?;
    if req_id != 0x00 {
        return Ok(());
    }

    let json_resp = make_booting_status_json(server_name, max_players, protocol);
    let mut payload = Vec::new();
    encode_varint(json_resp.len() as i32, &mut payload);
    payload.extend_from_slice(json_resp.as_bytes());

    let packet = encode_packet(0x00, &payload);
    stream.write_all(&packet).await?;
    stream.flush().await?;

    if let Ok(_ping_len) = read_varint(stream).await {
        if let Ok(ping_id) = read_varint(stream).await {
            if ping_id == 0x01 {
                let mut ping_payload = [0u8; 8];
                if stream.read_exact(&mut ping_payload).await.is_ok() {
                    let pong_packet = encode_packet(0x01, &ping_payload);
                    let _ = stream.write_all(&pong_packet).await;
                    let _ = stream.flush().await;
                }
            }
        }
    }

    Ok(())
}

/// Manages a running TCP proxy instance
pub struct ServerProxyHandle {
    pub shutdown_tx: broadcast::Sender<()>,
    pub active_connections: Arc<AtomicUsize>,
    pub is_running: Arc<AtomicBool>,
    listener_task: tokio::task::JoinHandle<()>,
}

impl ServerProxyHandle {
    pub(crate) async fn shutdown(self) {
        let _ = self.shutdown_tx.send(());
        let _ = self.listener_task.await;
    }
}

/// Spawns the TCP proxy listener on public_port
pub async fn start_server_proxy<FWake, FIsRunning, FIsSleeping>(
    config: ProxyConfig,
    wake_fn: FWake,
    is_running_fn: FIsRunning,
    is_sleeping_fn: FIsSleeping,
) -> io::Result<ServerProxyHandle>
where
    FWake: Fn() -> tokio::sync::oneshot::Receiver<bool> + Send + Sync + 'static,
    FIsRunning: Fn() -> bool + Send + Sync + 'static,
    FIsSleeping: Fn() -> bool + Send + Sync + 'static,
{
    let listener = TcpListener::bind(format!("0.0.0.0:{}", config.public_port)).await?;
    let (shutdown_tx, _) = broadcast::channel(1);
    let active_connections = Arc::new(AtomicUsize::new(0));
    let is_running = Arc::new(AtomicBool::new(true));

    let mut shutdown_rx = shutdown_tx.subscribe();
    let conns_clone = active_connections.clone();
    let is_running_clone = is_running.clone();
    let wake_arc = Arc::new(wake_fn);
    let is_running_arc = Arc::new(is_running_fn);
    let is_sleeping_arc = Arc::new(is_sleeping_fn);

    let listener_task = tokio::spawn(async move {
        loop {
            tokio::select! {
                accept_res = listener.accept() => {
                    match accept_res {
                        Ok((stream, _addr)) => {
                            let cfg = config.clone();
                            let conns = conns_clone.clone();
                            let wake = wake_arc.clone();
                            let running = is_running_arc.clone();
                            let sleeping = is_sleeping_arc.clone();

                            tokio::spawn(async move {
                                let _ = handle_proxy_client(
                                    stream,
                                    cfg,
                                    conns,
                                    wake,
                                    running,
                                    sleeping,
                                ).await;
                            });
                        }
                        Err(e) => {
                            eprintln!("[Proxy] Accept error: {e}");
                            tokio::time::sleep(Duration::from_millis(100)).await;
                        }
                    }
                }
                _ = shutdown_rx.recv() => {
                    is_running_clone.store(false, Ordering::SeqCst);
                    break;
                }
            }
        }
    });

    Ok(ServerProxyHandle {
        shutdown_tx,
        active_connections,
        is_running,
        listener_task,
    })
}
