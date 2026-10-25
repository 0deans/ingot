//! Shutdown ownership belongs to the original process, independently of RPC waiters.
use super::manager::ServerProcessManager;
use super::models::{ActiveServer, ProcessError, ServerLaunchGeneration, ServerTerminationError};
use crate::process_tree::{self, ProcessIdentity, TerminationError};
use std::{future::Future, pin::Pin, sync::Arc, time::Duration};
use tokio::{
    io::AsyncWriteExt,
    process::ChildStdin,
    sync::{Mutex, OnceCell},
};

const MONITOR_TIMEOUT: Duration = Duration::from_secs(3);
const POLL_INTERVAL: Duration = Duration::from_millis(50);
type StopResult = Result<(), Arc<TerminationError>>;
type StopFuture<'a> = Pin<Box<dyn Future<Output = Result<(), TerminationError>> + Send + 'a>>;

trait StopEffect: Send + Sync {
    fn terminate(&self, identity: ProcessIdentity) -> StopFuture<'_>;
}
struct NativeStop;
impl StopEffect for NativeStop {
    fn terminate(&self, identity: ProcessIdentity) -> StopFuture<'_> {
        Box::pin(process_tree::terminate(identity))
    }
}

pub(crate) struct ServerTermination {
    pid: u32,
    identity: Option<ProcessIdentity>,
    attempt: Mutex<Arc<OnceCell<StopResult>>>,
    effect: Arc<dyn StopEffect>,
}
impl ServerTermination {
    pub(crate) fn capture(pid: u32) -> Arc<Self> {
        Arc::new(Self {
            pid,
            identity: process_tree::capture(pid),
            attempt: Mutex::new(Arc::new(OnceCell::new())),
            effect: Arc::new(NativeStop),
        })
    }

    async fn terminate(self: Arc<Self>) -> Result<(), ProcessError> {
        let attempt = {
            let mut current = self.attempt.lock().await;
            if current.get().is_some_and(Result::is_err) {
                *current = Arc::new(OnceCell::new());
            }
            current.clone()
        };
        owned(async move {
            let result = attempt
                .get_or_init(|| async {
                    let Some(identity) = self.identity else {
                        if process_tree::capture(self.pid).is_none() {
                            return Ok(());
                        }
                        return Err(Arc::new(TerminationError::IdentityUnavailable(self.pid)));
                    };
                    self.effect.terminate(identity).await.map_err(Arc::new)
                })
                .await;
            result
                .clone()
                .map_err(|source| ProcessError::Termination(ServerTerminationError { source }))
        })
        .await
    }
}

struct StopTarget {
    server_id: String,
    pid: u32,
    stdin: Arc<Mutex<ChildStdin>>,
    termination: Arc<ServerTermination>,
}
impl StopTarget {
    fn claim(server: &mut ActiveServer) -> Self {
        server.status = crate::server::config::ServerStatus::Stopping;
        Self {
            server_id: server.server_id.clone(),
            pid: server.pid,
            stdin: server.stdin.clone(),
            termination: server.termination.clone(),
        }
    }
}

async fn owned(
    operation: impl Future<Output = Result<(), ProcessError>> + Send + 'static,
) -> Result<(), ProcessError> {
    tokio::spawn(async move {
        let result = operation.await;
        if let Err(error) = &result {
            eprintln!("Server shutdown failed: {error}");
        }
        result
    })
    .await
    .map_err(ProcessError::ShutdownWorker)?
}

impl ServerProcessManager {
    async fn shutdown_target(&self, server_id: &str) -> Result<StopTarget, ProcessError> {
        let mut servers = self.servers.lock().await;
        let server = servers
            .get_mut(server_id)
            .ok_or_else(|| ProcessError::NotRunning(server_id.to_owned()))?;
        Ok(StopTarget::claim(server))
    }

    async fn shutdown_target_if_owned(
        &self,
        server_id: &str,
        pid: u32,
        generation: &ServerLaunchGeneration,
    ) -> Option<StopTarget> {
        let mut servers = self.servers.lock().await;
        let server = servers
            .get_mut(server_id)
            .filter(|server| server.pid == pid && server.generation.matches(generation))?;
        Some(StopTarget::claim(server))
    }

    async fn owns_target(&self, target: &StopTarget) -> bool {
        self.servers
            .lock()
            .await
            .get(&target.server_id)
            .is_some_and(|server| {
                server.pid == target.pid && Arc::ptr_eq(&server.termination, &target.termination)
            })
    }

    async fn target_exited(&self, target: &StopTarget) {
        while self.owns_target(target).await {
            tokio::time::sleep(POLL_INTERVAL).await;
        }
    }

