pub use crate::server::config::PlayitTunnelStatus;
pub use generation::Generation;

pub(crate) mod adapter;
pub(crate) mod api;
pub(crate) mod binary;
pub(crate) mod credentials;
pub(crate) mod generation;
pub(crate) mod process;
#[cfg(test)]
mod tests;

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tauri::{Manager, Runtime};
use tokio::sync::{watch, Mutex};
use tokio::task::JoinHandle;

use adapter::{NativePlayitAdapter, PlayitAdapter};
use api::{apply_rundata, claim_agent, AgentContext};
use generation::StartingGuard;
use process::PlayitProcess;

pub(crate) const PLAYIT_API: &str = "https://api.playit.gg";

/// Manages the playit.gg agent lifecycle with generation identity and explicit ownership.
#[derive(Clone)]
pub struct PlayitManager {
    inner: Arc<Mutex<PlayitInner>>,
    lifecycle_lock: Arc<Mutex<()>>,
    status_tx: Arc<watch::Sender<PlayitTunnelStatus>>,
    status_rx: watch::Receiver<PlayitTunnelStatus>,
    adapter: Arc<dyn PlayitAdapter>,
}

pub(crate) struct PlayitInner {
    pub(crate) generation: u64,
    pub(crate) status: PlayitTunnelStatus,
    pub(crate) process: Option<PlayitProcess>,
    pub(crate) task: Option<JoinHandle<()>>,
    pub(crate) cancel_tx: Option<watch::Sender<bool>>,
}

pub(crate) fn stopped_status() -> PlayitTunnelStatus {
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
            generation: 0,
            status: stopped_status(),
            process: None,
            task: None,
            cancel_tx: None,
        }
    }
}

