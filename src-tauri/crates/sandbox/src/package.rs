use super::SandboxError;
use std::io;
use std::path::Path;
use std::process::Stdio;
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncReadExt};
use tokio::process::Command;

const OUTPUT_LIMIT: usize = 64 * 1024;
const INSTALL_TIMEOUT: Duration = Duration::from_secs(15 * 60);
const CLEANUP_TIMEOUT: Duration = Duration::from_secs(3);

pub(super) async fn install_java(
    sandbox: &Path,
    rootfs: &Path,
    package: &str,
) -> Result<(), SandboxError> {
    let mut command = super::proot::proot_command(sandbox, Some(rootfs), true, &[], "/root")?;
    command.args(["/sbin/apk", "add", "--no-cache", "--no-progress", package]);
    execute(command, package, INSTALL_TIMEOUT).await
}

async fn execute(
    mut command: Command,
    package: &str,
    deadline: Duration,
) -> Result<(), SandboxError> {
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    command.kill_on_drop(true);
    #[cfg(windows)]
    command.creation_flags(0x08000000);
    let mut child = command.spawn()?;
    execute_child(&mut child, package, deadline).await
}

async fn execute_child(
    child: &mut tokio::process::Child,
    package: &str,
    deadline: Duration,
) -> Result<(), SandboxError> {
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let execution = async { tokio::try_join!(child.wait(), read_tail(stdout), read_tail(stderr)) };
    let primary = match tokio::time::timeout(deadline, execution).await {
        Ok(Ok((status, stdout, stderr))) => {
            if status.success() {
                return Ok(());
            }
            let details = format!(
                "{}{}",
                String::from_utf8_lossy(&stdout),
                String::from_utf8_lossy(&stderr)
            );
            return Err(SandboxError::JavaInstallFailed {
                package: package.to_string(),
                status,
                details,
            });
        }
        Ok(Err(error)) => SandboxError::Io(error),
        Err(_) => SandboxError::ExecutionTimeout,
    };
    let cleanup = tokio::time::timeout(CLEANUP_TIMEOUT, child.kill()).await;
    match cleanup {
        Ok(Ok(())) => Err(primary),
        Ok(Err(_)) if matches!(child.try_wait(), Ok(Some(_))) => Err(primary),
        Ok(Err(source)) => Err(SandboxError::ProcessCleanup {
            primary: Box::new(primary),
            source,
        }),
        Err(_) => Err(SandboxError::ProcessCleanup {
            primary: Box::new(primary),
            source: io::Error::new(
                io::ErrorKind::TimedOut,
                "sandbox package process did not terminate",
            ),
        }),
    }
}

async fn read_tail(reader: Option<impl AsyncRead + Unpin>) -> io::Result<Vec<u8>> {
    let Some(mut reader) = reader else {
        return Ok(Vec::new());
    };
    let mut tail = Vec::new();
    let mut chunk = [0u8; 8192];
    loop {
        let count = reader.read(&mut chunk).await?;
        if count == 0 {
            return Ok(tail);
        }
        let overflow = tail
            .len()
            .saturating_add(count)
            .saturating_sub(OUTPUT_LIMIT);
        tail.drain(..overflow);
        tail.extend_from_slice(&chunk[..count]);
    }
}

#[cfg(test)]
mod tests;
