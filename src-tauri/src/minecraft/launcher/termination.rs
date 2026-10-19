use std::future::Future;
use std::io;
use std::process::{ExitStatus, Stdio};
use std::time::Duration;

const COMMAND_TIMEOUT: Duration = Duration::from_secs(10);
const EXIT_TIMEOUT: Duration = Duration::from_secs(5);
const POLL_INTERVAL: Duration = Duration::from_millis(50);

#[derive(Debug, thiserror::Error)]
pub enum TerminationError {
    #[error("Cannot execute game termination command: {0}")]
    Command(#[source] io::Error),
    #[error("Game termination command exited with {0}")]
    CommandFailed(ExitStatus),
    #[error("Game termination command exceeded its deadline")]
    CommandTimeout,
    #[error(
        "Termination command cleanup failed after {primary}; signal: {signal:?}; reap: {reap:?}"
    )]
    CommandCleanup {
        #[source]
        primary: Box<TerminationError>,
        signal: Option<io::Error>,
        reap: Option<io::Error>,
    },
    #[error("Original OS identity is unavailable for game process {0}")]
    IdentityUnavailable(u32),
    #[error("Game process {0} did not exit before the deadline")]
    ExitUnconfirmed(u32),
}

pub(super) async fn terminate(child: super::process::ActiveChild) -> Result<(), TerminationError> {
    terminate_identity(child, run_command(child.pid), crate::running::process_start).await
}

async fn terminate_identity<F, Inspect>(
    child: super::process::ActiveChild,
    command: F,
    inspect: Inspect,
) -> Result<(), TerminationError>
where
    F: Future<Output = Result<ExitStatus, TerminationError>>,
    Inspect: Fn(u32) -> Option<u64>,
{
    let Some(expected_start) = child.proc_start else {
        if inspect(child.pid).is_none() {
            return Ok(());
        }
        return Err(TerminationError::IdentityUnavailable(child.pid));
    };
    terminate_with(child.pid, command, || {
        inspect(child.pid) == Some(expected_start)
    })
    .await
}

async fn run_command(pid: u32) -> Result<ExitStatus, TerminationError> {
    #[cfg(target_os = "windows")]
    let mut command = {
        let mut command = tokio::process::Command::new("taskkill");
        command.args(["/F", "/T", "/PID", &pid.to_string()]);
        command.creation_flags(0x08000000);
        command
    };
    #[cfg(not(target_os = "windows"))]
    let mut command = {
        let mut command = tokio::process::Command::new("kill");
        command.args(["-9", &pid.to_string()]);
        command
    };
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .kill_on_drop(true);
    let mut child = command.spawn().map_err(TerminationError::Command)?;
    wait_for_command(&mut child).await
}

trait TerminationCommand {
    fn wait(&mut self) -> impl Future<Output = io::Result<ExitStatus>> + Send;
    fn start_kill(&mut self) -> io::Result<()>;
}

impl TerminationCommand for tokio::process::Child {
    async fn wait(&mut self) -> io::Result<ExitStatus> {
        tokio::process::Child::wait(self).await
    }
    fn start_kill(&mut self) -> io::Result<()> {
        tokio::process::Child::start_kill(self)
    }
}

async fn wait_for_command(
    child: &mut impl TerminationCommand,
) -> Result<ExitStatus, TerminationError> {
    let primary = match tokio::time::timeout(COMMAND_TIMEOUT, child.wait()).await {
        Ok(Ok(status)) => return Ok(status),
        Ok(Err(source)) => TerminationError::Command(source),
        Err(_) => TerminationError::CommandTimeout,
    };
    let signal = child.start_kill().err();
    let reap = match tokio::time::timeout(EXIT_TIMEOUT, child.wait()).await {
        Ok(result) => result.err(),
        Err(_) => Some(io::Error::new(
            io::ErrorKind::TimedOut,
            "termination command cleanup deadline exceeded",
        )),
    };
    if signal.is_some() || reap.is_some() {
        return Err(TerminationError::CommandCleanup {
            primary: Box::new(primary),
            signal,
            reap,
        });
    }
    Err(primary)
}

