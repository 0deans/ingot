use crate::server::config::{ServerCoreType, ServerStatus};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::process::ChildStdin;
use tokio::sync::Mutex;

pub(crate) struct StartAdmissionPause(pub(super) Arc<std::sync::atomic::AtomicUsize>);

impl Drop for StartAdmissionPause {
    fn drop(&mut self) {
        self.0.fetch_sub(1, std::sync::atomic::Ordering::AcqRel);
    }
}

pub(crate) struct ActiveServer {
    pub(crate) server_id: String,
    pub(crate) core: ServerCoreType,
    pub(crate) pid: u32,
    pub(crate) port: u16,
    pub(crate) started_at: u64,
    pub(crate) status: ServerStatus,
    pub(crate) stdin: Arc<Mutex<ChildStdin>>,
    pub(crate) log_history: Arc<Mutex<Vec<String>>>,
    pub(crate) online_players: Arc<Mutex<Vec<String>>>,
    pub(crate) player_xuids: Arc<Mutex<HashMap<String, String>>>,
    /// Every cleaned stdout line, for capturing command replies
    pub(crate) line_tx: tokio::sync::broadcast::Sender<String>,
    /// Serializes console queries so replies cannot be mixed up
    pub(crate) query_lock: Arc<Mutex<()>>,
    /// While > 0, query replies are hidden from the user's console
    pub(crate) pending_queries: Arc<std::sync::atomic::AtomicUsize>,
    /// Set once the server logs "Done (...)"; commands sent earlier crash on Paper
    pub(crate) ready: Arc<std::sync::atomic::AtomicBool>,
    /// Reader tasks; the exit monitor retains ownership until the process is reaped.
    pub(crate) abort_handles: Vec<tokio::task::AbortHandle>,
}

impl ActiveServer {
    pub(crate) fn cancel_tasks(&self) {
        for handle in &self.abort_handles {
            handle.abort();
        }
    }
}

/// Removes a server from the "starting" set when launch finishes or fails
pub struct StartingGuard {
    pub(crate) set: Arc<std::sync::Mutex<HashMap<String, ServerLaunchGeneration>>>,
    pub(crate) server_id: String,
    pub(crate) generation: ServerLaunchGeneration,
}

#[derive(Clone)]
pub(crate) struct ServerLaunchGeneration(Arc<std::sync::Mutex<bool>>);

impl Default for ServerLaunchGeneration {
    fn default() -> Self {
        Self(Arc::new(std::sync::Mutex::new(true)))
    }
}

impl ServerLaunchGeneration {
    fn matches(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }

    pub(crate) fn cancel(&self) {
        match self.0.lock() {
            Ok(mut active) => *active = false,
            Err(poisoned) => *poisoned.into_inner() = false,
        }
    }

    pub(super) fn enter(&self) -> Result<std::sync::MutexGuard<'_, bool>, ProcessError> {
        let active = self.0.lock().map_err(|_| ProcessError::StatePoisoned {
            state: "launch generation",
        })?;
        if !*active {
            return Err(ProcessError::LaunchCancelled);
        }
        Ok(active)
    }
}

impl Drop for StartingGuard {
    fn drop(&mut self) {
        let mut set = match self.set.lock() {
            Ok(set) => set,
            Err(poisoned) => poisoned.into_inner(),
        };
        let owns_reservation = set
            .get(&self.server_id)
            .is_some_and(|generation| generation.matches(&self.generation));
        if owns_reservation {
            set.remove(&self.server_id);
        }
    }
}

/// RAII guard for pending console queries.
/// Decrements `pending_queries` immediately on drop if dropped before normal completion
/// (e.g., cancelled or dropped future), or defers decrement by a specified delay on normal completion
/// so late replies don't leak into the console.
pub struct PendingQueryGuard {
    pending: Arc<std::sync::atomic::AtomicUsize>,
    disarmed: bool,
}

impl PendingQueryGuard {
    pub(crate) fn new(pending: Arc<std::sync::atomic::AtomicUsize>) -> Self {
        pending.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Self {
            pending,
            disarmed: false,
        }
    }

    pub(crate) fn complete_delayed(mut self, delay: std::time::Duration) {
        self.disarmed = true;
        let pending = self.pending.clone();
        tokio::spawn(async move {
            tokio::time::sleep(delay).await;
            pending.fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
        });
    }
}

impl Drop for PendingQueryGuard {
    fn drop(&mut self) {
        if !self.disarmed {
            self.pending
                .fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ProcessError {
    #[error("Server is not running: {0}")]
    NotRunning(String),
    #[error("Server not found: {0}")]
    ServerNotFound(String),
    #[error("Server is already running")]
    AlreadyRunning,
    #[error("Server is already starting")]
    AlreadyStarting,
    #[error("Server is still starting")]
    StillStarting,
    #[error("Server stopped")]
    ServerStopped,
    #[error("Server must be running to put it into sleep mode")]
    MustBeRunningToSleep,
    #[error("No reply to `{0}`")]
    NoReply(String),
    #[error("Failed to write command to server stdin: {0}")]
    StdinWrite(#[source] std::io::Error),
    #[error("Failed to flush server stdin: {0}")]
    StdinFlush(#[source] std::io::Error),
    #[error("Failed to spawn server process: {0}")]
    SpawnFailed(#[source] std::io::Error),
    #[error("Failed to get server PID")]
    MissingPid,
    #[error("Failed to attach stdin")]
    MissingStdin,
    #[error("Failed to attach stdout")]
    MissingStdout,
    #[error("Failed to attach stderr")]
    MissingStderr,
    #[error("Port {port} is already in use ({source}). Close whatever uses it, or give this server another port in Settings.")]
    PortInUse {
        port: u16,
        #[source]
        source: std::io::Error,
    },
    #[error("Failed to start proxy on port {port}: {source}")]
    ProxyStartup {
        port: u16,
        #[source]
        source: std::io::Error,
    },
    #[error("{primary}; server cleanup also failed: {cleanup}")]
    ProxyStartupCleanup {
        #[source]
        primary: Box<ProcessError>,
        cleanup: Box<ProcessError>,
    },
    #[error("Could not find Java console binary in {}", runtime_dir.display())]
    MissingJavaConsoleBinary { runtime_dir: std::path::PathBuf },
    #[error("Server {server_id} did not stop within {timeout:?}")]
    ShutdownTimeout {
        server_id: String,
        timeout: std::time::Duration,
    },
    #[error("Server {state} lock is poisoned")]
    StatePoisoned { state: &'static str },
    #[error("Server starts are temporarily paused")]
    StartAdmissionPaused,
    #[error("Server launch was cancelled")]
    LaunchCancelled,
    #[error("Server configuration or directory error: {0}")]
    Config(#[from] crate::server::config::ConfigError),
    #[error("Server metadata failed: {0}")]
    Metadata(#[from] crate::minecraft::metadata::MetadataError),
    #[error("Server file operation failed: {0}")]
    File(#[from] crate::server::files::ServerFileError),
    #[error("Java runtime failed: {0}")]
    Java(#[from] crate::minecraft::java::JavaError),
    #[error("Server core error: {0}")]
    Core(#[from] crate::server::downloader::ServerCoreError),
    #[error("Installer error: {0}")]
    Installer(#[from] crate::server::installer::InstallerError),
    #[error("Pumpkin server error: {0}")]
    Pumpkin(#[from] crate::server::pumpkin::PumpkinError),
    #[error("Bedrock server error: {0}")]
    Bedrock(#[from] crate::server::bedrock::BedrockServerError),
    #[error("Sandbox error: {0}")]
    Sandbox(#[from] crate::server::sandbox::SandboxError),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}
