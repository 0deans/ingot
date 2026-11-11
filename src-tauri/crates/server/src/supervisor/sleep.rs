use super::SupervisedServer;
use crate::config::{ServerStatus, ServerStatusEvent};
use crate::process::{ProcessError, ServerProcessManager};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

const IDLE_CHECK_INTERVAL: Duration = Duration::from_secs(10);
const SLEEP_STOP_TIMEOUT: Duration = Duration::from_secs(10);
const UNEXPECTED_EXIT_GRACE: Duration = Duration::from_secs(12);

#[cfg(test)]
pub(super) type TestStopFuture<'a> =
    std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), ProcessError>> + Send + 'a>>;

#[cfg(test)]
pub(super) trait TestSleepStop: Send + Sync {
    fn stop_for_sleep<'a>(
        &'a self,
        pm: &'a ServerProcessManager,
        server_id: &'a str,
        grace: Duration,
    ) -> TestStopFuture<'a>;
}

#[derive(Clone, Default)]
struct ServerSleepState {
    transition_lock: Arc<Mutex<()>>,
    invalidation_epoch: Arc<AtomicU64>,
}

enum SleepTrigger {
    Manual,
    Idle { expected_generation: u64 },
}

pub(super) struct IdleMonitorParams {
    pub(super) server_id: String,
    pub(super) generation: u64,
    pub(super) idle_timeout: u64,
    pub(super) pm: ServerProcessManager,
    pub(super) active_conns: Arc<AtomicUsize>,
    pub(super) on_status: Arc<dyn Fn(ServerStatusEvent) + Send + Sync + 'static>,
    pub(super) supervised: Arc<Mutex<HashMap<String, SupervisedServer>>>,
    pub(super) sleep: SleepCoordinator,
}

#[derive(Clone, Default)]
pub(super) struct SleepCoordinator {
    states: Arc<Mutex<HashMap<String, ServerSleepState>>>,
    #[cfg(test)]
    test_stop: Option<Arc<dyn TestSleepStop>>,
}

impl SleepCoordinator {
    #[cfg(test)]
    pub(super) fn with_test_stop(test_stop: Arc<dyn TestSleepStop>) -> Self {
        Self {
            states: Arc::new(Mutex::new(HashMap::new())),
            test_stop: Some(test_stop),
        }
    }

    async fn state_for(&self, server_id: &str) -> ServerSleepState {
        let mut states = self.states.lock().await;
        states.entry(server_id.to_owned()).or_default().clone()
    }

    pub(super) async fn invalidate(&self, server_id: &str) {
        let state = self.state_for(server_id).await;
        state.invalidation_epoch.fetch_add(1, Ordering::SeqCst);
    }

    pub(super) async fn put_to_sleep<FStatus>(
        &self,
        supervised: &Mutex<HashMap<String, SupervisedServer>>,
        pm: &ServerProcessManager,
        server_id: &str,
        on_status: &FStatus,
    ) -> Result<(), ProcessError>
    where
        FStatus: Fn(ServerStatusEvent) + Send + Sync + ?Sized,
    {
        self.transition_to_sleep(supervised, pm, server_id, SleepTrigger::Manual, on_status)
            .await
    }

    async fn stop_process(
        &self,
        pm: &ServerProcessManager,
        server_id: &str,
    ) -> Result<(), ProcessError> {
        #[cfg(test)]
        if let Some(stopper) = &self.test_stop {
            return stopper
                .stop_for_sleep(pm, server_id, SLEEP_STOP_TIMEOUT)
                .await;
        }
        pm.stop_server_and_wait(server_id, SLEEP_STOP_TIMEOUT).await
    }

    async fn transition_to_sleep<FStatus>(
        &self,
        supervised: &Mutex<HashMap<String, SupervisedServer>>,
        pm: &ServerProcessManager,
        server_id: &str,
        trigger: SleepTrigger,
        on_status: &FStatus,
    ) -> Result<(), ProcessError>
    where
        FStatus: Fn(ServerStatusEvent) + Send + Sync + ?Sized,
    {
        let state = self.state_for(server_id).await;
        let _transition_guard = state.transition_lock.lock().await;

        // Snapshot invalidation before generation so concurrent `release` (remove -> invalidate)
        // is always detected when re-validating after stopping the process.
        let initial_epoch = state.invalidation_epoch.load(Ordering::SeqCst);
        let initial_generation = match trigger {
            SleepTrigger::Idle {
                expected_generation,
            } => {
                if current_generation(supervised, server_id).await != Some(expected_generation) {
                    return Err(ProcessError::LaunchCancelled);
                }
                Some(expected_generation)
            }
            SleepTrigger::Manual => current_generation(supervised, server_id).await,
        };

        if pm.get_server_status(server_id).await != ServerStatus::Running {
            return Err(ProcessError::MustBeRunningToSleep);
        }

        self.stop_process(pm, server_id).await?;

        if !sleep_commit_still_valid(
            supervised,
            pm,
            server_id,
            &state.invalidation_epoch,
            initial_epoch,
            initial_generation,
        )
        .await
        {
            return Err(ProcessError::LaunchCancelled);
        }

        pm.set_server_status(server_id, ServerStatus::Sleeping)
            .await;
        on_status(ServerStatusEvent {
            server_id: server_id.to_string(),
            status: ServerStatus::Sleeping,
            pid: None,
        });

        Ok(())
    }
}