async fn terminate_with<F, Check>(
    pid: u32,
    command: F,
    is_original_alive: Check,
) -> Result<(), TerminationError>
where
    F: Future<Output = Result<ExitStatus, TerminationError>>,
    Check: Fn() -> bool,
{
    if !is_original_alive() {
        return Ok(());
    }
    let status = command.await?;
    if !status.success() {
        return Err(TerminationError::CommandFailed(status));
    }
    tokio::time::timeout(EXIT_TIMEOUT, async {
        while is_original_alive() {
            tokio::time::sleep(POLL_INTERVAL).await;
        }
    })
    .await
    .map_err(|_| TerminationError::ExitUnconfirmed(pid))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn status(code: u32) -> ExitStatus {
        #[cfg(windows)]
        {
            use std::os::windows::process::ExitStatusExt;
            ExitStatus::from_raw(code)
        }
        #[cfg(unix)]
        {
            use std::os::unix::process::ExitStatusExt;
            ExitStatus::from_raw((code as i32) << 8)
        }
    }

    #[tokio::test]
    async fn command_execution_and_status_failures_are_not_success() {
        let error = terminate_with(
            1,
            async {
                Err(TerminationError::Command(io::Error::from(
                    io::ErrorKind::PermissionDenied,
                )))
            },
            || true,
        )
        .await
        .unwrap_err();
        assert!(matches!(error, TerminationError::Command(_)));
        assert!(error.source().unwrap().is::<io::Error>());
        assert!(matches!(
            terminate_with(1, async { Ok(status(5)) }, || true).await,
            Err(TerminationError::CommandFailed(_))
        ));
    }

    #[tokio::test(start_paused = true)]
    async fn successful_command_requires_confirmed_exit() {
        assert!(matches!(
            terminate_with(1, async { Ok(status(0)) }, || true).await,
            Err(TerminationError::ExitUnconfirmed(1))
        ));
        let calls = AtomicUsize::new(0);
        terminate_with(1, async { Ok(status(0)) }, || {
            calls.fetch_add(1, Ordering::SeqCst) < 2
        })
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn exited_or_replaced_process_does_not_run_the_command() {
        terminate_with(
            1,
            async { panic!("stale process must not be killed") },
            || false,
        )
        .await
        .unwrap();
    }
    #[tokio::test]
    async fn reused_pid_before_termination_never_runs_the_command() {
        let original = super::super::process::ActiveChild {
            pid: 10,
            started_at: 20,
            proc_start: Some(19),
        };
        terminate_identity(
            original,
            async { panic!("PID belongs to a replacement process") },
            |_| Some(30),
        )
        .await
        .unwrap();
        let unknown = super::super::process::ActiveChild {
            proc_start: None,
            ..original
        };
        assert!(matches!(
            terminate_identity(
                unknown,
                async { panic!("identity was never established") },
                |_| Some(19)
            )
            .await,
            Err(TerminationError::IdentityUnavailable(10))
        ));
    }

    struct FailedWait {
        waits: usize,
        signal_failure: bool,
        reap_failure: bool,
        signalled: bool,
    }

    impl TerminationCommand for FailedWait {
        async fn wait(&mut self) -> io::Result<ExitStatus> {
            self.waits += 1;
            if self.waits == 1 {
                return Err(io::Error::from(io::ErrorKind::BrokenPipe));
            }
            if self.reap_failure {
                return Err(io::Error::from(io::ErrorKind::PermissionDenied));
            }
            Ok(status(0))
        }
        fn start_kill(&mut self) -> io::Result<()> {
            self.signalled = true;
            if self.signal_failure {
                return Err(io::Error::from(io::ErrorKind::PermissionDenied));
            }
            Ok(())
        }
    }

    #[tokio::test]
    async fn wait_failure_attempts_kill_and_reap_and_preserves_all_failures() {
        let mut child = FailedWait {
            waits: 0,
            signal_failure: true,
            reap_failure: true,
            signalled: false,
        };
        let failure = wait_for_command(&mut child).await.unwrap_err();
        let TerminationError::CommandCleanup {
            primary,
            signal,
            reap,
        } = failure
        else {
            panic!("missing cleanup failures")
        };
        assert!(
            matches!(*primary, TerminationError::Command(ref source) if source.kind() == io::ErrorKind::BrokenPipe)
        );
        assert_eq!(signal.unwrap().kind(), io::ErrorKind::PermissionDenied);
        assert_eq!(reap.unwrap().kind(), io::ErrorKind::PermissionDenied);
        assert!(child.signalled);
        assert_eq!(child.waits, 2);

        let mut child = FailedWait {
            waits: 0,
            signal_failure: false,
            reap_failure: false,
            signalled: false,
        };
        assert!(
            matches!(wait_for_command(&mut child).await, Err(TerminationError::Command(source)) if source.kind() == io::ErrorKind::BrokenPipe)
        );
        assert!(child.signalled);
        assert_eq!(child.waits, 2);
    }
    struct HungCommand {
        signalled: bool,
    }
    impl TerminationCommand for HungCommand {
        async fn wait(&mut self) -> io::Result<ExitStatus> {
            std::future::pending().await
        }
        fn start_kill(&mut self) -> io::Result<()> {
            self.signalled = true;
            Ok(())
        }
    }

    #[tokio::test(start_paused = true)]
    async fn command_timeout_and_unconfirmed_cleanup_remain_distinct() {
        let mut child = HungCommand { signalled: false };
        let error = wait_for_command(&mut child).await.unwrap_err();
        let TerminationError::CommandCleanup {
            primary,
            signal,
            reap,
        } = error
        else {
            panic!("cleanup must be reported")
        };
        assert!(matches!(*primary, TerminationError::CommandTimeout));
        assert!(signal.is_none());
        assert_eq!(reap.unwrap().kind(), io::ErrorKind::TimedOut);
        assert!(child.signalled);
    }
}