impl Default for PlayitManager {
    fn default() -> Self {
        Self::new()
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
    #[error("Playit tunnel operation was cancelled")]
    Cancelled,
    #[error("Failed to read saved Playit secret: {0}")]
    CredentialRead(#[source] std::io::Error),
    #[error("Failed to write Playit secret: {0}")]
    CredentialWrite(#[source] std::io::Error),
    #[error("Failed to delete Playit secret: {0}")]
    CredentialDelete(#[source] std::io::Error),
    #[error("Failed to terminate Playit process: {0}")]
    TerminationFailed(#[source] std::io::Error),
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
        let (status_tx, status_rx) = watch::channel(stopped_status());
        Self {
            inner: Arc::new(Mutex::new(PlayitInner::default())),
            lifecycle_lock: Arc::new(Mutex::new(())),
            status_tx: Arc::new(status_tx),
            status_rx,
            adapter: Arc::new(NativePlayitAdapter),
        }
    }

    #[cfg(test)]
    pub(crate) fn with_adapter(adapter: Arc<dyn PlayitAdapter>) -> Self {
        let (status_tx, status_rx) = watch::channel(stopped_status());
        Self {
            inner: Arc::new(Mutex::new(PlayitInner::default())),
            lifecycle_lock: Arc::new(Mutex::new(())),
            status_tx: Arc::new(status_tx),
            status_rx,
            adapter,
        }
    }

    pub async fn get_status(&self) -> PlayitTunnelStatus {
        let guard = self.inner.lock().await;
        guard.status.clone()
    }

    /// Finds or downloads the playit agent, the first time someone goes public.
    pub async fn ensure_playit_binary<R: Runtime>(
        &self,
        app: &tauri::AppHandle<R>,
        client: &reqwest::Client,
    ) -> Result<PathBuf, PlayitError> {
        binary::ensure_playit_binary(app, client).await
    }

    /// Starts the playit tunnel agent.
    pub async fn start_tunnel<R: Runtime>(
        &self,
        app: &tauri::AppHandle<R>,
        client: &reqwest::Client,
        secret_key: Option<String>,
    ) -> Result<PlayitTunnelStatus, PlayitError> {
        let data_dir = app.path().app_data_dir().map_err(PlayitError::AppData)?;
        let playit_dir = data_dir.join("playit");
        let sandbox_dir = if cfg!(target_os = "android") {
            Some(crate::server::sandbox::get_sandbox_dir(app)?)
        } else {
            None
        };

        self.start_tunnel_internal(
            client,
            secret_key,
            &data_dir,
            &playit_dir,
            sandbox_dir.as_deref(),
        )
        .await
    }

    pub(crate) async fn start_tunnel_internal(
        &self,
        client: &reqwest::Client,
        secret_key: Option<String>,
        data_dir: &Path,
        playit_dir: &Path,
        sandbox_dir: Option<&Path>,
    ) -> Result<PlayitTunnelStatus, PlayitError> {
        // Serialize transition under lifecycle_lock without holding it across async I/O
        let (generation, cancel_rx) = {
            let _transition_guard = self.lifecycle_lock.lock().await;

            let mut guard = self.inner.lock().await;
            if let Some(tx) = guard.cancel_tx.take() {
                let _ = tx.send(true);
            }
            if let Some(task) = guard.task.take() {
                task.abort();
            }
            cleanup_process(&mut guard, &self.status_tx).await?;
            guard.generation = guard.generation.saturating_add(1);
            let gen = Generation(guard.generation);

            let (cancel_tx, cancel_rx) = watch::channel(false);
            guard.cancel_tx = Some(cancel_tx);
            guard.status = PlayitTunnelStatus {
                is_running: true,
                status: "starting".to_string(),
                ..stopped_status()
            };
            let _ = self.status_tx.send(guard.status.clone());

            (gen, cancel_rx)
        };

        let mut starting_guard = StartingGuard::new(generation, self.clone());

        std::fs::create_dir_all(playit_dir).map_err(PlayitError::Io)?;

        let binary_path = self
            .adapter
            .ensure_binary(data_dir, client, cancel_rx.clone())
            .await?;

        if self.is_cancelled(generation, &cancel_rx).await {
            return Err(PlayitError::Cancelled);
        }

        let secret = match secret_key.filter(|s| !s.trim().is_empty()) {
            Some(s) => Some(s),
            None => credentials::read_secret(playit_dir)?,
        };

        if self.is_cancelled(generation, &cancel_rx).await {
            return Err(PlayitError::Cancelled);
        }

        let ctx = AgentContext {
            inner: self.inner.clone(),
            status_tx: self.status_tx.clone(),
            adapter: self.adapter.clone(),
            client: client.clone(),
            generation,
            binary_path,
            playit_dir: playit_dir.to_path_buf(),
            sandbox_dir: sandbox_dir.map(Path::to_path_buf),
            cancel_rx,
        };

        {
            let mut guard = self.inner.lock().await;
            if guard.generation != generation.0 {
                return Err(PlayitError::Cancelled);
            }
            guard.task = Some(tokio::spawn(run_agent(ctx, secret)));
        }

        let initial_status = self.wait_for_readiness(generation).await;
        if self.inner.lock().await.generation != generation.0 {
            return Err(PlayitError::Cancelled);
        }
        starting_guard.defuse();

        Ok(initial_status)
    }

    pub async fn stop_tunnel(&self) -> Result<(), PlayitError> {
        self.stop_generation(None).await
    }

    pub(crate) async fn stop_generation(
        &self,
        generation: Option<Generation>,
    ) -> Result<(), PlayitError> {
        let _transition_guard = self.lifecycle_lock.lock().await;
        let mut guard = self.inner.lock().await;
        if generation.is_some_and(|expected| expected.0 != guard.generation) {
            return Ok(());
        }
        guard.generation = guard.generation.saturating_add(1);
        if let Some(tx) = guard.cancel_tx.take() {
            let _ = tx.send(true);
        }
        if let Some(task) = guard.task.take() {
            task.abort();
        }
        cleanup_process(&mut guard, &self.status_tx).await?;
        guard.status = stopped_status();
        let _ = self.status_tx.send(guard.status.clone());
        Ok(())
    }

    async fn is_cancelled(&self, gen: Generation, cancel_rx: &watch::Receiver<bool>) -> bool {
        if *cancel_rx.borrow() {
            return true;
        }
        let guard = self.inner.lock().await;
        guard.generation != gen.0
    }

    async fn wait_for_readiness(&self, gen: Generation) -> PlayitTunnelStatus {
        let mut rx = self.status_rx.clone();
        let timeout = Duration::from_millis(2000);
        let wait_fut = async {
            let current = rx.borrow().clone();
            if current.status != "starting" {
                return current;
            }
            while rx.changed().await.is_ok() {
                let current = rx.borrow().clone();
                let inner = self.inner.lock().await;
                if inner.generation != gen.0 {
                    return current;
                }
                if current.status != "starting" {
                    return current;
                }
            }
            self.get_status().await
        };

        tokio::select! {
            status = wait_fut => status,
            _ = tokio::time::sleep(timeout) => self.get_status().await,
        }
    }
}

async fn run_agent(ctx: AgentContext, secret: Option<String>) {
    let gen = ctx.generation;
    if let Err(err) = run_agent_inner(&ctx, secret).await {
        if matches!(err, PlayitError::Cancelled) {
            return;
        }
        let message = err.to_string();
        eprintln!("[Playit] {message}");
        let mut guard = ctx.inner.lock().await;
        if guard.generation == gen.0 {
            guard.status = PlayitTunnelStatus {
                status: "error".to_string(),
                message: Some(message.clone()),
                ..stopped_status()
            };
            let _ = ctx.status_tx.send(guard.status.clone());
            if let Err(cleanup_error) = cleanup_process(&mut guard, &ctx.status_tx).await {
                guard.status.message = Some(format!("{message}; {cleanup_error}"));
                let _ = ctx.status_tx.send(guard.status.clone());
            }
        }
    }
}

async fn run_agent_inner(ctx: &AgentContext, secret: Option<String>) -> Result<(), PlayitError> {
    let secret = match secret {
        Some(s) => s,
        None => {
            let s = claim_agent(ctx).await?;
            let guard = ctx.inner.lock().await;
            if guard.generation != ctx.generation.0 {
                return Err(PlayitError::Cancelled);
            }
            credentials::write_secret(&ctx.playit_dir, &s)?;
            s
        }
    };

    if *ctx.cancel_rx.borrow() {
        return Err(PlayitError::Cancelled);
    }

    // Registration and spawning are one synchronous ownership transition. There is no
    // cancellation point at which an unregistered child or its reader tasks can escape.
    let mut guard = ctx.inner.lock().await;
    if guard.generation != ctx.generation.0 || *ctx.cancel_rx.borrow() {
        return Err(PlayitError::Cancelled);
    }
    let child = ctx.adapter.spawn_process(
        &ctx.binary_path,
        &secret,
        &ctx.playit_dir,
        ctx.sandbox_dir.as_deref(),
    )?;

    let (process, mut exit_rx) = process::PlayitProcess::new(child);
    guard.process = Some(process);
    guard.status.status = "connecting".to_string();
    guard.status.claim_url = None;
    let _ = ctx.status_tx.send(guard.status.clone());
    drop(guard);

    let mut rundata_interval = tokio::time::interval(Duration::from_secs(5));
    rundata_interval.tick().await;

    let mut cancel_rx = ctx.cancel_rx.clone();
    loop {
        tokio::select! {
            _ = cancel_rx.changed() => return Ok(()),
            exit_res = &mut exit_rx => {
                match exit_res {
                    Ok(Ok(exit)) => {
                        let last = {
                            let mut guard = ctx.inner.lock().await;
                            if let Some(ref mut proc) = guard.process {
                                proc.last_log.lock().await.clone()
                            } else {
                                String::new()
                            }
                        };
                        if last.contains("InvalidAgentKey") || last.contains("no longer valid") {
                            let guard = ctx.inner.lock().await;
                            if guard.generation != ctx.generation.0 {
                                return Err(PlayitError::Cancelled);
                            }
                            let saved_secret = credentials::read_secret(&ctx.playit_dir)?;
                            if saved_secret.as_deref() == Some(secret.as_str()) {
                                credentials::delete_secret(&ctx.playit_dir)?;
                            }
                            return Err(PlayitError::InvalidCredential);
                        }
                        return Err(PlayitError::AgentExited(exit));
                    }
                    Ok(Err(err)) => return Err(PlayitError::Io(err)),
                    Err(_) => return Ok(()),
                }
            }
            _ = rundata_interval.tick() => {
                match ctx.adapter.fetch_rundata(&ctx.client, &secret).await {
                    Ok(data) => apply_rundata(ctx, &data).await,
                    Err(e) => eprintln!("[Playit] Failed to fetch tunnel info: {e}"),
                }
            }
        }
    }
}

// The process stays in the registry across awaits, including when cleanup is
// cancelled or fails. A later stop/start can therefore retry termination.
async fn cleanup_process(
    inner: &mut PlayitInner,
    status_tx: &watch::Sender<PlayitTunnelStatus>,
) -> Result<(), PlayitError> {
    if let Some(process) = inner.process.as_mut() {
        if let Err(error) = process.terminate_and_reap().await {
            inner.status.is_running = true;
            inner.status.status = "error".to_string();
            inner.status.message = Some(format!("Failed to stop playit process: {error}"));
            let _ = status_tx.send(inner.status.clone());
            return Err(error);
        }
    }
    inner.process = None;
    Ok(())
}
