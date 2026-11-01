//! OS desktop and application menu shortcut generation.

use base64::Engine;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use super::models::{InstanceConfig, ModLoaderType};

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

/// Retrieves the bundled multi-resolution ICO icon for a mod loader if available.
pub fn get_default_loader_icon_ico(loader: &ModLoaderType) -> Option<&'static [u8]> {
    match loader {
        ModLoaderType::Vanilla => {
            Some(include_bytes!("../../../icons/loaders/vanilla.ico").as_slice())
        }
        ModLoaderType::Fabric => {
            Some(include_bytes!("../../../icons/loaders/fabric.ico").as_slice())
        }
        ModLoaderType::Forge => Some(include_bytes!("../../../icons/loaders/forge.ico").as_slice()),
        ModLoaderType::NeoForge => {
            Some(include_bytes!("../../../icons/loaders/neoforge.ico").as_slice())
        }
        ModLoaderType::Quilt => Some(include_bytes!("../../../icons/loaders/quilt.ico").as_slice()),
        ModLoaderType::Horizon => {
            Some(include_bytes!("../../../icons/loaders/horizon.ico").as_slice())
        }
    }
}

/// Retrieves the bundled PNG icon for a mod loader if available.
pub fn get_default_loader_icon_png(loader: &ModLoaderType) -> Option<&'static [u8]> {
    match loader {
        ModLoaderType::Vanilla => {
            Some(include_bytes!("../../../icons/loaders/vanilla.png").as_slice())
        }
        ModLoaderType::Fabric => {
            Some(include_bytes!("../../../icons/loaders/fabric.png").as_slice())
        }
        ModLoaderType::Forge => Some(include_bytes!("../../../icons/loaders/forge.png").as_slice()),
        ModLoaderType::NeoForge => {
            Some(include_bytes!("../../../icons/loaders/neoforge.png").as_slice())
        }
        ModLoaderType::Quilt => Some(include_bytes!("../../../icons/loaders/quilt.png").as_slice()),
        ModLoaderType::Horizon => {
            Some(include_bytes!("../../../icons/loaders/horizon.png").as_slice())
        }
    }
}

/// Encapsulates raw PNG image bytes in a standard Windows .ico container.
pub fn png_to_ico(png_bytes: &[u8]) -> Result<Vec<u8>, std::io::Error> {
    if png_bytes.len() < 24 || &png_bytes[0..8] != b"\x89PNG\r\n\x1a\n" {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Invalid PNG image data",
        ));
    }

    let width = u32::from_be_bytes([png_bytes[16], png_bytes[17], png_bytes[18], png_bytes[19]]);
    let height = u32::from_be_bytes([png_bytes[20], png_bytes[21], png_bytes[22], png_bytes[23]]);
    let b_width = if width >= 256 { 0 } else { width as u8 };
    let b_height = if height >= 256 { 0 } else { height as u8 };

    let mut ico = Vec::with_capacity(22 + png_bytes.len());
    // ICONDIR
    ico.extend_from_slice(&0u16.to_le_bytes()); // Reserved
    ico.extend_from_slice(&1u16.to_le_bytes()); // Type: 1 for Icon
    ico.extend_from_slice(&1u16.to_le_bytes()); // Count: 1 image

    // ICONDIRENTRY
    ico.push(b_width);
    ico.push(b_height);
    ico.push(0); // Color count
    ico.push(0); // Reserved
    ico.extend_from_slice(&0u16.to_le_bytes()); // Planes: 0 for PNG
    ico.extend_from_slice(&32u16.to_le_bytes()); // Bit count: 32
    ico.extend_from_slice(&(png_bytes.len() as u32).to_le_bytes()); // Resource size
    ico.extend_from_slice(&22u32.to_le_bytes()); // Offset: 22

    ico.extend_from_slice(png_bytes);
    Ok(ico)
}

fn decode_base64_image(spec: &str) -> Option<Vec<u8>> {
    let comma_pos = spec.find(',')?;
    let header = &spec[..comma_pos];
    if !header.starts_with("data:image/") || !header.contains(";base64") {
        return None;
    }
    let encoded = &spec[comma_pos + 1..];
    base64::engine::general_purpose::STANDARD
        .decode(encoded.trim())
        .ok()
}

