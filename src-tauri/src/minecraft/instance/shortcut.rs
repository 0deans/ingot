//! OS desktop and application menu shortcut generation.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, specta::Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ShortcutLocation {
    Desktop,
    ApplicationsMenu,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ShortcutResult {
    pub path: String,
}

#[derive(Debug, thiserror::Error)]
pub enum ShortcutError {
    #[error("Failed to perform shortcut file operation: {0}")]
    Io(#[from] std::io::Error),
    #[error("Could not determine current executable path: {0}")]
    Executable(#[source] std::io::Error),
    #[error("Shortcut target directory not found: {0}")]
    DirectoryNotFound(String),
    #[error("Shortcut creation is not supported on this platform: {0}")]
    UnsupportedPlatform(String),
}

/// Sanitizes an instance name for safe use in file systems.
pub fn sanitize_shortcut_filename(name: &str) -> String {
    let sanitized: String = name
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' | '\0' => '-',
            _ => c,
        })
        .collect();
    let trimmed = sanitized.trim_matches(|c: char| c.is_whitespace() || c == '.' || c == '-');
    if trimmed.is_empty() {
        "Minecraft-Instance".to_string()
    } else {
        trimmed.to_string()
    }
}

pub fn generate_linux_desktop_entry(
    target_exe: &Path,
    instance_id: &str,
    instance_name: &str,
) -> String {
    let exe_str = target_exe.to_string_lossy();
    format!(
        "[Desktop Entry]\n\
         Type=Application\n\
         Version=1.0\n\
         Name=Ingot - {instance_name}\n\
         Comment=Launch {instance_name} with Ingot\n\
         Exec=\"{exe_str}\" --launch-instance {instance_id}\n\
         Icon=ingot\n\
         Terminal=false\n\
         Categories=Game;\n\
         StartupWMClass=ingot\n"
    )
}

pub fn generate_macos_plist(instance_name: &str, instance_id: &str) -> String {
    let safe_id = sanitize_shortcut_filename(instance_id);
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n\
         <plist version=\"1.0\">\n\
         <dict>\n\
             <key>CFBundlePackageType</key>\n\
             <string>APPL</string>\n\
             <key>CFBundleName</key>\n\
             <string>Ingot - {instance_name}</string>\n\
             <key>CFBundleExecutable</key>\n\
             <string>launcher</string>\n\
             <key>CFBundleIdentifier</key>\n\
             <string>com.ingot.instance.{safe_id}</string>\n\
             <key>CFBundleVersion</key>\n\
             <string>1.0</string>\n\
         </dict>\n\
         </plist>\n"
    )
}

#[cfg(windows)]
pub fn create_windows_shortcut(
    destination_dir: &Path,
    target_exe: &Path,
    instance_id: &str,
    instance_name: &str,
) -> Result<PathBuf, ShortcutError> {
    std::fs::create_dir_all(destination_dir)?;
    let safe_name = sanitize_shortcut_filename(instance_name);
    let shortcut_path = destination_dir.join(format!("Ingot - {safe_name}.lnk"));

    let mut link = mslnk::ShellLink::new(target_exe)
        .map_err(|e| ShortcutError::Io(std::io::Error::other(e)))?;

    link.set_arguments(Some(format!("--launch-instance \"{instance_id}\"")));
    if let Some(parent) = target_exe.parent() {
        link.set_working_dir(Some(parent.to_string_lossy().into_owned()));
    }
    link.set_icon_location(Some(target_exe.to_string_lossy().into_owned()));

    link.create_lnk(&shortcut_path)
        .map_err(|e| ShortcutError::Io(std::io::Error::other(e)))?;
    Ok(shortcut_path)
}

#[cfg(target_os = "linux")]
pub fn create_linux_shortcut(
    destination_dir: &Path,
    target_exe: &Path,
    instance_id: &str,
    instance_name: &str,
) -> Result<PathBuf, ShortcutError> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::create_dir_all(destination_dir)?;
    let safe_name = sanitize_shortcut_filename(instance_name);
    let shortcut_path = destination_dir.join(format!("ingot-{safe_name}.desktop"));
    let content = generate_linux_desktop_entry(target_exe, instance_id, instance_name);
    std::fs::write(&shortcut_path, content)?;

    let mut perms = std::fs::metadata(&shortcut_path)?.permissions();
    perms.set_mode(0o755);
    let _ = std::fs::set_permissions(&shortcut_path, perms);

    Ok(shortcut_path)
}

