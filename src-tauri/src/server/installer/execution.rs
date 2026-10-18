use std::io;
use std::path::Path;
use std::process::Stdio;
use std::time::Duration;

use tokio::io::{AsyncRead, AsyncReadExt};
use tokio::process::Command;

use super::{InstallerError, ServerCoreType};

const OUTPUT_LIMIT_BYTES: usize = 64 * 1024;
const INSTALLER_TIMEOUT: Duration = Duration::from_secs(15 * 60);
const TERMINATION_TIMEOUT: Duration = Duration::from_secs(3);

pub(super) async fn run_downloaded_installer<E>(
    server_dir: &Path,
    core: &ServerCoreType,
    installer: &str,
    arguments: Vec<String>,
    java: impl Fn(&[String]) -> Result<Command, E>,
) -> Result<(), InstallerError>
where
    E: std::error::Error + Send + Sync + 'static,
{
    run_with_timeout(
        server_dir,
        core,
        installer,
        arguments,
        java,
        INSTALLER_TIMEOUT,
    )
    .await
}

pub(super) async fn run_with_timeout<E>(
    server_dir: &Path,
    core: &ServerCoreType,
    installer: &str,
    arguments: Vec<String>,
    java: impl Fn(&[String]) -> Result<Command, E>,
    timeout: Duration,
) -> Result<(), InstallerError>
where
    E: std::error::Error + Send + Sync + 'static,
{
    let mut java_arguments = vec!["-jar".to_string(), installer.to_string()];
    java_arguments.extend(arguments);
    let result = match java(&java_arguments) {
        Ok(command) => execute(command, core, timeout).await,
        Err(error) => Err(process_error(core, error)),
    };
    let cleanup = cleanup_artifacts(server_dir, installer);
    match (result, cleanup) {
        (result, Ok(())) => result,
        (result, Err(source)) => Err(InstallerError::Cleanup {
            primary: result.err().map(Box::new),
            source,
        }),
    }
}

async fn execute(
    mut command: Command,
    core: &ServerCoreType,
    timeout: Duration,
) -> Result<(), InstallerError> {
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    // Tokio requests termination and arranges reaping if the caller drops the
    // future. Output readers run in this future, so cancellation detaches no tasks.
    command.kill_on_drop(true);
    #[cfg(windows)]
    command.creation_flags(0x08000000);
    let mut child = command
        .spawn()
        .map_err(|error| process_error(core, error))?;
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let execution = async { tokio::try_join!(child.wait(), read_tail(stdout), read_tail(stderr)) };
    let outcome = tokio::time::timeout(timeout, execution).await;
    let primary = match outcome {
        Ok(Ok((status, stdout, stderr))) => {
            if status.success() {
                return Ok(());
            }
            let text = format!(
                "{}{}",
                String::from_utf8_lossy(&stdout),
                String::from_utf8_lossy(&stderr)
            );
            let lines: Vec<&str> = text
                .lines()
                .filter(|line| !line.trim().is_empty())
                .collect();
            let details = lines[lines.len().saturating_sub(8)..].join("\n");
            return Err(InstallerError::InstallationFailed {
                core: core.clone(),
                details,
            });
        }
        Ok(Err(error)) => process_error(core, error),
        Err(_) => InstallerError::ExecutionTimeout { core: core.clone() },
    };
    match tokio::time::timeout(TERMINATION_TIMEOUT, child.kill()).await {
        Ok(Ok(())) => Err(primary),
        Ok(Err(source)) => {
            if matches!(child.try_wait(), Ok(Some(_))) {
                return Err(primary);
            }
            Err(InstallerError::Cleanup {
                primary: Some(Box::new(primary)),
                source,
            })
        }
        Err(_) => Err(InstallerError::Cleanup {
            primary: Some(Box::new(primary)),
            source: io::Error::new(io::ErrorKind::TimedOut, "installer termination timed out"),
        }),
    }
}

fn process_error(
    core: &ServerCoreType,
    source: impl std::error::Error + Send + Sync + 'static,
) -> InstallerError {
    InstallerError::Process {
        core: core.clone(),
        source: Box::new(source),
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
            .saturating_sub(OUTPUT_LIMIT_BYTES);
        if overflow > 0 {
            tail.drain(..overflow);
        }
        tail.extend_from_slice(&chunk[..count]);
    }
}

fn cleanup_artifacts(server_dir: &Path, installer: &str) -> io::Result<()> {
    let mut failures = Vec::new();
    for name in [installer.to_string(), format!("{installer}.log")] {
        match std::fs::remove_file(server_dir.join(&name)) {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => failures.push((name, error)),
        }
    }
    if failures.is_empty() {
        return Ok(());
    }
    Err(io::Error::other(ArtifactCleanupFailures(failures)))
}

#[derive(Debug)]
struct ArtifactCleanupFailures(Vec<(String, io::Error)>);

impl std::fmt::Display for ArtifactCleanupFailures {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (index, (name, error)) in self.0.iter().enumerate() {
            if index > 0 {
                write!(formatter, "; ")?;
            }
            write!(formatter, "{name}: {error}")?;
        }
        Ok(())
    }
}

impl std::error::Error for ArtifactCleanupFailures {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.0
            .first()
            .map(|(_, error)| error as &dyn std::error::Error)
    }
}

#[cfg(test)]
mod tests;
