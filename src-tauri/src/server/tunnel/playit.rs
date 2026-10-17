pub use crate::server::config::PlayitTunnelStatus;

pub(crate) mod api;
pub(crate) mod binary;
#[cfg(test)]
mod tests;

use api::{api_call, apply_rundata, claim_agent, AgentContext};
use serde_json::json;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;
use tauri::{Manager, Runtime};
use tokio::io::{AsyncBufReadExt, AsyncRead, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

pub(crate) const PLAYIT_API: &str = "https://api.playit.gg";
const SECRET_FILE: &str = "secret.key";

/// Manages the playit.gg agent.
///
/// playit v1.x (`playitd`) no longer prints a claim link: without a secret it waits
/// for a GUI frontend over IPC. So Ingot performs the claim flow itself through the
/// playit API, stores the secret, runs the daemon with `--secret`, and reads the
/// tunnel address back from the API.
#[derive(Clone, Default)]
pub struct PlayitManager {
    inner: Arc<Mutex<PlayitInner>>,
}

pub(crate) struct PlayitInner {
    pub(crate) status: PlayitTunnelStatus,
    pub(crate) child: Option<Arc<Mutex<Child>>>,
    pub(crate) task: Option<JoinHandle<()>>,
}

fn stopped_status() -> PlayitTunnelStatus {
    PlayitTunnelStatus {
        is_running: false,
        status: "stopped".to_string(),
        claim_url: None,
        public_address: None,
        ping_ms: None,
        message: None,
    }
}

impl Default for PlayitInner {
    fn default() -> Self {
        Self {
            status: stopped_status(),
            child: None,
            task: None,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum PlayitError {
    #[error("Playit network request failed")]
    Network(#[source] reqwest::Error),
    #[error("Playit provider returned HTTP {0}")]
    HttpStatus(reqwest::StatusCode),
    #[error("Playit provider returned an invalid response")]
    InvalidResponse,
    #[error("Playit provider rejected the request: {0}")]
    Rejected(ClaimRejection),
    #[error("I/O error during playit tunnel operation: {0}")]
    Io(#[from] std::io::Error),
    #[error("Sandbox error: {0}")]
    Sandbox(#[from] crate::server::sandbox::SandboxError),
    #[error("Playit claim was rejected")]
    ClaimRejected,
    #[error("Playit claim expired; connect again to request a new claim")]
    ClaimExpired,
    #[error("Timed out completing the Playit claim")]
    ClaimTimeout,
    #[error("The saved Playit credential is invalid; connect again to link this device")]
    InvalidCredential,
    #[error("Playit agent exited ({0})")]
    AgentExited(std::process::ExitStatus),
    #[error("Playit application directory is unavailable")]
    AppData(#[source] tauri::Error),
}

#[derive(Debug, thiserror::Error)]
pub enum ClaimRejection {
    #[error("claim expired")]
    Expired,
    #[error("claim is invalid")]
    InvalidCode,
    #[error("claim has not yet been accepted")]
    NotAccepted,
    #[error("provider rejection")]
    Other,
}

impl PlayitManager {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(PlayitInner::default())),
        }
    }

    pub async fn get_status(&self) -> PlayitTunnelStatus {
        let guard = self.inner.lock().await;
        guard.status.clone()
    }

    /// Finds or downloads the playit agent, the first time someone goes public.
    /// On Android it isn't bundled: it runs through PRoot (see `build_agent_command`),
    /// which can start it from app storage where Android won't run programs itself.
    pub async fn ensure_playit_binary<R: Runtime>(
        &self,
        app: &tauri::AppHandle<R>,
        client: &reqwest::Client,
    ) -> Result<PathBuf, PlayitError> {
        binary::ensure_playit_binary(app, client).await
    }

    /// Starts the playit tunnel agent. Claiming and connecting continue in the
    /// background; the UI follows progress through `get_status`.
    pub async fn start_tunnel<R: Runtime>(
        &self,
        app: &tauri::AppHandle<R>,
        client: &reqwest::Client,
        secret_key: Option<String>,
    ) -> Result<PlayitTunnelStatus, PlayitError> {
        // Stop existing instance if running
        let _ = self.stop_tunnel().await;

        let binary_path = self.ensure_playit_binary(app, client).await?;

        let data_dir = app.path().app_data_dir().map_err(PlayitError::AppData)?;
        let playit_dir = data_dir.join("playit");
        std::fs::create_dir_all(&playit_dir)?;

        let sandbox_dir = if cfg!(target_os = "android") {
            Some(crate::server::sandbox::get_sandbox_dir(app)?)
        } else {
            None
        };

        let secret = secret_key
            .filter(|s| !s.trim().is_empty())
            .or_else(|| read_saved_secret(&playit_dir));

        let ctx = AgentContext {
            inner: self.inner.clone(),
            client: client.clone(),
            binary_path,
            playit_dir,
            sandbox_dir,
        };

        {
            let mut guard = self.inner.lock().await;
            guard.status = PlayitTunnelStatus {
                is_running: true,
                status: "starting".to_string(),
                ..stopped_status()
            };
            guard.task = Some(tokio::spawn(run_agent(ctx, secret)));
        }

        // Wait a short moment so an immediate claim URL is included in the response
        tokio::time::sleep(Duration::from_millis(800)).await;
        Ok(self.get_status().await)
    }

    /// Stops the running playit tunnel
    pub async fn stop_tunnel(&self) -> Result<(), PlayitError> {
        let (task, child) = {
            let mut guard = self.inner.lock().await;
            guard.status = stopped_status();
            (guard.task.take(), guard.child.take())
        };

        if let Some(task) = task {
            task.abort();
        }
        if let Some(child_arc) = child {
            let mut child = child_arc.lock().await;
            let _ = child.kill().await;
        }

        Ok(())
    }
}

fn read_saved_secret(playit_dir: &Path) -> Option<String> {
    std::fs::read_to_string(playit_dir.join(SECRET_FILE))
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// Background task: claim (if needed), launch the daemon, and track the tunnel address
async fn run_agent(ctx: AgentContext, secret: Option<String>) {
    if let Err(err) = run_agent_inner(&ctx, secret).await {
        let message = err.to_string();
        eprintln!("[Playit] {message}");
        let child = {
            let mut guard = ctx.inner.lock().await;
            guard.status = PlayitTunnelStatus {
                status: "error".to_string(),
                message: Some(message),
                ..stopped_status()
            };
            guard.child.take()
        };
        if let Some(child_arc) = child {
            let _ = child_arc.lock().await.kill().await;
        }
    }
}

async fn run_agent_inner(ctx: &AgentContext, secret: Option<String>) -> Result<(), PlayitError> {
    let secret = match secret {
        Some(s) => s,
        None => {
            let s = claim_agent(ctx).await?;
            let _ = std::fs::write(ctx.playit_dir.join(SECRET_FILE), &s);
            s
        }
    };

    ctx.update(|s| {
        s.status = "connecting".to_string();
        s.claim_url = None;
    })
    .await;

    let mut cmd = build_agent_command(ctx, &secret)?;
    cmd.stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);

    #[cfg(windows)]
    {
        // CREATE_NO_WINDOW flag so a console window does not pop up
        cmd.creation_flags(0x08000000);
    }

    let mut child = cmd.spawn()?;

    let last_log = Arc::new(Mutex::new(String::new()));
    if let Some(stdout) = child.stdout.take() {
        tokio::spawn(forward_logs(stdout, last_log.clone()));
    }
    if let Some(stderr) = child.stderr.take() {
        tokio::spawn(forward_logs(stderr, last_log.clone()));
    }

    let child_arc = Arc::new(Mutex::new(child));
    ctx.inner.lock().await.child = Some(child_arc.clone());

    loop {
        if let Ok(Some(exit)) = child_arc.lock().await.try_wait() {
            let last = last_log.lock().await.clone();
            if last.contains("InvalidAgentKey") || last.contains("no longer valid") {
                let _ = std::fs::remove_file(ctx.playit_dir.join(SECRET_FILE));
                return Err(PlayitError::InvalidCredential);
            }
            return Err(PlayitError::AgentExited(exit));
        }

        match api_call(&ctx.client, "/v1/agents/rundata", json!({}), Some(&secret)).await {
            Ok(data) => apply_rundata(ctx, &data).await,
            Err(e) => eprintln!("[Playit] Failed to fetch tunnel info: {e}"),
        }

        tokio::time::sleep(Duration::from_secs(5)).await;
    }
}

/// Builds the daemon command. On Android, playit (a static musl binary) resolves DNS
/// via `/etc/resolv.conf`, which Android doesn't have, so it runs under PRoot with
/// a resolv.conf bound in. PRoot also runs it from app storage, where Android won't
/// execute downloaded programs directly.
fn build_agent_command(ctx: &AgentContext, secret: &str) -> Result<Command, PlayitError> {
    let mut cmd = if let Some(ref sandbox_dir) = ctx.sandbox_dir {
        let resolv = ctx.playit_dir.join("resolv.conf");
        std::fs::write(&resolv, "nameserver 1.1.1.1\nnameserver 8.8.8.8\n")?;
        let mut cmd = crate::server::sandbox::proot_command(
            sandbox_dir,
            None,
            false,
            &[(resolv, "/etc/resolv.conf")],
            &ctx.playit_dir.to_string_lossy(),
        )?;
        cmd.env("HOME", &ctx.playit_dir);
        cmd.env("XDG_CONFIG_HOME", &ctx.playit_dir);
        cmd.arg(&ctx.binary_path);
        cmd
    } else {
        Command::new(&ctx.binary_path)
    };

    cmd.current_dir(&ctx.playit_dir);
    cmd.arg("--secret").arg(secret);

    // The default IPC socket lives in a system dir (e.g. /run) that apps can't write to
    #[cfg(unix)]
    {
        let socket = ctx.playit_dir.join("playit.sock");
        let _ = std::fs::remove_file(&socket);
        cmd.arg("--socket-path").arg(socket);
    }

    Ok(cmd)
}

async fn forward_logs<T: AsyncRead + Unpin>(stream: T, last_log: Arc<Mutex<String>>) {
    let mut reader = BufReader::new(stream).lines();
    while let Ok(Some(line)) = reader.next_line().await {
        if !line.trim().is_empty() {
            *last_log.lock().await = line;
        }
    }
}