    async fn force_target(&self, target: &StopTarget) -> Result<(), ProcessError> {
        if !self.owns_target(target).await {
            return Ok(());
        }
        target.termination.clone().terminate().await?;
        tokio::time::timeout(MONITOR_TIMEOUT, self.target_exited(target))
            .await
            .map_err(|_| ProcessError::ShutdownTimeout {
                server_id: target.server_id.clone(),
                timeout: MONITOR_TIMEOUT,
            })
    }

    pub async fn kill_server(&self, server_id: &str) -> Result<(), ProcessError> {
        self.kill_server_typed(server_id).await
    }

    pub async fn kill_server_typed(&self, server_id: &str) -> Result<(), ProcessError> {
        let target = self.shutdown_target(server_id).await?;
        let manager = self.clone();
        owned(async move { manager.force_target(&target).await }).await
    }

    pub(crate) async fn kill_if_owned(
        &self,
        server_id: &str,
        pid: u32,
        generation: &ServerLaunchGeneration,
    ) -> Result<(), ProcessError> {
        let Some(target) = self
            .shutdown_target_if_owned(server_id, pid, generation)
            .await
        else {
            return Ok(());
        };
        let manager = self.clone();
        owned(async move { manager.force_target(&target).await }).await
    }

    pub async fn stop_server(&self, server_id: &str) -> Result<(), ProcessError> {
        self.stop_server_within(server_id, Duration::from_secs(10))
            .await
    }

    pub async fn stop_server_within(
        &self,
        server_id: &str,
        timeout: Duration,
    ) -> Result<(), ProcessError> {
        self.stop_server_within_typed(server_id, timeout).await
    }

    pub async fn stop_server_within_typed(
        &self,
        server_id: &str,
        timeout: Duration,
    ) -> Result<(), ProcessError> {
        let target = self.shutdown_target(server_id).await?;
        let manager = self.clone();
        let deadline = tokio::time::Instant::now() + timeout;
        // This acknowledges admission; the owned worker reports failures even if no caller waits.
        tokio::spawn(async move {
            if let Err(error) =
                owned(async move { manager.graceful_target(target, deadline).await }).await
            {
                eprintln!("Detached server shutdown failed: {error}");
            }
        });
        Ok(())
    }

    pub(crate) async fn stop_server_and_wait(
        &self,
        server_id: &str,
        grace: Duration,
    ) -> Result<(), ProcessError> {
        let target = self.shutdown_target(server_id).await?;
        let manager = self.clone();
        let deadline = tokio::time::Instant::now() + grace;
        owned(async move { manager.graceful_target(target, deadline).await }).await
    }

    pub(crate) async fn stop_owned_and_wait(
        &self,
        server_id: &str,
        pid: u32,
        generation: &ServerLaunchGeneration,
        grace: Duration,
    ) -> Result<(), ProcessError> {
        let Some(target) = self
            .shutdown_target_if_owned(server_id, pid, generation)
            .await
        else {
            return Ok(());
        };
        let manager = self.clone();
        let deadline = tokio::time::Instant::now() + grace;
        owned(async move { manager.graceful_target(target, deadline).await }).await
    }

    async fn graceful_target(
        &self,
        target: StopTarget,
        deadline: tokio::time::Instant,
    ) -> Result<(), ProcessError> {
        let grace_period = deadline.saturating_duration_since(tokio::time::Instant::now());
        let write_stop = async {
            let mut stdin = target.stdin.lock().await;
            stdin
                .write_all(b"stop\n")
                .await
                .map_err(ProcessError::StdinWrite)?;
            stdin.flush().await.map_err(ProcessError::StdinFlush)
        };
        let graceful_failure = tokio::select! {
            _ = self.target_exited(&target) => return Ok(()),
            result = tokio::time::timeout_at(deadline, write_stop) => match result {
                Ok(result) => result.err(),
                Err(_) => Some(ProcessError::ShutdownTimeout {
                    server_id: target.server_id.clone(), timeout: grace_period,
                }),
            }
        };
        tokio::select! {
            _ = self.target_exited(&target) => return Ok(()),
            _ = tokio::time::sleep_until(deadline) => {}
        }
        // A failed/blocked graceful write is recovered only by confirmed process exit.
        match self.force_target(&target).await {
            Ok(()) => Ok(()),
            Err(termination) => match graceful_failure {
                Some(graceful) => Err(ProcessError::ShutdownFailed {
                    graceful: Box::new(graceful),
                    termination: Box::new(termination),
                }),
                None => Err(termination),
            },
        }
    }
}

#[cfg(test)]
mod tests;