pub(super) fn spawn_idle_monitor(params: IdleMonitorParams) -> tokio::task::AbortHandle {
    tokio::spawn(run_idle_monitor(params)).abort_handle()
}

async fn run_idle_monitor(params: IdleMonitorParams) {
    eprintln!(
        "[Supervisor] Idle detection loop started for {} (Timeout: {}s)",
        params.server_id, params.idle_timeout
    );
    let mut idle_seconds = 0u64;

    loop {
        tokio::time::sleep(IDLE_CHECK_INTERVAL).await;

        if !is_generation_active(&params.supervised, &params.server_id, params.generation).await {
            break;
        }

        match params.pm.get_server_status(&params.server_id).await {
            ServerStatus::Stopped => {
                idle_seconds = 0;
                if handle_stopped_server(&params).await {
                    break;
                }
            }
            ServerStatus::Running => {
                idle_seconds = handle_running_server(&params, idle_seconds).await;
            }
            ServerStatus::Starting | ServerStatus::Stopping | ServerStatus::Sleeping => {
                idle_seconds = 0;
            }
        }
    }
}

async fn handle_stopped_server(params: &IdleMonitorParams) -> bool {
    let state = params.sleep.state_for(&params.server_id).await;
    if state.transition_lock.try_lock().is_err() {
        return false;
    }

    // Going to sleep passes through "stopped" briefly; still stopped a bit later means the
    // server exited on its own (crash or failed start).
    tokio::time::sleep(UNEXPECTED_EXIT_GRACE).await;

    if !is_generation_active(&params.supervised, &params.server_id, params.generation).await {
        return true;
    }
    if state.transition_lock.try_lock().is_err() {
        return false;
    }
    if params.pm.get_server_status(&params.server_id).await != ServerStatus::Stopped {
        return false;
    }

    let removed = {
        let mut guard = params.supervised.lock().await;
        if guard.get(&params.server_id).map(|s| s.generation) == Some(params.generation) {
            guard.remove(&params.server_id)
        } else {
            None
        }
    };

    if let Some(mut server) = removed {
        // Detach this task's own AbortHandle before drop so `proxy.shutdown().await` completes.
        server.idle_task = None;
        let proxy = server.proxy_handle.take();
        drop(server);
        if let Some(proxy) = proxy {
            proxy.shutdown().await;
        }
        eprintln!(
            "[Supervisor] {} exited; released its port",
            params.server_id
        );
    }

    true
}

async fn handle_running_server(params: &IdleMonitorParams, mut idle_seconds: u64) -> u64 {
    let conns = params.active_conns.load(Ordering::SeqCst);
    let online_players = params.pm.get_server_online_players(&params.server_id).await;

    if conns > 0 || !online_players.is_empty() {
        return 0;
    }

    idle_seconds += IDLE_CHECK_INTERVAL.as_secs();
    if idle_seconds < params.idle_timeout {
        return idle_seconds;
    }

    eprintln!(
        "[Supervisor] Server {} has been idle for {}s. Entering sleep mode...",
        params.server_id, idle_seconds
    );

    if let Err(error) = params
        .sleep
        .transition_to_sleep(
            &params.supervised,
            &params.pm,
            &params.server_id,
            SleepTrigger::Idle {
                expected_generation: params.generation,
            },
            params.on_status.as_ref(),
        )
        .await
    {
        match error {
            ProcessError::LaunchCancelled => {}
            ProcessError::NotRunning(_) | ProcessError::MustBeRunningToSleep => {
                eprintln!(
                    "[Supervisor] Auto-sleep skipped for {}: {error}",
                    params.server_id
                );
            }
            other => {
                eprintln!(
                    "[Supervisor] Auto-sleep stop failed for {}: {other}",
                    params.server_id
                );
            }
        }
    }

    0
}

async fn current_generation(
    supervised: &Mutex<HashMap<String, SupervisedServer>>,
    server_id: &str,
) -> Option<u64> {
    supervised
        .lock()
        .await
        .get(server_id)
        .map(|server| server.generation)
}

async fn is_generation_active(
    supervised: &Mutex<HashMap<String, SupervisedServer>>,
    server_id: &str,
    generation: u64,
) -> bool {
    current_generation(supervised, server_id).await == Some(generation)
}

async fn sleep_commit_still_valid(
    supervised: &Mutex<HashMap<String, SupervisedServer>>,
    pm: &ServerProcessManager,
    server_id: &str,
    invalidation_epoch: &AtomicU64,
    initial_epoch: u64,
    initial_generation: Option<u64>,
) -> bool {
    if invalidation_epoch.load(Ordering::SeqCst) != initial_epoch {
        return false;
    }
    if current_generation(supervised, server_id).await != initial_generation {
        return false;
    }
    pm.get_server_status(server_id).await == ServerStatus::Stopped
}