#[cfg(target_os = "macos")]
pub fn create_macos_shortcut(
    destination_dir: &Path,
    target_exe: &Path,
    instance_id: &str,
    instance_name: &str,
) -> Result<PathBuf, ShortcutError> {
    use std::os::unix::fs::PermissionsExt;
    let safe_name = sanitize_shortcut_filename(instance_name);
    let app_dir = destination_dir.join(format!("Ingot - {safe_name}.app"));
    let contents_dir = app_dir.join("Contents");
    let macos_dir = contents_dir.join("MacOS");
    std::fs::create_dir_all(&macos_dir)?;

    let plist_content = generate_macos_plist(instance_name, instance_id);
    std::fs::write(contents_dir.join("Info.plist"), plist_content)?;

    let launcher_path = macos_dir.join("launcher");
    let exe_str = target_exe.to_string_lossy();
    let script = format!("#!/bin/sh\nexec \"{exe_str}\" --launch-instance \"{instance_id}\"\n");
    std::fs::write(&launcher_path, script)?;

    let mut perms = std::fs::metadata(&launcher_path)?.permissions();
    perms.set_mode(0o755);
    let _ = std::fs::set_permissions(&launcher_path, perms);

    Ok(app_dir)
}

pub fn create_platform_shortcut(
    destination_dir: &Path,
    target_exe: &Path,
    instance_id: &str,
    instance_name: &str,
) -> Result<PathBuf, ShortcutError> {
    #[cfg(windows)]
    {
        create_windows_shortcut(destination_dir, target_exe, instance_id, instance_name)
    }
    #[cfg(target_os = "linux")]
    {
        create_linux_shortcut(destination_dir, target_exe, instance_id, instance_name)
    }
    #[cfg(target_os = "macos")]
    {
        create_macos_shortcut(destination_dir, target_exe, instance_id, instance_name)
    }
    #[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
    {
        let _ = (destination_dir, target_exe, instance_id, instance_name);
        Err(ShortcutError::UnsupportedPlatform(
            std::env::consts::OS.to_string(),
        ))
    }
}

pub fn resolve_shortcut_directory<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    location: ShortcutLocation,
) -> Result<PathBuf, ShortcutError> {
    use tauri::Manager;
    match location {
        ShortcutLocation::Desktop => app
            .path()
            .desktop_dir()
            .map_err(|e| ShortcutError::DirectoryNotFound(e.to_string())),
        ShortcutLocation::ApplicationsMenu => {
            #[cfg(windows)]
            {
                let data = app
                    .path()
                    .data_dir()
                    .map_err(|e| ShortcutError::DirectoryNotFound(e.to_string()))?;
                let start_menu = data
                    .join("Microsoft")
                    .join("Windows")
                    .join("Start Menu")
                    .join("Programs")
                    .join("Ingot");
                Ok(start_menu)
            }
            #[cfg(target_os = "linux")]
            {
                let data = app
                    .path()
                    .data_dir()
                    .map_err(|e| ShortcutError::DirectoryNotFound(e.to_string()))?;
                Ok(data.join("applications"))
            }
            #[cfg(target_os = "macos")]
            {
                let home = app
                    .path()
                    .home_dir()
                    .map_err(|e| ShortcutError::DirectoryNotFound(e.to_string()))?;
                Ok(home.join("Applications"))
            }
            #[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
            {
                Err(ShortcutError::UnsupportedPlatform(
                    std::env::consts::OS.to_string(),
                ))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitizes_forbidden_characters() {
        assert_eq!(
            sanitize_shortcut_filename("My:Cool*World?/|<>\""),
            "My-Cool-World"
        );
        assert_eq!(
            sanitize_shortcut_filename("...Trim Spaces and Dots..."),
            "Trim Spaces and Dots"
        );
        assert_eq!(sanitize_shortcut_filename(""), "Minecraft-Instance");
        assert_eq!(
            sanitize_shortcut_filename("   :::   "),
            "Minecraft-Instance"
        );
    }

    #[test]
    fn generates_valid_linux_desktop_entry() {
        let entry = generate_linux_desktop_entry(
            Path::new("/usr/bin/ingot"),
            "test-inst-1",
            "Survival World",
        );
        assert!(entry.contains("Type=Application"));
        assert!(entry.contains("Name=Ingot - Survival World"));
        assert!(entry.contains("Exec=\"/usr/bin/ingot\" --launch-instance test-inst-1"));
        assert!(entry.contains("Icon=ingot"));
    }

    #[test]
    fn generates_valid_macos_plist() {
        let plist = generate_macos_plist("My Modpack", "pack-123");
        assert!(plist.contains("<string>Ingot - My Modpack</string>"));
        assert!(plist.contains("<string>launcher</string>"));
        assert!(plist.contains("<string>com.ingot.instance.pack-123</string>"));
    }

    #[cfg(windows)]
    #[test]
    fn creates_windows_shortcut_in_temporary_directory() {
        let temp_dir = tempfile::tempdir().unwrap();
        let target_exe = std::env::current_exe().unwrap();
        let shortcut = create_windows_shortcut(
            temp_dir.path(),
            &target_exe,
            "inst-windows-test",
            "Windows Instance",
        )
        .unwrap();

        assert!(shortcut.exists());
        assert_eq!(
            shortcut.file_name().unwrap().to_string_lossy(),
            "Ingot - Windows Instance.lnk"
        );
    }
}
