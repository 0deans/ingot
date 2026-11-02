//! Steam non-Steam shortcut registration and custom grid artwork export.

pub mod grid;
pub mod vdf;

use std::collections::HashSet;
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::minecraft::instance::models::{InstanceConfig, ModLoaderType};
use crate::minecraft::instance::shortcut::icons::resolve_instance_icon;
use crate::minecraft::instance::shortcut::ShortcutError;

use self::grid::generate_steam_grid_assets;
use self::vdf::{
    compute_steam_app_id, parse_shortcuts_vdf, quote_steam_path, serialize_shortcuts_vdf,
    upsert_steam_shortcut, SteamShortcutSpec,
};

/// Discovers active Steam `userdata/<uid>/config` directories and exports the instance shortcut + custom grid banners.
pub fn create_steam_shortcut(
    instance_dir: &Path,
    target_exe: &Path,
    instance: &InstanceConfig,
) -> Result<PathBuf, ShortcutError> {
    let steam_roots = discover_steam_roots();
    let config_dirs = resolve_userdata_config_dirs(&steam_roots);
    if config_dirs.is_empty() {
        return Err(ShortcutError::SteamNotFound);
    }

    export_instance_to_steam_configs(&config_dirs, instance_dir, target_exe, instance)
}

/// Exports the instance shortcut and 5 custom grid banners into each provided Steam `userdata/<uid>/config` directory.
pub fn export_instance_to_steam_configs(
    config_dirs: &[PathBuf],
    instance_dir: &Path,
    target_exe: &Path,
    instance: &InstanceConfig,
) -> Result<PathBuf, ShortcutError> {
    if config_dirs.is_empty() {
        return Err(ShortcutError::SteamNotFound);
    }

    let app_name = format!("Ingot - {}", instance.name);
    let quoted_exe = quote_steam_path(target_exe);
    let app_id = compute_steam_app_id(&quoted_exe, &app_name);
    let start_dir = target_exe
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    let os_icon_path = resolve_instance_icon(instance_dir, instance);

    let mut primary_vdf_path: Option<PathBuf> = None;

    for config_dir in config_dirs {
        std::fs::create_dir_all(config_dir)?;
        let grid_dir = config_dir.join("grid");
        let grid_assets = generate_steam_grid_assets(&grid_dir, app_id, instance_dir, instance)?;

        // On Windows, Steam reads .ico or .png; on Linux/macOS, the 256x256 grid PNG is ideal
        let shortcut_icon = os_icon_path
            .clone()
            .unwrap_or_else(|| grid_assets.icon_path.clone());

        let vdf_path = config_dir.join("shortcuts.vdf");
        let existing_bytes = match std::fs::read(&vdf_path) {
            Ok(bytes) => bytes,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Vec::new(),
            Err(err) => return Err(ShortcutError::Io(err)),
        };

        let mut root_entries =
            parse_shortcuts_vdf(&existing_bytes).map_err(ShortcutError::SteamCorruptVdf)?;

        let spec = SteamShortcutSpec {
            instance_id: instance.id.clone(),
            app_name: app_name.clone(),
            target_exe: target_exe.to_path_buf(),
            start_dir: start_dir.clone(),
            icon_path: Some(shortcut_icon),
            group: instance.group.clone(),
            loader_name: loader_tag_label(&instance.loader).to_string(),
            last_played: instance.last_played,
        };

        upsert_steam_shortcut(&mut root_entries, &spec).map_err(ShortcutError::SteamCorruptVdf)?;
        let serialized = serialize_shortcuts_vdf(&root_entries);

        let mut staged = tempfile::NamedTempFile::new_in(config_dir)?;
        staged.write_all(&serialized)?;
        staged.flush()?;
        staged.persist(&vdf_path).map_err(|e| e.error)?;

        if primary_vdf_path.is_none() {
            primary_vdf_path = Some(vdf_path);
        }
    }

    primary_vdf_path.ok_or(ShortcutError::SteamNotFound)
}

fn loader_tag_label(loader: &ModLoaderType) -> &'static str {
    match loader {
        ModLoaderType::Vanilla => "Vanilla",
        ModLoaderType::Fabric => "Fabric",
        ModLoaderType::Forge => "Forge",
        ModLoaderType::NeoForge => "NeoForge",
        ModLoaderType::Quilt => "Quilt",
        ModLoaderType::Horizon => "Horizon",
    }
}

/// Resolves `<steam_root>/userdata/<numeric_uid>/config` directories across candidate Steam root paths.
pub fn resolve_userdata_config_dirs(steam_roots: &[PathBuf]) -> Vec<PathBuf> {
    let mut seen_canonical = HashSet::new();
    let mut configs = Vec::new();

    for root in steam_roots {
        let userdata_dir = root.join("userdata");
        let Ok(entries) = std::fs::read_dir(&userdata_dir) else {
            continue;
        };

        let mut user_dirs: Vec<PathBuf> = entries
            .filter_map(|entry| {
                let entry = entry.ok()?;
                let path = entry.path();
                if !path.is_dir() {
                    return None;
                }
                let name = path.file_name()?.to_str()?;
                let uid = name.parse::<u64>().ok()?;
                if uid == 0 {
                    return None;
                }
                Some(path)
            })
            .collect();

        user_dirs.sort();

        for user_dir in user_dirs {
            let dedup_key = std::fs::canonicalize(&user_dir).unwrap_or_else(|_| user_dir.clone());
            if seen_canonical.insert(dedup_key) {
                configs.push(user_dir.join("config"));
            }
        }
    }

    configs
}

fn discover_steam_roots() -> Vec<PathBuf> {
    #[allow(unused_mut)]
    let mut candidates: Vec<PathBuf> = Vec::new();

    #[cfg(windows)]
    {
        if let Some(reg_path) = query_windows_registry_steam_path() {
            candidates.push(reg_path);
        }
        if let Ok(pf_x86) = std::env::var("ProgramFiles(x86)") {
            candidates.push(PathBuf::from(pf_x86).join("Steam"));
        }
        if let Ok(pf) = std::env::var("ProgramFiles") {
            candidates.push(PathBuf::from(pf).join("Steam"));
        }
        if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
            candidates.push(PathBuf::from(local_app_data).join("Steam"));
        }
    }

    #[cfg(target_os = "linux")]
    {
        if let Ok(home) = std::env::var("HOME") {
            let home_path = PathBuf::from(home);
            candidates.push(home_path.join(".local/share/Steam"));
            candidates.push(home_path.join(".steam/steam"));
            candidates.push(home_path.join(".steam/root"));
            candidates.push(home_path.join(".var/app/com.valvesoftware.Steam/.local/share/Steam"));
            candidates.push(home_path.join("snap/steam/common/.local/share/Steam"));
        }
    }

    #[cfg(target_os = "macos")]
    {
        if let Ok(home) = std::env::var("HOME") {
            candidates.push(
                PathBuf::from(home)
                    .join("Library")
                    .join("Application Support")
                    .join("Steam"),
            );
        }
    }

    candidates.into_iter().filter(|p| p.is_dir()).collect()
}

#[cfg(windows)]
fn query_windows_registry_steam_path() -> Option<PathBuf> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x08000000;

    let output = std::process::Command::new("reg")
        .args(["query", r"HKCU\Software\Valve\Steam", "/v", "SteamPath"])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        if line.contains("SteamPath") {
            if let Some(pos) = line.find("REG_SZ") {
                let raw_path = line[pos + "REG_SZ".len()..].trim();
                if !raw_path.is_empty() {
                    return Some(PathBuf::from(raw_path.replace('/', "\\")));
                }
            }
        }
    }

    None
}
