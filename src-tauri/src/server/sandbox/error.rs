use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum SandboxError {
    #[error("I/O error during sandbox operation: {0}")]
    Io(#[from] std::io::Error),

    #[error("Network error during sandbox operation: {0}")]
    Network(#[from] reqwest::Error),

    #[error("Task join error during sandbox operation: {0}")]
    Task(#[from] tokio::task::JoinError),

    #[error("Invalid library path: {0}")]
    JoinPaths(#[from] std::env::JoinPathsError),

    #[error("Failed to get app data directory: {0}")]
    AppDataDir(String),

    #[error(
        "Linux rootfs download failed (HTTP {status}). On mobile, you can use PumpkinMC (Rust) which runs natively without requiring Java or rootfs."
    )]
    DownloadStatus { status: reqwest::StatusCode },

    #[error("PRoot binary not found. The APK must bundle libproot.so.")]
    ProotNotFound,

    #[error("PRoot loader not found at {0}. The APK must bundle libproot-loader.so.")]
    LoaderNotFound(PathBuf),

    #[error("Sandbox rootfs path is required on Android")]
    RootfsRequired,

    #[error("Java package {package} installed but no java binary was found in rootfs")]
    JavaBinaryNotFound { package: String },

    #[error("Failed to install {package} in Linux sandbox ({status}):\n{details}")]
    JavaInstallFailed {
        package: String,
        status: std::process::ExitStatus,
        details: String,
    },

    #[error("{0}")]
    Other(String),
}
