//! Platform-specific folder opening and file revealing in OS file managers.
use crate::minecraft::screenshots::ScreenshotError;
use std::path::Path;

/// Opens the screenshots folder corresponding to an instance or Bedrock in the OS file explorer.
pub fn open_screenshots_folder(target_dir: &Path) -> Result<(), ScreenshotError> {
    let _path_str = target_dir.to_string_lossy().to_string();
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(&_path_str)
            .spawn()
            .map_err(ScreenshotError::Io)?;
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&_path_str)
            .spawn()
            .map_err(ScreenshotError::Io)?;
    }
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(&_path_str)
            .spawn()
            .map_err(ScreenshotError::Io)?;
    }

    Ok(())
}

/// Highlights and selects a screenshot file in the OS file manager.
pub fn reveal_screenshot_file(file_path: &str) -> Result<(), ScreenshotError> {
    let path = Path::new(file_path);
    if !path.exists() {
        return Err(ScreenshotError::NotFound(file_path.to_string()));
    }

    #[cfg(target_os = "windows")]
    {
        let win_path = file_path.replace('/', "\\");
        std::process::Command::new("explorer")
            .arg("/select,")
            .arg(&win_path)
            .spawn()
            .map_err(ScreenshotError::Io)?;
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg("-R")
            .arg(file_path)
            .spawn()
            .map_err(ScreenshotError::Io)?;
    }
    #[cfg(target_os = "linux")]
    {
        let parent = path.parent().unwrap_or(path);
        std::process::Command::new("xdg-open")
            .arg(parent)
            .spawn()
            .map_err(ScreenshotError::Io)?;
    }

    Ok(())
}