#[cfg(windows)]
fn resolve_windows_instance_icon(
    instance_dir: &Path,
    instance: &InstanceConfig,
) -> Option<PathBuf> {
    let ico_path = instance_dir.join("icon.ico");
    let png_path = instance_dir.join("icon.png");

    if let Some(icon_spec) = &instance.icon {
        if let Some(bytes) = decode_base64_image(icon_spec) {
            if let Ok(ico_bytes) = png_to_ico(&bytes) {
                let _ = std::fs::write(&ico_path, ico_bytes);
                let _ = std::fs::write(&png_path, bytes);
                return Some(ico_path);
            }
        } else {
            let custom_path = PathBuf::from(icon_spec);
            if custom_path.is_file() {
                if custom_path.extension().and_then(|e| e.to_str()) == Some("ico") {
                    return Some(custom_path);
                }
                if let Ok(bytes) = std::fs::read(&custom_path) {
                    if let Ok(ico_bytes) = png_to_ico(&bytes) {
                        let _ = std::fs::write(&ico_path, ico_bytes);
                        return Some(ico_path);
                    }
                }
            }
        }
    }

    if let Some(loader_ico) = get_default_loader_icon_ico(&instance.loader) {
        if std::fs::write(&ico_path, loader_ico).is_ok() {
            if let Some(loader_png) = get_default_loader_icon_png(&instance.loader) {
                let _ = std::fs::write(&png_path, loader_png);
            }
            return Some(ico_path);
        }
    }

    if png_path.is_file() {
        if let Ok(bytes) = std::fs::read(&png_path) {
            if bytes.len() > 400 {
                if let Ok(ico_bytes) = png_to_ico(&bytes) {
                    if std::fs::write(&ico_path, ico_bytes).is_ok() {
                        return Some(ico_path);
                    }
                }
            }
        }
    }

    if ico_path.is_file() {
        if let Ok(meta) = std::fs::metadata(&ico_path) {
            if meta.len() > 1000 {
                return Some(ico_path);
            }
        }
    }

    None
}

#[cfg(not(windows))]
fn resolve_unix_instance_icon(instance_dir: &Path, instance: &InstanceConfig) -> Option<PathBuf> {
    let png_path = instance_dir.join("icon.png");

    if let Some(icon_spec) = &instance.icon {
        if let Some(bytes) = decode_base64_image(icon_spec) {
            let _ = std::fs::write(&png_path, bytes);
            return Some(png_path);
        }
        let custom_path = PathBuf::from(icon_spec);
        if custom_path.is_file() {
            return Some(custom_path);
        }
    }

    if let Some(loader_png) = get_default_loader_icon_png(&instance.loader) {
        if std::fs::write(&png_path, loader_png).is_ok() {
            return Some(png_path);
        }
    }

    if png_path.is_file() {
        if let Ok(meta) = std::fs::metadata(&png_path) {
            if meta.len() > 400 {
                return Some(png_path);
            }
        }
    }

    None
}

/// Resolves the file path of an OS-compatible icon for the specified instance.
pub fn resolve_instance_icon(instance_dir: &Path, instance: &InstanceConfig) -> Option<PathBuf> {
    #[cfg(windows)]
    {
        resolve_windows_instance_icon(instance_dir, instance)
    }
    #[cfg(not(windows))]
    {
        resolve_unix_instance_icon(instance_dir, instance)
    }
}

