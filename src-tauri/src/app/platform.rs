use std::path::Path;

#[derive(Debug, thiserror::Error)]
pub(crate) enum OpenDirectoryError {
    #[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
    #[error("Failed to open the system file manager: {0}")]
    Spawn(#[source] std::io::Error),
    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    #[error("Opening a directory is unsupported on this platform")]
    Unsupported,
}

pub(crate) fn open_directory(directory: &Path) -> Result<(), OpenDirectoryError> {
    #[cfg(target_os = "windows")]
    let program = "explorer";
    #[cfg(target_os = "macos")]
    let program = "open";
    #[cfg(target_os = "linux")]
    let program = "xdg-open";
    #[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
    {
        std::process::Command::new(program)
            .arg(directory)
            .spawn()
            .map_err(OpenDirectoryError::Spawn)?;
        Ok(())
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        let _ = directory;
        Err(OpenDirectoryError::Unsupported)
    }
}
