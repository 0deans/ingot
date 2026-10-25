//! Bounded native termination of an original process and its observed descendants.

mod command;
mod inspection;
#[cfg(test)]
mod tests;

use std::io;
use std::process::ExitStatus;
use std::time::Duration;

const OPERATION_TIMEOUT: Duration = Duration::from_secs(30);
const EXIT_TIMEOUT: Duration = Duration::from_secs(5);
const POLL_INTERVAL: Duration = Duration::from_millis(50);
// Native command cleanup has its own deadline and must finish before an outer
// timeout may drop its future. The extra margin also bounds faulty effect adapters.
pub(crate) const MAX_TERMINATION_DURATION: Duration = OPERATION_TIMEOUT
    .saturating_add(command::CLEANUP_TIMEOUT)
    .saturating_add(Duration::from_secs(1));

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ProcessIdentity {
    pid: u32,
    proc_start: u64,
}

impl ProcessIdentity {
    pub(crate) fn new(pid: u32, proc_start: u64) -> Self {
        Self { pid, proc_start }
    }

    pub(crate) fn start(self) -> u64 {
        self.proc_start
    }

    pub(crate) fn pid(self) -> u32 {
        self.pid
    }
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum TerminationError {
    #[error("Original OS identity is unavailable for process {0}")]
    IdentityUnavailable(u32),
    #[error("Process-tree termination failed: {primary}; additional failures: {additional:?}")]
    Failures {
        #[source]
        primary: Box<TerminationError>,
        additional: Vec<TerminationError>,
    },
    #[error("Cannot spawn termination command for process {pid}: {source}")]
    Spawn {
        pid: u32,
        #[source]
        source: io::Error,
    },
    #[error("Cannot wait for termination command for process {pid}: {source}")]
    Wait {
        pid: u32,
        #[source]
        source: io::Error,
    },
    #[error("Termination command for process {pid} exited with {status}")]
    CommandFailed { pid: u32, status: ExitStatus },
    #[error("Termination command for process {0} exceeded its deadline")]
    CommandTimeout(u32),
    #[error("Termination exceeded its deadline before process {0} could be stopped")]
    OperationTimeout(u32),
    #[error(
        "Termination command cleanup failed after {primary}; signal: {signal:?}; reap: {reap:?}"
    )]
    Cleanup {
        #[source]
        primary: Box<TerminationError>,
        signal: Option<io::Error>,
        reap: Option<io::Error>,
    },
    #[error("Exit was not confirmed for original process {0}")]
    ExitUnconfirmed(u32),
    #[error("Process inspection worker failed: {0}")]
    Inspection(#[source] tokio::task::JoinError),
    #[error("Process termination worker failed: {0}")]
    Worker(#[source] tokio::task::JoinError),
}

pub(crate) fn capture(pid: u32) -> Option<ProcessIdentity> {
    inspection::capture(pid)
}

/// Once polled, termination owns its worker until command cleanup and exit checks finish.
/// Unix targets cover the observed descendant snapshot; children created or reparented
/// after that snapshot cannot be safely inferred from a numeric PID alone.
pub(crate) async fn terminate(identity: ProcessIdentity) -> Result<(), TerminationError> {
    if identity.pid == 0 || identity.proc_start == 0 {
        return Err(TerminationError::IdentityUnavailable(identity.pid));
    }
    owned(async move { terminate_with(identity, inspection::Native).await }).await
}

async fn owned(
    operation: impl std::future::Future<Output = Result<(), TerminationError>> + Send + 'static,
) -> Result<(), TerminationError> {
    let worker = tokio::spawn(async move {
        let result = operation.await;
        if let Err(error) = &result {
            eprintln!("Process-tree termination failed: {error}");
        }
        result
    });
    worker.await.map_err(TerminationError::Worker)?
}

trait Effects: Send + Sync {
    fn snapshot(
        &self,
        root: ProcessIdentity,
    ) -> impl std::future::Future<Output = Result<Vec<ProcessIdentity>, TerminationError>> + Send;
    fn alive(
        &self,
        identity: ProcessIdentity,
    ) -> impl std::future::Future<Output = Result<bool, TerminationError>> + Send;
    fn stop(
        &self,
        identity: ProcessIdentity,
        deadline: tokio::time::Instant,
    ) -> impl std::future::Future<Output = Result<(), TerminationError>> + Send;
}

async fn terminate_with(
    root: ProcessIdentity,
    effects: impl Effects,
) -> Result<(), TerminationError> {
    let started = tokio::time::Instant::now();
    let deadline = started + OPERATION_TIMEOUT;
    let cleanup_deadline = started + MAX_TERMINATION_DURATION;
    let targets = inspect_before(deadline, root.pid, effects.snapshot(root)).await?;
    let mut failures = Vec::new();
    for target in &targets {
        if tokio::time::Instant::now() >= deadline {
            failures.push(TerminationError::OperationTimeout(target.pid));
            break;
        }
        match inspect_before(deadline, target.pid, effects.alive(*target)).await {
            Ok(false) => continue,
            Ok(true) => {}
            Err(error) => {
                failures.push(error);
                continue;
            }
        }
        let result = tokio::time::timeout_at(cleanup_deadline, effects.stop(*target, deadline))
            .await
            .unwrap_or_else(|_| Err(TerminationError::OperationTimeout(target.pid)));
        if let Err(error) = result {
            failures.push(error);
        }
    }
    let exit_deadline = deadline.min(tokio::time::Instant::now() + EXIT_TIMEOUT);
    for target in targets {
        loop {
            if tokio::time::Instant::now() >= exit_deadline {
                failures.push(TerminationError::ExitUnconfirmed(target.pid));
                break;
            }
            match inspect_before(exit_deadline, target.pid, effects.alive(target)).await {
                Ok(false) => break,
                Ok(true) => {}
                Err(error) => {
                    failures.push(error);
                    break;
                }
            }
            tokio::time::sleep(POLL_INTERVAL).await;
        }
    }
    let mut failures = failures.into_iter();
    let Some(primary) = failures.next() else {
        return Ok(());
    };
    let additional = failures.collect::<Vec<_>>();
    if additional.is_empty() {
        return Err(primary);
    }
    Err(TerminationError::Failures {
        primary: Box::new(primary),
        additional,
    })
}

async fn inspect_before<T>(
    deadline: tokio::time::Instant,
    pid: u32,
    inspection: impl std::future::Future<Output = Result<T, TerminationError>>,
) -> Result<T, TerminationError> {
    tokio::time::timeout_at(deadline, inspection)
        .await
        .unwrap_or_else(|_| Err(TerminationError::OperationTimeout(pid)))
}