pub fn generate_linux_desktop_entry(
    target_exe: &Path,
    instance_id: &str,
    instance_name: &str,
    icon: &str,
) -> String {
    let exe_str = target_exe.to_string_lossy();
    format!(
        "[Desktop Entry]\n\
         Type=Application\n\
         Version=1.0\n\
         Name=Ingot - {instance_name}\n\
         Comment=Launch {instance_name} with Ingot\n\
         Exec=\"{exe_str}\" --launch-instance {instance_id}\n\
         Icon={icon}\n\
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
#[link(name = "shell32")]
extern "system" {
    fn SHChangeNotify(
        w_event_id: i32,
        u_flags: u32,
        dw_item1: *const std::ffi::c_void,
        dw_item2: *const std::ffi::c_void,
    );
}

#[cfg(windows)]
pub fn notify_shell_change() {
    const SHCNE_ASSOCCHANGED: i32 = 0x08000000;
    const SHCNF_IDLIST: u32 = 0x0000;
    const SHCNF_FLUSH: u32 = 0x1000;
    unsafe {
        SHChangeNotify(
            SHCNE_ASSOCCHANGED,
            SHCNF_IDLIST | SHCNF_FLUSH,
            std::ptr::null(),
            std::ptr::null(),
        );
    }
}

#[cfg(windows)]
fn create_windows_shortcut_com(
    shortcut_path: &Path,
    target_exe: &Path,
    instance_id: &str,
    icon_path: Option<&Path>,
) -> Result<(), std::io::Error> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x08000000;

    let target_str = target_exe.to_string_lossy();
    let shortcut_str = shortcut_path.to_string_lossy();
    let working_dir_str = target_exe
        .parent()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_default();
    let icon_to_set = icon_path.unwrap_or(target_exe).to_string_lossy();

    let target_escaped = target_str.replace('\'', "''");
    let shortcut_escaped = shortcut_str.replace('\'', "''");
    let working_dir_escaped = working_dir_str.replace('\'', "''");
    let icon_escaped = icon_to_set.replace('\'', "''");
    let id_escaped = instance_id.replace('\'', "''");

    let script = format!(
        "$ws = New-Object -ComObject WScript.Shell; \
         $sc = $ws.CreateShortcut('{shortcut_escaped}'); \
         $sc.TargetPath = '{target_escaped}'; \
         $sc.Arguments = '--launch-instance \"{id_escaped}\"'; \
         $sc.WorkingDirectory = '{working_dir_escaped}'; \
         $sc.IconLocation = '{icon_escaped},0'; \
         $sc.Save();"
    );

    let output = std::process::Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &script])
        .creation_flags(CREATE_NO_WINDOW)
        .output()?;

    if output.status.success() {
        Ok(())
    } else {
        let err = String::from_utf8_lossy(&output.stderr);
        Err(std::io::Error::other(err.into_owned()))
    }
}

#[cfg(windows)]
pub fn create_windows_shortcut(
    destination_dir: &Path,
    target_exe: &Path,
    instance_id: &str,
    instance_name: &str,
    icon_path: Option<&Path>,
) -> Result<PathBuf, ShortcutError> {
    std::fs::create_dir_all(destination_dir)?;
    let safe_name = sanitize_shortcut_filename(instance_name);
    let shortcut_path = destination_dir.join(format!("Ingot - {safe_name}.lnk"));

    let icon_to_set = icon_path.filter(|p| p.exists());

    // Try Windows native Shell COM first for 100% Explorer compatibility
    let com_result = create_windows_shortcut_com(
        &shortcut_path,
        target_exe,
        instance_id,
        icon_to_set,
    );

    if com_result.is_err() {
        let mut link = mslnk::ShellLink::new(target_exe)
            .map_err(|e| ShortcutError::Io(std::io::Error::other(e)))?;

        link.set_arguments(Some(format!("--launch-instance \"{instance_id}\"")));
        link.set_relative_path(None);
        if let Some(parent) = target_exe.parent() {
            link.set_working_dir(Some(parent.to_string_lossy().into_owned()));
        }

        let fallback_icon = icon_to_set.unwrap_or(target_exe);
        link.set_icon_location(Some(fallback_icon.to_string_lossy().into_owned()));

        link.create_lnk(&shortcut_path)
            .map_err(|e| ShortcutError::Io(std::io::Error::other(e)))?;
    }

    notify_shell_change();

    Ok(shortcut_path)
}

