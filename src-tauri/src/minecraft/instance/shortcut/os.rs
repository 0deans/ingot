//! Platform-native desktop and application menu shortcut creation (Windows, Linux, macOS).

use std::path::{Path, PathBuf};

use super::{sanitize_shortcut_filename, ShortcutError, ShortcutLocation};

pub fn generate_linux_desktop_entry(
    target_exe: &Path,
    instance_id: &str,
    instance_name: &str,
    icon: &str,
) -> String {
    let exe_str = target_exe.to_string_lossy();
    let safe_id = sanitize_shortcut_filename(instance_id);
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
         StartupWMClass=ingot-instance-{safe_id}\n"
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
pub fn notify_shortcut_created(shortcut_path: &Path) {
    use std::os::windows::ffi::OsStrExt;
    const SHCNE_CREATE: i32 = 0x00000002;
    const SHCNF_PATHW: u32 = 0x0005;

    let wide_path: Vec<u16> = shortcut_path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    unsafe {
        SHChangeNotify(
            SHCNE_CREATE,
            SHCNF_PATHW,
            wide_path.as_ptr() as *const std::ffi::c_void,
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
         $sc.Arguments = '--launch-instance ' + [char]34 + '{id_escaped}' + [char]34; \
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
    let com_result =
        create_windows_shortcut_com(&shortcut_path, target_exe, instance_id, icon_to_set);

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

    notify_shortcut_created(&shortcut_path);

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
    #[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
    {
        let _ = (app, location);
        Err(ShortcutError::UnsupportedPlatform(
            std::env::consts::OS.to_string(),
        ))
    }
    #[cfg(any(windows, target_os = "linux", target_os = "macos"))]
    {
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
                        .join("Ingot")
                        .join("Instances");
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
            }
            ShortcutLocation::Steam => Err(ShortcutError::DirectoryNotFound(
                "Steam shortcuts are resolved through Steam userdata discovery".to_string(),
            )),
        }
    }
}
