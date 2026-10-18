use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::{oneshot, Mutex};
use tokio::task::JoinHandle;

use super::adapter::PlayitChild;
use super::PlayitError;

pub(crate) struct PlayitProcess {
    pub(crate) child: Arc<Mutex<Box<dyn PlayitChild>>>,
    pub(crate) stdout_task: Option<JoinHandle<()>>,
    pub(crate) stderr_task: Option<JoinHandle<()>>,
    pub(crate) exit_task: Option<JoinHandle<()>>,
    pub(crate) last_log: Arc<Mutex<String>>,
}

impl PlayitProcess {
    pub(crate) fn new(
        mut child: Box<dyn PlayitChild>,
    ) -> (
        Self,
        oneshot::Receiver<Result<std::process::ExitStatus, std::io::Error>>,
    ) {
        let last_log = Arc::new(Mutex::new(String::new()));
        let stdout_task = child
            .take_stdout()
            .map(|stream| tokio::spawn(forward_logs(stream, last_log.clone())));
        let stderr_task = child
            .take_stderr()
            .map(|stream| tokio::spawn(forward_logs(stream, last_log.clone())));

        let child_arc = Arc::new(Mutex::new(child));
        let child_for_wait = child_arc.clone();
        let (exit_tx, exit_rx) = oneshot::channel();
        let exit_task = tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_millis(50));
            loop {
                interval.tick().await;
                let result = child_for_wait.lock().await.try_wait();
                match result {
                    Ok(None) => continue,
                    Ok(Some(status)) => {
                        let _ = exit_tx.send(Ok(status));
                    }
                    Err(error) => {
                        let _ = exit_tx.send(Err(error));
                    }
                }
                break;
            }
        });

        (
            Self {
                child: child_arc,
                stdout_task,
                stderr_task,
                exit_task: Some(exit_task),
                last_log,
            },
            exit_rx,
        )
    }

    pub(crate) async fn terminate_and_reap(&mut self) -> Result<(), PlayitError> {
        if let Some(task) = self.exit_task.take() {
            task.abort();
            let _ = task.await;
        }
        if let Some(task) = self.stdout_task.take() {
            task.abort();
            let _ = task.await;
        }
        if let Some(task) = self.stderr_task.take() {
            task.abort();
            let _ = task.await;
        }

        let terminate = async {
            let mut child = self.child.lock().await;
            if child.try_wait()?.is_some() {
                return Ok(());
            }
            if let Err(error) = child.kill().await {
                if child.try_wait()?.is_some() {
                    return Ok(());
                }
                return Err(error);
            }
            child.wait().await?;
            Ok(())
        };
        let wait_result = tokio::time::timeout(Duration::from_secs(3), terminate).await;
        match wait_result {
            Ok(Ok(())) => Ok(()),
            Ok(Err(err)) => Err(PlayitError::TerminationFailed(err)),
            Err(_) => Err(PlayitError::TerminationFailed(std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                "playit process termination timed out",
            ))),
        }
    }
}

impl Drop for PlayitProcess {
    fn drop(&mut self) {
        for task in [&self.stdout_task, &self.stderr_task, &self.exit_task]
            .into_iter()
            .flatten()
        {
            task.abort();
        }
    }
}

pub(crate) fn build_agent_command(
    binary_path: &Path,
    secret: &str,
    playit_dir: &Path,
    sandbox_dir: Option<&Path>,
) -> Result<Command, PlayitError> {
    let mut cmd = if let Some(sandbox_dir) = sandbox_dir {
        let resolv = playit_dir.join("resolv.conf");
        std::fs::write(&resolv, "nameserver 1.1.1.1\nnameserver 8.8.8.8\n")?;
        let mut cmd = crate::server::sandbox::proot_command(
            sandbox_dir,
            None,
            false,
            &[(resolv, "/etc/resolv.conf")],
            &playit_dir.to_string_lossy(),
        )?;
        cmd.env("HOME", playit_dir);
        cmd.env("XDG_CONFIG_HOME", playit_dir);
        cmd.arg(binary_path);
        cmd
    } else {
        Command::new(binary_path)
    };

    cmd.current_dir(playit_dir);
    cmd.arg("--secret").arg(secret);

    #[cfg(unix)]
    {
        let socket = playit_dir.join("playit.sock");
        match std::fs::remove_file(&socket) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(PlayitError::Io(error)),
        }
        cmd.arg("--socket-path").arg(socket);
    }

    Ok(cmd)
}

async fn forward_logs(
    stream: Box<dyn tokio::io::AsyncRead + Unpin + Send>,
    last_log: Arc<Mutex<String>>,
) {
    let mut reader = BufReader::new(stream).lines();
    while let Ok(Some(line)) = reader.next_line().await {
        let trimmed = line.trim();
        if !trimmed.is_empty() {
            let mut guard = last_log.lock().await;
            *guard = trimmed.to_string();
        }
    }
}
