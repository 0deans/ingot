use crate::server::config::{ServerCoreType, ServerStatus};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::process::ChildStdin;
use tokio::sync::Mutex;

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
}

/// Removes a server from the "starting" set when launch finishes or fails
pub struct StartingGuard {
    pub(crate) set: Arc<std::sync::Mutex<std::collections::HashSet<String>>>,
    pub(crate) server_id: String,
}

impl Drop for StartingGuard {
    fn drop(&mut self) {
        if let Ok(mut set) = self.set.lock() {
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
            self.pending.fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ProcessError {
    #[error("Server is not running: {0}")]
    NotRunning(String),
    #[error("Server is already running")]
    AlreadyRunning,
    #[error("Server is already starting")]
    AlreadyStarting,
    #[error("Server is still starting")]
    StillStarting,
    #[error("Server stopped")]
    ServerStopped,
    #[error("No reply to `{0}`")]
    NoReply(String),
    #[error("Failed to write command to server stdin: {0}")]
    StdinWrite(String),
    #[error("Failed to flush server stdin: {0}")]
    StdinFlush(String),
    #[error("Failed to download server jar: {0}")]
    JarDownload(String),
    #[error("Failed to spawn server process: {0}")]
    SpawnFailed(String),
    #[error("Failed to get server PID")]
    MissingPid,
    #[error("Failed to attach stdin")]
    MissingStdin,
    #[error("Failed to attach stdout")]
    MissingStdout,
    #[error("Failed to attach stderr")]
    MissingStderr,
    #[error("Server configuration or directory error: {0}")]
    Config(String),
    #[error("{0}")]
    Other(String),
}

impl From<ProcessError> for String {
    fn from(e: ProcessError) -> Self {
        e.to_string()
    }
}
