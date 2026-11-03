use crate::version_change::crash::CrashSuspect;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum LogLevel {
    Fatal,
    Error,
    Warn,
    Info,
    Debug,
    Launcher,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct InstanceLogLine {
    pub seq: u64,
    pub timestamp: Option<String>,
    pub thread: Option<String>,
    pub source: Option<String>,
    pub level: LogLevel,
    pub message: String,
    pub raw: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct InstanceLogEvent {
    pub instance_id: String,
    pub line: InstanceLogLine,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum LogFileKind {
    Log,
    CrashReport,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct InstanceLogFileSummary {
    pub file_name: String,
    pub kind: LogFileKind,
    pub size_bytes: u64,
    pub modified_at: u64,
    pub compressed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct InstanceLogFileContent {
    pub file_name: String,
    pub kind: LogFileKind,
    pub lines: Vec<InstanceLogLine>,
    pub raw: String,
    pub suspects: Vec<CrashSuspect>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct InstanceCrashEvent {
    pub instance_id: String,
    pub instance_name: String,
    pub exit_code: Option<i32>,
    pub crash_report_file: Option<String>,
    pub description: Option<String>,
    pub excerpt: String,
    pub suspects: Vec<CrashSuspect>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum PasteProvider {
    Mclogs,
    PasteEe,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PasteUploadResult {
    pub provider: PasteProvider,
    pub id: String,
    pub url: String,
    pub raw_url: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum LogError {
    #[error("Invalid log file name: {0}")]
    InvalidFileName(String),

    #[error("Log file not found: {0}")]
    NotFound(String),

    #[error("Log content cannot be empty")]
    EmptyContent,

    #[error("Log file is too large to read ({0} bytes)")]
    FileTooLarge(u64),

    #[error("Failed to read log file: {0}")]
    Io(#[from] std::io::Error),

    #[error("Instance error: {0}")]
    Instance(#[from] crate::minecraft::instance::InstanceError),

    #[error("Background worker failed: {0}")]
    Task(#[from] tokio::task::JoinError),

    #[error("Paste upload request failed: {0}")]
    Transport(#[source] reqwest::Error),

    #[error("Paste service returned HTTP {status}: {message}")]
    HttpStatus { status: u16, message: String },

    #[error("Paste service rejected the log: {0}")]
    ProviderRejected(String),

    #[error("Invalid response from paste service: {0}")]
    InvalidResponse(String),
}
