use super::{ProcessIdentity, TerminationError};
use std::future::Future;
use std::io;
use std::process::{ExitStatus, Stdio};
use std::time::Duration;

const COMMAND_TIMEOUT: Duration = Duration::from_secs(10);
pub(super) const CLEANUP_TIMEOUT: Duration = Duration::from_secs(3);

pub(super) async fn run(
    identity: ProcessIdentity,
    deadline: tokio::time::Instant,
) -> Result<(), TerminationError> {
    run_command(native_command(identity.pid()), identity.pid(), deadline).await
}

pub(super) async fn run_command(
    mut command: tokio::process::Command,
    pid: u32,
    deadline: tokio::time::Instant,
) -> Result<(), TerminationError> {
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .kill_on_drop(true);
    let mut child = command
        .spawn()
        .map_err(|source| TerminationError::Spawn { pid, source })?;
    wait(&mut child, pid, deadline).await
}

fn native_command(pid: u32) -> tokio::process::Command {
    #[cfg(windows)]
    {
        let mut command = tokio::process::Command::new("taskkill");
        command.args(["/F", "/T", "/PID", &pid.to_string()]);
        command.creation_flags(0x08000000);
        command
    }
    #[cfg(not(windows))]
    {
        let mut command = tokio::process::Command::new("kill");
        command.args(["-9", &pid.to_string()]);
        command
    }
}

pub(super) trait Command {
    fn wait(&mut self) -> impl Future<Output = io::Result<ExitStatus>> + Send;
    fn start_kill(&mut self) -> io::Result<()>;
}

impl Command for tokio::process::Child {
    async fn wait(&mut self) -> io::Result<ExitStatus> {
        tokio::process::Child::wait(self).await
    }
    fn start_kill(&mut self) -> io::Result<()> {
        tokio::process::Child::start_kill(self)
    }
}

pub(super) async fn wait(
    child: &mut impl Command,
    pid: u32,
    deadline: tokio::time::Instant,
) -> Result<(), TerminationError> {
    let command_deadline = deadline.min(tokio::time::Instant::now() + COMMAND_TIMEOUT);
    let primary = match tokio::time::timeout_at(command_deadline, child.wait()).await {
        Ok(Ok(status)) if status.success() => return Ok(()),
        Ok(Ok(status)) => return Err(TerminationError::CommandFailed { pid, status }),
        Ok(Err(source)) => TerminationError::Wait { pid, source },
        Err(_) => TerminationError::CommandTimeout(pid),
    };
    let signal = child.start_kill().err();
    let cleanup_deadline =
        (tokio::time::Instant::now() + CLEANUP_TIMEOUT).min(command_deadline + CLEANUP_TIMEOUT);
    let reap = match tokio::time::timeout_at(cleanup_deadline, child.wait()).await {
        Ok(result) => result.err(),
        Err(_) => Some(io::Error::new(
            io::ErrorKind::TimedOut,
            "termination command was not reaped before cleanup deadline",
        )),
    };
    if signal.is_some() || reap.is_some() {
        return Err(TerminationError::Cleanup {
            primary: Box::new(primary),
            signal,
            reap,
        });
    }
    Err(primary)
}
