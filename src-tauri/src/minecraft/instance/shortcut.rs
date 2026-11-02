//! OS desktop, application menu, and Steam non-Steam shortcut generation.

pub mod icons;
pub mod os;
pub mod steam;

use serde::{Deserialize, Serialize};

pub use icons::{
    get_default_loader_icon_ico, get_default_loader_icon_png, png_to_ico, render_preset_icon_png,
    resolve_instance_icon, resolve_instance_icon_bytes,
};
#[cfg(target_os = "linux")]
pub use os::create_linux_shortcut;
#[cfg(target_os = "macos")]
pub use os::create_macos_shortcut;
pub use os::{
    create_platform_shortcut, generate_linux_desktop_entry, generate_macos_plist,
    resolve_shortcut_directory,
};
#[cfg(windows)]
pub use os::{create_windows_shortcut, notify_shortcut_created};
pub use steam::{create_steam_shortcut, export_instance_to_steam_configs};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, specta::Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ShortcutLocation {
    Desktop,
    ApplicationsMenu,
    Steam,
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
    #[error("No Steam installation with an active user profile was found on this system")]
    SteamNotFound,
    #[error("Steam shortcuts.vdf is corrupt and was not modified: {0}")]
    SteamCorruptVdf(String),
    #[error("Failed to generate Steam grid artwork: {0}")]
    Image(String),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::minecraft::instance::models::{InstanceConfig, ModLoaderType};
    use std::path::Path;

    fn sample_instance(id: &str, name: &str, loader: ModLoaderType) -> InstanceConfig {
        InstanceConfig {
            id: id.to_string(),
            name: name.to_string(),
            game_version: "1.21.4".to_string(),
            loader,
            loader_version: None,
            java_path: None,
            memory_min_mb: None,
            memory_max_mb: None,
            icon: None,
            group: None,
            color_tag: None,
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
        }
    }

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
        assert!(entry.contains("StartupWMClass=ingot-instance-test-inst-1"));
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
        assert_eq!(&ico[0..6], &[0, 0, 1, 0, 1, 0]);
        assert_eq!(&ico[22..], loader_png);
    }

    #[test]
    fn png_to_ico_rejects_invalid_data() {
        let err = png_to_ico(b"not a valid png file").unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
    }

    #[test]
    fn resolves_default_loader_icon() {
        let temp_dir = tempfile::tempdir().unwrap();
        let instance = sample_instance("inst-test", "Test Instance", ModLoaderType::Horizon);

        let resolved = resolve_instance_icon(temp_dir.path(), &instance);
        assert!(resolved.is_some());
        let icon_path = resolved.unwrap();
        assert!(icon_path.exists());
        #[cfg(windows)]
        assert_eq!(icon_path.extension().unwrap(), "ico");
        #[cfg(not(windows))]
        assert_eq!(icon_path.extension().unwrap(), "png");
    }

    #[test]
    fn exports_instance_to_steam_with_custom_grid_banners_and_preserves_corrupt_files() {
        let temp_dir = tempfile::tempdir().unwrap();
        let steam_root = temp_dir.path().join("Steam");
        let user_config = steam_root.join("userdata").join("12345678").join("config");
        std::fs::create_dir_all(&user_config).unwrap();

        let instance_dir = temp_dir.path().join("instance");
        std::fs::create_dir_all(&instance_dir).unwrap();
        let mut instance = sample_instance("steam-inst-1", "Deck Adventure", ModLoaderType::Fabric);
        instance.icon = Some("preset:creeper".to_string());
        instance.group = Some("1.21 SMP".to_string());
        instance.color_tag = Some("#38bdf8".to_string());
        instance.last_played = Some(1_715_000_000);
        let target_exe = Path::new("/opt/ingot/ingot");

        let resolved_configs = steam::resolve_userdata_config_dirs(&[steam_root]);
        assert_eq!(resolved_configs.len(), 1);

        let vdf_path = export_instance_to_steam_configs(
            &resolved_configs,
            &instance_dir,
            target_exe,
            &instance,
        )
        .unwrap();
        assert!(vdf_path.is_file());

        let vdf_bytes = std::fs::read(&vdf_path).unwrap();
        let parsed = steam::vdf::parse_shortcuts_vdf(&vdf_bytes).unwrap();
        let steam::vdf::VdfValue::Map(shortcuts) = &parsed[0].value else {
            panic!("expected shortcuts map");
        };
        assert_eq!(shortcuts.len(), 1);

        let app_id = steam::vdf::compute_steam_app_id(
            &steam::vdf::quote_steam_path(target_exe),
            "Ingot - Deck Adventure",
        );
        let grid_dir = user_config.join("grid");
        let portrait = image::open(grid_dir.join(format!("{app_id}p.png"))).unwrap();
        assert_eq!((portrait.width(), portrait.height()), (600, 900));

        let wide = image::open(grid_dir.join(format!("{app_id}.png"))).unwrap();
        assert_eq!((wide.width(), wide.height()), (920, 430));

        let hero = image::open(grid_dir.join(format!("{app_id}_hero.png"))).unwrap();
        assert_eq!((hero.width(), hero.height()), (1920, 620));

        let logo = image::open(grid_dir.join(format!("{app_id}_logo.png"))).unwrap();
        assert_eq!((logo.width(), logo.height()), (600, 300));

        let icon = image::open(grid_dir.join(format!("{app_id}_icon.png"))).unwrap();
        assert_eq!((icon.width(), icon.height()), (256, 256));

        let preset_png = render_preset_icon_png("preset:creeper").unwrap();
        let decoded_preset = image::load_from_memory(&preset_png).unwrap();
        assert_eq!(
            (decoded_preset.width(), decoded_preset.height()),
            (256, 256)
        );

        // Corrupting shortcuts.vdf must fail with SteamCorruptVdf and not overwrite the file
        let corrupt_payload = b"\x00shortcuts\x00\x99invalid";
        std::fs::write(&vdf_path, corrupt_payload).unwrap();
        let err = export_instance_to_steam_configs(
            &resolved_configs,
            &instance_dir,
            target_exe,
            &instance,
        )
        .unwrap_err();
        assert!(matches!(err, ShortcutError::SteamCorruptVdf(_)));
        assert_eq!(std::fs::read(&vdf_path).unwrap(), corrupt_payload);
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

        let script = format!(
            "$ws = New-Object -ComObject WScript.Shell; \
             $sc = $ws.CreateShortcut('{}'); \
             $sc.Arguments",
            shortcut.to_string_lossy().replace('\'', "''")
        );
        let output = std::process::Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", &script])
            .output()
            .unwrap();
        let args = String::from_utf8_lossy(&output.stdout).trim().to_string();
        assert_eq!(args, "--launch-instance \"inst-windows-test\"");
    }
}