#[cfg(target_os = "linux")]
pub fn create_linux_shortcut(
    destination_dir: &Path,
    target_exe: &Path,
    instance_id: &str,
    instance_name: &str,
    icon_path: Option<&Path>,
) -> Result<PathBuf, ShortcutError> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::create_dir_all(destination_dir)?;
    let safe_name = sanitize_shortcut_filename(instance_name);
    let shortcut_path = destination_dir.join(format!("ingot-{safe_name}.desktop"));
    let icon_str = icon_path.and_then(|p| p.to_str()).unwrap_or("ingot");
    let content = generate_linux_desktop_entry(target_exe, instance_id, instance_name, icon_str);
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
    icon_path: Option<&Path>,
) -> Result<PathBuf, ShortcutError> {
    use std::os::unix::fs::PermissionsExt;
    let safe_name = sanitize_shortcut_filename(instance_name);
    let app_dir = destination_dir.join(format!("Ingot - {safe_name}.app"));
    let contents_dir = app_dir.join("Contents");
    let macos_dir = contents_dir.join("MacOS");
    let resources_dir = contents_dir.join("Resources");
    std::fs::create_dir_all(&macos_dir)?;
    std::fs::create_dir_all(&resources_dir)?;

    if let Some(src_icon) = icon_path {
        if src_icon.exists() {
            let ext = src_icon
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("png");
            let _ = std::fs::copy(src_icon, resources_dir.join(format!("icon.{ext}")));
        }
    }

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
    icon_path: Option<&Path>,
) -> Result<PathBuf, ShortcutError> {
    #[cfg(windows)]
    {
        create_windows_shortcut(
            destination_dir,
            target_exe,
            instance_id,
            instance_name,
            icon_path,
        )
    }
    #[cfg(target_os = "linux")]
    {
        create_linux_shortcut(
            destination_dir,
            target_exe,
            instance_id,
            instance_name,
            icon_path,
        )
    }
    #[cfg(target_os = "macos")]
    {
        create_macos_shortcut(
            destination_dir,
            target_exe,
            instance_id,
            instance_name,
            icon_path,
        )
    }
    #[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
    {
        let _ = (
            destination_dir,
            target_exe,
            instance_id,
            instance_name,
            icon_path,
        );
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
            "ingot",
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

    #[test]
    fn png_to_ico_converts_valid_png() {
        let loader_png = get_default_loader_icon_png(&ModLoaderType::Horizon).unwrap();
        let ico = png_to_ico(loader_png).unwrap();
        assert_eq!(&ico[0..6], &[0, 0, 1, 0, 1, 0]); // ICO header
        assert_eq!(&ico[22..], loader_png); // PNG payload starts at offset 22
    }

    #[test]
    fn png_to_ico_rejects_invalid_data() {
        let err = png_to_ico(b"not a valid png file").unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
    }

    #[test]
    fn resolves_default_loader_icon() {
        let temp_dir = tempfile::tempdir().unwrap();
        let instance = InstanceConfig {
            id: "inst-test".to_string(),
            name: "Test Instance".to_string(),
            game_version: "1.21.4".to_string(),
            loader: ModLoaderType::Horizon,
            loader_version: None,
            java_path: None,
            memory_min_mb: None,
            memory_max_mb: None,
            icon: None,
            created_at: 0,
            last_played: None,
            total_play_time_seconds: 0,
            jvm_args: None,
            fullscreen: None,
            window_width: None,
            window_height: None,
            sync_options: None,
            sync_servers: None,
            sync_resource_packs: None,
            sync_command_history: None,
            sync_creative_hotbars: None,
            last_synced_at: None,
        };

        let resolved = resolve_instance_icon(temp_dir.path(), &instance);
        assert!(resolved.is_some());
        let icon_path = resolved.unwrap();
        assert!(icon_path.exists());
        #[cfg(windows)]
        assert_eq!(icon_path.extension().unwrap(), "ico");
        #[cfg(not(windows))]
        assert_eq!(icon_path.extension().unwrap(), "png");
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
            None,
        )
        .unwrap();

        assert!(shortcut.exists());
        assert_eq!(
            shortcut.file_name().unwrap().to_string_lossy(),
            "Ingot - Windows Instance.lnk"
        );
    }
}
