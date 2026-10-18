use super::discovery::{parse_version, resolve_probe_binary};
use super::JavaError;
use std::{future::Future, io, path::Path, pin::Pin, sync::Arc, time::Duration};
use tokio::io::AsyncReadExt;

const PROBE_TIMEOUT: Duration = Duration::from_secs(5);
const CLEANUP_TIMEOUT: Duration = Duration::from_secs(3);
const MAX_OUTPUT_BYTES: usize = 65_536;

pub(crate) type JavaProbeFuture<'a> =
    Pin<Box<dyn Future<Output = Result<u32, JavaError>> + Send + 'a>>;
type CleanupFuture<'a> = Pin<Box<dyn Future<Output = io::Result<()>> + Send + 'a>>;

pub(crate) trait JavaProbe: Send + Sync {
    fn major_version<'a>(&'a self, binary: &'a Path) -> JavaProbeFuture<'a>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ProbeExit {
    pub(crate) success: bool,
    pub(crate) code: Option<i32>,
}

#[cfg(test)]
impl ProbeExit {
    pub(crate) fn success() -> Self {
        Self {
            success: true,
            code: Some(0),
        }
    }
    pub(crate) fn failure(code: i32) -> Self {
        Self {
            success: false,
            code: Some(code),
        }
    }
}

impl From<std::process::ExitStatus> for ProbeExit {
    fn from(status: std::process::ExitStatus) -> Self {
        Self {
            success: status.success(),
            code: status.code(),
        }
    }
}

pub(crate) trait ProbeChild: Send {
    fn take_stdout(&mut self) -> Option<Box<dyn tokio::io::AsyncRead + Send + Unpin>>;
    fn take_stderr(&mut self) -> Option<Box<dyn tokio::io::AsyncRead + Send + Unpin>>;
    fn wait(&mut self) -> Pin<Box<dyn Future<Output = io::Result<ProbeExit>> + Send + '_>>;
    fn terminate_and_reap(&mut self) -> CleanupFuture<'_>;
}

pub(crate) trait ProcessRunner: Send + Sync {
    fn spawn(&self, command: &Path, args: &[&str]) -> io::Result<Box<dyn ProbeChild>>;
}

struct OsProcessRunner;

impl ProcessRunner for OsProcessRunner {
    fn spawn(&self, command: &Path, args: &[&str]) -> io::Result<Box<dyn ProbeChild>> {
        let mut command = tokio::process::Command::new(command);
        command.args(args).stdin(std::process::Stdio::null());
        command
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());
        command.kill_on_drop(true);
        #[cfg(windows)]
        command.creation_flags(0x08000000);
        Ok(Box::new(OsProbeChild {
            child: command.spawn()?,
            reaped: false,
        }))
    }
}

struct OsProbeChild {
    child: tokio::process::Child,
    reaped: bool,
}

impl ProbeChild for OsProbeChild {
    fn take_stdout(&mut self) -> Option<Box<dyn tokio::io::AsyncRead + Send + Unpin>> {
        self.child.stdout.take().map(|stream| Box::new(stream) as _)
    }

    fn take_stderr(&mut self) -> Option<Box<dyn tokio::io::AsyncRead + Send + Unpin>> {
        self.child.stderr.take().map(|stream| Box::new(stream) as _)
    }

    fn wait(&mut self) -> Pin<Box<dyn Future<Output = io::Result<ProbeExit>> + Send + '_>> {
        Box::pin(async move {
            let status = self.child.wait().await?;
            self.reaped = true;
            Ok(status.into())
        })
    }

    fn terminate_and_reap(&mut self) -> CleanupFuture<'_> {
        Box::pin(async move {
            if self.reaped {
                return Ok(());
            }
            if self.child.try_wait()?.is_some() {
                self.reaped = true;
                return Ok(());
            }
            self.child.start_kill()?;
            self.child.wait().await?;
            self.reaped = true;
            Ok(())
        })
    }
}

pub(crate) struct NativeProbe {
    timeout: Duration,
    max_output_bytes: usize,
    runner: Arc<dyn ProcessRunner>,
}

impl NativeProbe {
    pub(crate) fn new() -> Self {
        Self {
            timeout: PROBE_TIMEOUT,
            max_output_bytes: MAX_OUTPUT_BYTES,
            runner: Arc::new(OsProcessRunner),
        }
    }

    #[cfg(test)]
    pub(crate) fn with_runner(runner: Arc<dyn ProcessRunner>) -> Self {
        Self {
            runner,
            ..Self::new()
        }
    }

    #[cfg(test)]
    pub(crate) fn with_options(
        timeout: Duration,
        max_output_bytes: usize,
        runner: Arc<dyn ProcessRunner>,
    ) -> Self {
        Self {
            timeout,
            max_output_bytes,
            runner,
        }
    }
}

