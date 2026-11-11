//! Native Tauri integration for the CLI waiter host and listener startup.

use super::endpoint::{cleanup_endpoint_file_at, write_endpoint_file};
use super::protocol::WaiterEndpointInfo;
use super::transport::serve_waiter_listener;
use super::{ReservedLaunch, WaiterLaunchError, WaiterSessionHost};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;
use tauri::{AppHandle, Manager, Runtime};
use tokio::net::TcpListener;

/// Aborts a spawned launch task if `start_launch` is dropped before reservation completes.
struct PreReservationTaskGuard(Option<tokio::task::JoinHandle<()>>);

impl PreReservationTaskGuard {
    fn disarm(&mut self) {
        self.0 = None;
    }

    fn take_handle(&mut self) -> Option<tokio::task::JoinHandle<()>> {
        self.0.take()
    }
}

impl Drop for PreReservationTaskGuard {
    fn drop(&mut self) {
        if let Some(handle) = self.0.take() {
            handle.abort();
        }
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
    ) -> Result<ReservedLaunch, WaiterLaunchError> {
        let (reserved_tx, reserved_rx) = tokio::sync::oneshot::channel();
        let (result_tx, result_rx) = tokio::sync::oneshot::channel();
        let handle = self.app.clone();
        let id_for_launch = instance_id.clone();

        let launch_task = tokio::spawn(async move {
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
                    crate::app::cli::restore_window_on_cli_failure(&handle, &err);
                    let _ = result_tx.send(Err(WaiterLaunchError::Launch(err)));
                }
            }
        });
        let mut task_guard = PreReservationTaskGuard(Some(launch_task));

        match reserved_rx.await {
            Ok(reservation_id) => {
                task_guard.disarm();
                Ok(ReservedLaunch {
                    reservation_id,
                    launch_result: result_rx,
                })
            }
            Err(_) => {
                let launch_task = task_guard.take_handle();
                match result_rx.await {
                    Ok(Err(err)) => Err(err),
                    Ok(Ok(_)) | Err(_) => match launch_task {
                        Some(handle) => match handle.await {
                            Ok(()) => Err(WaiterLaunchError::EndedBeforeReservation),
                            Err(join_err) => Err(WaiterLaunchError::Task(join_err)),
                        },
                        None => Err(WaiterLaunchError::EndedBeforeReservation),
                    },
                }
            }
        }
    }

    async fn wait_for_reservation_end(&self, instance_id: String, reservation_id: u64) {
        let pm = crate::app::state(&self.app).games.clone();
        pm.wait_for_reservation_end(&instance_id, reservation_id)
            .await;
    }

    async fn cancel_or_kill_reservation(&self, instance_id: String, reservation_id: u64) {
        let pm = crate::app::state(&self.app).games.clone();
        if let Err(err) = pm
            .cancel_or_kill_reservation(&instance_id, reservation_id)
            .await
        {
            eprintln!("[CLI] Failed to stop instance {instance_id} after waiter disconnect: {err}");
        }
    }
}

pub(super) fn start_server<R: Runtime>(app: &AppHandle<R>) {
    let Ok(data_dir) = app.path().app_data_dir() else {
        return;
    };
    let endpoint_path = crate::app::AppPaths::new(data_dir).cli_waiter_file();
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

pub(super) fn cleanup_endpoint_file<R: Runtime>(app: &AppHandle<R>) {
    if let Ok(data_dir) = app.path().app_data_dir() {
        let path = crate::app::AppPaths::new(data_dir).cli_waiter_file();
        cleanup_endpoint_file_at(&path, std::process::id());
    }
}
