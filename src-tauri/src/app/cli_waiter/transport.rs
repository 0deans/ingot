//! Local listener, connection session ownership, and delegation client transport.

use super::endpoint::read_delegation_endpoint;
use super::protocol::{
    read_bounded_line, send_message, validate_launch_request, WaiterLaunchRequest,
    WaiterLaunchResponse,
};
use super::WaiterSessionHost;
use std::io;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, BufReader};
use tokio::net::{TcpListener, TcpStream};

const CONNECT_TIMEOUT: Duration = Duration::from_millis(500);
pub(super) const RESERVATION_ACK_TIMEOUT: Duration = Duration::from_secs(15);

struct ActiveReservation<H: WaiterSessionHost> {
    host: Arc<H>,
    instance_id: String,
    reservation_id: u64,
}

/// Ensures an accepted reservation is cancelled if sending `Accepted` fails or the
/// waiter session task is dropped before the reservation finishes normally.
struct ReservationGuard<H: WaiterSessionHost> {
    active: Option<ActiveReservation<H>>,
}

impl<H: WaiterSessionHost> ReservationGuard<H> {
    fn new(host: Arc<H>, instance_id: String, reservation_id: u64) -> Self {
        Self {
            active: Some(ActiveReservation {
                host,
                instance_id,
                reservation_id,
            }),
        }
    }

    fn disarm(&mut self) {
        self.active = None;
    }

    async fn cancel_and_disarm(&mut self) {
        if let Some(active) = &self.active {
            active
                .host
                .cancel_or_kill_reservation(active.instance_id.clone(), active.reservation_id)
                .await;
            self.active = None;
        }
    }
}

impl<H: WaiterSessionHost> Drop for ReservationGuard<H> {
    fn drop(&mut self) {
        let Some(active) = self.active.take() else {
            return;
        };
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            handle.spawn(async move {
                active
                    .host
                    .cancel_or_kill_reservation(active.instance_id, active.reservation_id)
                    .await;
            });
        }
    }
}

pub(super) async fn serve_waiter_listener<H: WaiterSessionHost>(
    listener: TcpListener,
    expected_token: String,
    host: Arc<H>,
) {
    let mut connections = tokio::task::JoinSet::new();
    loop {
        tokio::select! {
            accepted = listener.accept() => {
                let Ok((stream, _)) = accepted else {
                    break;
                };
                let token = expected_token.clone();
                let host = Arc::clone(&host);
                connections.spawn(async move {
                    let (read_half, write_half) = stream.into_split();
                    let _ = handle_waiter_io(read_half, write_half, &token, host).await;
                });
            }
            Some(_) = connections.join_next(), if !connections.is_empty() => {}
        }
    }
    while connections.join_next().await.is_some() {}
}

pub(super) async fn handle_waiter_io<R, W, H>(
    read_half: R,
    mut write_half: W,
    expected_token: &str,
    host: Arc<H>,
) -> io::Result<()>
where
    R: tokio::io::AsyncRead + Unpin,
    W: tokio::io::AsyncWrite + Unpin,
    H: WaiterSessionHost,
{
    let mut reader = BufReader::new(read_half);

    let Some(request_line) = read_bounded_line(&mut reader).await? else {
        return Ok(());
    };

    let validated = match validate_launch_request(&request_line, expected_token) {
        Ok(req) => req,
        Err(err) => {
            send_message(&mut write_half, &err.into_response()).await?;
            return Ok(());
        }
    };

    let reserved = match host
        .start_launch(validated.instance_id.clone(), validated.steam_env)
        .await
    {
        Ok(reserved) => reserved,
        Err(error) => {
            send_message(
                &mut write_half,
                &WaiterLaunchResponse::Failed {
                    message: error.to_string(),
                },
            )
            .await?;
            return Ok(());
        }
    };

    let reservation_id = reserved.reservation_id;
    let mut reservation_guard = ReservationGuard::new(
        Arc::clone(&host),
        validated.instance_id.clone(),
        reservation_id,
    );

    if let Err(err) = send_message(
        &mut write_half,
        &WaiterLaunchResponse::Accepted { reservation_id },
    )
    .await
    {
        reservation_guard.cancel_and_disarm().await;
        return Err(err);
    }

    let reservation_done = host.wait_for_reservation_end(validated.instance_id, reservation_id);
    let client_disconnected = wait_for_client_disconnect(&mut reader);

    tokio::select! {
        () = reservation_done => {
            reservation_guard.disarm();
            let response = resolve_launch_outcome(reserved.launch_result.await);
            send_message(&mut write_half, &response).await?;
        }
        () = client_disconnected => {
            reservation_guard.cancel_and_disarm().await;
        }
    }

    Ok(())
}