impl JavaProbe for NativeProbe {
    fn major_version<'a>(&'a self, binary: &'a Path) -> JavaProbeFuture<'a> {
        Box::pin(async move {
            let binary = resolve_probe_binary(binary);
            let child = self
                .runner
                .spawn(&binary, &["-version"])
                .map_err(JavaError::ProbeSpawn)?;
            let (cancellation, cancelled) = tokio::sync::oneshot::channel();
            // The worker retains process ownership while a dropped caller signals
            // cancellation. Cleanup must outlive cancellation of the request.
            let worker = tokio::spawn(run_probe(
                child,
                cancelled,
                self.timeout,
                self.max_output_bytes,
            ));
            let result = worker.await?;
            drop(cancellation);
            result
        })
    }
}

async fn read_bounded(
    reader: Option<Box<dyn tokio::io::AsyncRead + Send + Unpin>>,
    limit: usize,
) -> io::Result<Vec<u8>> {
    let Some(mut reader) = reader else {
        return Ok(Vec::new());
    };
    let mut output = Vec::new();
    let mut chunk = [0; 4096];
    loop {
        let count = reader.read(&mut chunk).await?;
        if count == 0 {
            return Ok(output);
        }
        let retained = count.min(limit.saturating_sub(output.len()));
        output.extend_from_slice(&chunk[..retained]);
    }
}

async fn run_probe(
    mut child: Box<dyn ProbeChild>,
    cancelled: tokio::sync::oneshot::Receiver<()>,
    timeout: Duration,
    output_limit: usize,
) -> Result<u32, JavaError> {
    let stdout = child.take_stdout();
    let stderr = child.take_stderr();
    let execute = async {
        let (stdout, stderr) = tokio::try_join!(
            read_bounded(stdout, output_limit),
            read_bounded(stderr, output_limit)
        )?;
        let status = child.wait().await?;
        Ok::<_, io::Error>((stdout, stderr, status))
    };
    let result = tokio::select! {
        _ = cancelled => Err(JavaError::ProbeCancelled),
        result = tokio::time::timeout(timeout, execute) => match result {
            Ok(result) => result.map_err(JavaError::Io),
            Err(_) => Err(JavaError::ProbeTimeout(timeout)),
        }
    };
    let (stdout, stderr, status) = match result {
        Ok(output) => output,
        Err(primary) => {
            let cancelled = matches!(primary, JavaError::ProbeCancelled);
            let error = cleanup_failure(child.as_mut(), primary).await;
            if cancelled && matches!(error, JavaError::ProbeCleanup { .. }) {
                eprintln!("[Java] Cancelled probe cleanup failed: {error}");
            }
            return Err(error);
        }
    };
    if !status.success {
        return Err(JavaError::ProbeFailed);
    }
    let stderr = String::from_utf8_lossy(&stderr);
    let stdout = String::from_utf8_lossy(&stdout);
    parse_version(&stderr)
        .or_else(|| parse_version(&stdout))
        .ok_or(JavaError::InvalidVersion)
}

async fn cleanup_failure(child: &mut dyn ProbeChild, primary: JavaError) -> JavaError {
    let cleanup = match tokio::time::timeout(CLEANUP_TIMEOUT, child.terminate_and_reap()).await {
        Ok(result) => result,
        Err(_) => Err(io::Error::new(
            io::ErrorKind::TimedOut,
            "Java probe cleanup timed out",
        )),
    };
    match cleanup {
        Ok(()) => primary,
        Err(cleanup) => JavaError::ProbeCleanup {
            primary: Some(Box::new(primary)),
            cleanup,
        },
    }
}

#[cfg(test)]
mod native_tests {
    use super::*;
    use std::io::Write;
    use tokio::io::AsyncBufReadExt;

    #[test]
    #[ignore = "subprocess fixture invoked by native_probe_cleanup_confirms_exit"]
    fn probe_fixture() {
        println!("probe fixture ready");
        std::io::stdout().flush().unwrap();
        std::thread::park();
    }

    #[tokio::test]
    async fn native_probe_cleanup_confirms_exit() {
        let mut command = tokio::process::Command::new(std::env::current_exe().unwrap());
        command.args([
            "--ignored",
            "--exact",
            "minecraft::java::probe::native_tests::probe_fixture",
            "--nocapture",
        ]);
        command
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .kill_on_drop(true);
        #[cfg(windows)]
        command.creation_flags(0x08000000);
        let mut child = OsProbeChild {
            child: command.spawn().unwrap(),
            reaped: false,
        };
        let stdout = child.child.stdout.take().unwrap();
        let mut lines = tokio::io::BufReader::new(stdout).lines();
        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                let line = lines.next_line().await.unwrap().unwrap();
                if line == "probe fixture ready" {
                    break;
                }
            }
        })
        .await
        .unwrap();
        assert!(child.child.try_wait().unwrap().is_none());
        tokio::time::timeout(CLEANUP_TIMEOUT, child.terminate_and_reap())
            .await
            .unwrap()
            .unwrap();
        assert!(child.reaped);
        assert!(child.child.try_wait().unwrap().is_some());
    }
}