fn resolve_launch_outcome(
    launch_result: Result<
        Result<u32, super::WaiterLaunchError>,
        tokio::sync::oneshot::error::RecvError,
    >,
) -> WaiterLaunchResponse {
    match launch_result {
        Ok(Ok(_pid)) => WaiterLaunchResponse::Finished { exit_code: 0 },
        Ok(Err(error)) => WaiterLaunchResponse::Failed {
            message: error.to_string(),
        },
        Err(_) => WaiterLaunchResponse::Finished { exit_code: 0 },
    }
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

pub(super) async fn delegate_launch_with_file(
    endpoint_path: &Path,
    instance_id: &str,
    steam_env: Vec<(String, String)>,
) -> Option<i32> {
    let info = read_delegation_endpoint(endpoint_path, std::process::id())?;

    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), info.port);
    let stream = match tokio::time::timeout(CONNECT_TIMEOUT, TcpStream::connect(addr)).await {
        Ok(Ok(stream)) => stream,
        _ => return None,
    };

    let (read_half, write_half) = stream.into_split();
    delegate_launch_over_io(read_half, write_half, info.token, instance_id, steam_env).await
}

pub(super) async fn delegate_launch_over_io<R, W>(
    read_half: R,
    mut write_half: W,
    token: String,
    instance_id: &str,
    steam_env: Vec<(String, String)>,
) -> Option<i32>
where
    R: tokio::io::AsyncRead + Unpin,
    W: tokio::io::AsyncWrite + Unpin,
{
    let mut reader = BufReader::new(read_half);

    let request = WaiterLaunchRequest {
        token,
        instance_id: instance_id.to_string(),
        steam_env,
    };
    if send_message(&mut write_half, &request).await.is_err() {
        return None;
    }

    let first_line =
        match tokio::time::timeout(RESERVATION_ACK_TIMEOUT, read_bounded_line(&mut reader)).await {
            Ok(Ok(Some(line))) => line,
            _ => return None,
        };

    let first_response: WaiterLaunchResponse = match serde_json::from_str(&first_line) {
        Ok(response) => response,
        Err(_) => return None,
    };
    match first_response {
        WaiterLaunchResponse::Accepted { .. } => {}
        WaiterLaunchResponse::Finished { exit_code } => return Some(exit_code),
        WaiterLaunchResponse::Failed { message } => {
            eprintln!("[CLI] {message}");
            return Some(1);
        }
    }

    let exit_code = match read_bounded_line(&mut reader).await {
        Ok(Some(second_line)) => match serde_json::from_str::<WaiterLaunchResponse>(&second_line) {
            Ok(response) => interpret_terminal_response(response),
            Err(_) => 0,
        },
        Ok(None) | Err(_) => 0,
    };
    Some(exit_code)
}

fn interpret_terminal_response(response: WaiterLaunchResponse) -> i32 {
    match response {
        WaiterLaunchResponse::Finished { exit_code } => exit_code,
        WaiterLaunchResponse::Failed { message } => {
            eprintln!("[CLI] {message}");
            1
        }
        WaiterLaunchResponse::Accepted { .. } => 0,
    }
}
