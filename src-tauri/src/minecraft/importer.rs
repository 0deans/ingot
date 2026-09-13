use crate::minecraft::instance::ModLoaderType;
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};
use tauri::{Manager, Runtime};

#[taurpc::ipc_type]
#[derive(Debug)]
#[serde(rename_all = "camelCase")]
pub struct DetectedLauncher {
    pub id: String,
    pub name: String,
    pub icon: String,
    pub path: String,
    pub instances_count: u32,
    pub available: bool,
}

#[taurpc::ipc_type]
#[derive(Debug)]
#[serde(rename_all = "camelCase")]
pub struct ImportableInstance {
    pub id: String,
    pub launcher_id: String,
    pub name: String,
    pub game_version: String,
    pub loader: ModLoaderType,
    pub loader_version: Option<String>,
    pub icon: Option<String>,
    pub source_path: String,
    pub mods_count: u32,
    pub saves_count: u32,
    pub screenshots_count: u32,
}

#[taurpc::ipc_type]
#[derive(Debug)]
#[serde(rename_all = "camelCase")]
pub struct ImportInstanceOptions {
    pub source_path: String,
    pub launcher_id: String,
    pub name: String,
    pub game_version: String,
    pub loader: ModLoaderType,
    pub loader_version: Option<String>,
    pub copy_mods: bool,
    pub copy_configs: bool,
    pub copy_saves: bool,
    pub copy_resource_packs: bool,
    pub copy_options: bool,
    pub copy_servers: bool,
    pub copy_screenshots: bool,
    pub copy_extra_data: bool,
}

#[taurpc::ipc_type]
#[derive(Debug)]
#[serde(rename_all = "camelCase")]
pub struct ImportReport {
    pub success: bool,
    pub instance_id: String,
    pub name: String,
    pub mods_copied: u32,
    pub saves_copied: u32,
    pub message: String,
}

fn get_appdata_dir<R: Runtime>(app: &tauri::AppHandle<R>) -> Option<PathBuf> {
    if let Ok(p) = std::env::var("APPDATA") {
        return Some(PathBuf::from(p));
    }
    app.path()
        .app_data_dir()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
}

fn get_local_appdata_dir() -> Option<PathBuf> {
    std::env::var("LOCALAPPDATA").ok().map(PathBuf::from)
}

fn get_user_home<R: Runtime>(app: &tauri::AppHandle<R>) -> Option<PathBuf> {
    if let Ok(p) = std::env::var("USERPROFILE") {
        return Some(PathBuf::from(p));
    }
    if let Ok(p) = std::env::var("HOME") {
        return Some(PathBuf::from(p));
    }
    app.path().home_dir().ok()
}

fn get_curseforge_dirs<R: Runtime>(app: &tauri::AppHandle<R>) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(home) = get_user_home(app) {
        dirs.push(home.join("curseforge").join("minecraft").join("Instances"));
        dirs.push(home.join("CurseForge").join("minecraft").join("Instances"));
        dirs.push(
            home.join("Documents")
                .join("curseforge")
                .join("minecraft")
                .join("Instances"),
        );
    }
    dirs
}

fn get_modrinth_dirs<R: Runtime>(app: &tauri::AppHandle<R>) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(appdata) = get_appdata_dir(app) {
        dirs.push(appdata.join("Modrinth App").join("profiles"));
        dirs.push(appdata.join("com.modrinth.theseus").join("profiles"));
    }
    if let Some(local) = get_local_appdata_dir() {
        dirs.push(local.join("Modrinth App").join("profiles"));
    }
    if let Some(home) = get_user_home(app) {
        dirs.push(
            home.join("Library")
                .join("Application Support")
                .join("Modrinth App")
                .join("profiles"),
        );
        dirs.push(home.join(".config").join("Modrinth App").join("profiles"));
    }
    dirs
}

fn get_atlauncher_dirs<R: Runtime>(app: &tauri::AppHandle<R>) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(appdata) = get_appdata_dir(app) {
        dirs.push(appdata.join("ATLauncher").join("instances"));
    }
    if let Some(local) = get_local_appdata_dir() {
        dirs.push(local.join("Programs").join("ATLauncher").join("instances"));
        dirs.push(local.join("ATLauncher").join("instances"));
    }
    dirs.push(PathBuf::from("C:\\ATLauncher\\instances"));
    if let Some(home) = get_user_home(app) {
        dirs.push(home.join("atlauncher").join("instances"));
        dirs.push(home.join("ATLauncher").join("instances"));
    }
    dirs
}

fn get_legacylauncher_dirs<R: Runtime>(app: &tauri::AppHandle<R>) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(appdata) = get_appdata_dir(app) {
        dirs.push(
            appdata
                .join(".tlauncher")
                .join("legacy")
                .join("Minecraft")
                .join("game"),
        );
        dirs.push(appdata.join(".tlauncher").join("legacy").join("Minecraft"));
        dirs.push(appdata.join(".tlauncher"));
    }
    dirs
}

fn get_vanilla_dirs<R: Runtime>(app: &tauri::AppHandle<R>) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(appdata) = get_appdata_dir(app) {
        dirs.push(appdata.join(".minecraft"));
    }
    if let Some(home) = get_user_home(app) {
        dirs.push(
            home.join("Library")
                .join("Application Support")
                .join("minecraft"),
        );
        dirs.push(home.join(".minecraft"));
    }
    dirs
}

fn count_files_in_dir(dir: &Path, extension: Option<&str>) -> u32 {
    if !dir.exists() || !dir.is_dir() {
        return 0;
    }
    if let Ok(entries) = fs::read_dir(dir) {
        entries
            .flatten()
            .filter(|e| {
                if let Ok(ft) = e.file_type() {
                    if ft.is_file() {
                        if let Some(ext) = extension {
                            return e.path().extension().and_then(|s| s.to_str()) == Some(ext);
                        }
                        return true;
                    }
                }
                false
            })
            .count() as u32
    } else {
        0
    }
}

fn count_subdirectories(dir: &Path) -> u32 {
    if !dir.exists() || !dir.is_dir() {
        return 0;
    }
    if let Ok(entries) = fs::read_dir(dir) {
        entries
            .flatten()
            .filter(|e| e.file_type().map(|ft| ft.is_dir()).unwrap_or(false))
            .count() as u32
    } else {
        0
    }
}

fn copy_dir_all(src: &Path, dst: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_dir() {
            copy_dir_all(&entry.path(), &dst.join(entry.file_name()))?;
        } else {
            fs::copy(entry.path(), dst.join(entry.file_name()))?;
        }
    }
    Ok(())
}

fn parse_vanilla_version_id(version_id: &str) -> (String, ModLoaderType, Option<String>) {
    if let Some(rest) = version_id.strip_prefix("fabric-loader-") {
        if let Some((ldr_ver, mc_ver)) = rest.split_once('-') {
            return (
                mc_ver.to_string(),
                ModLoaderType::Fabric,
                Some(ldr_ver.to_string()),
            );
        }
        return (rest.to_string(), ModLoaderType::Fabric, None);
    }
    if let Some(rest) = version_id.strip_prefix("quilt-loader-") {
        if let Some((ldr_ver, mc_ver)) = rest.split_once('-') {
            return (
                mc_ver.to_string(),
                ModLoaderType::Quilt,
                Some(ldr_ver.to_string()),
            );
        }
        return (rest.to_string(), ModLoaderType::Quilt, None);
    }
    if let Some((mc_ver, ldr_ver)) = version_id.split_once("-forge-") {
        return (
            mc_ver.to_string(),
            ModLoaderType::Forge,
            Some(ldr_ver.to_string()),
        );
    }
    if let Some(rest) = version_id.strip_prefix("forge-") {
        if let Some((mc_ver, ldr_ver)) = rest.split_once('-') {
            return (
                mc_ver.to_string(),
                ModLoaderType::Forge,
                Some(ldr_ver.to_string()),
            );
        }
        return (rest.to_string(), ModLoaderType::Forge, None);
    }
    if let Some(rest) = version_id.strip_prefix("neoforge-") {
        return (
            "1.21.1".to_string(),
            ModLoaderType::NeoForge,
            Some(rest.to_string()),
        );
    }

    (version_id.to_string(), ModLoaderType::Vanilla, None)
}

// -------------------------------------------------------------------------------------------------
// Launcher Parsers
// -------------------------------------------------------------------------------------------------

#[derive(Deserialize)]
struct CurseForgeInstanceJson {
    name: Option<String>,
    #[serde(rename = "gameVersion")]
    game_version: Option<String>,
    #[serde(rename = "baseModLoader")]
    base_mod_loader: Option<CurseForgeModLoader>,
}

#[derive(Deserialize)]
struct CurseForgeModLoader {
    name: Option<String>,
    #[serde(rename = "type")]
    loader_type: Option<u32>,
}

fn scan_curseforge_instances(instances_dir: &Path) -> Vec<ImportableInstance> {
    let mut list = Vec::new();
    if !instances_dir.exists() {
        return list;
    }

    if let Ok(entries) = fs::read_dir(instances_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let json_file = path.join("minecraftinstance.json");
                if json_file.exists() {
                    if let Ok(content) = fs::read_to_string(&json_file) {
                        if let Ok(cfg) = serde_json::from_str::<CurseForgeInstanceJson>(&content) {
                            let name = cfg.name.unwrap_or_else(|| {
                                path.file_name()
                                    .and_then(|s| s.to_str())
                                    .unwrap_or("CurseForge Instance")
                                    .to_string()
                            });
                            let game_version =
                                cfg.game_version.unwrap_or_else(|| "1.20.1".to_string());

                            let mut loader = ModLoaderType::Vanilla;
                            let mut loader_version: Option<String> = None;

                            if let Some(bml) = cfg.base_mod_loader {
                                let ldr_name = bml.name.unwrap_or_default().to_lowercase();
                                let ldr_type = bml.loader_type.unwrap_or(0);

                                if ldr_name.contains("fabric") || ldr_type == 4 {
                                    loader = ModLoaderType::Fabric;
                                    loader_version =
                                        ldr_name.strip_prefix("fabric-").map(|s| s.to_string());
                                } else if ldr_name.contains("quilt") || ldr_type == 5 {
                                    loader = ModLoaderType::Quilt;
                                    loader_version =
                                        ldr_name.strip_prefix("quilt-").map(|s| s.to_string());
                                } else if ldr_name.contains("neoforge") || ldr_type == 6 {
                                    loader = ModLoaderType::NeoForge;
                                    loader_version =
                                        ldr_name.strip_prefix("neoforge-").map(|s| s.to_string());
                                } else if ldr_name.contains("forge") || ldr_type == 1 {
                                    loader = ModLoaderType::Forge;
                                    loader_version =
                                        ldr_name.strip_prefix("forge-").map(|s| s.to_string());
                                }
                            }

                            let mods_count = count_files_in_dir(&path.join("mods"), Some("jar"));
                            let saves_count = count_subdirectories(&path.join("saves"));
                            let screenshots_count =
                                count_files_in_dir(&path.join("screenshots"), Some("png"));

                            list.push(ImportableInstance {
                                id: format!(
                                    "cf-{}",
                                    path.file_name().and_then(|s| s.to_str()).unwrap_or("inst")
                                ),
                                launcher_id: "curseforge".to_string(),
                                name,
                                game_version,
                                loader,
                                loader_version,
                                icon: None,
                                source_path: path.to_string_lossy().to_string(),
                                mods_count,
                                saves_count,
                                screenshots_count,
                            });
                        }
                    }
                }
            }
        }
    }
    list
}

#[derive(Deserialize)]
struct ModrinthProfileJson {
    name: Option<String>,
    game_version: Option<String>,
    loader: Option<String>,
    loader_version: Option<String>,
    icon: Option<String>,
}

fn scan_modrinth_instances(profiles_dir: &Path) -> Vec<ImportableInstance> {
    let mut list = Vec::new();
    if !profiles_dir.exists() {
        return list;
    }

    if let Ok(entries) = fs::read_dir(profiles_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let json_file = if path.join("profile.json").exists() {
                    path.join("profile.json")
                } else if path.join("metadata.json").exists() {
                    path.join("metadata.json")
                } else {
                    continue;
                };

                if let Ok(content) = fs::read_to_string(&json_file) {
                    if let Ok(cfg) = serde_json::from_str::<ModrinthProfileJson>(&content) {
                        let name = cfg.name.unwrap_or_else(|| {
                            path.file_name()
                                .and_then(|s| s.to_str())
                                .unwrap_or("Modrinth Profile")
                                .to_string()
                        });
                        let game_version = cfg.game_version.unwrap_or_else(|| "1.20.1".to_string());
                        let ldr_str = cfg.loader.unwrap_or_default().to_lowercase();
                        let loader = match ldr_str.as_str() {
                            "fabric" => ModLoaderType::Fabric,
                            "quilt" => ModLoaderType::Quilt,
                            "neoforge" => ModLoaderType::NeoForge,
                            "forge" => ModLoaderType::Forge,
                            _ => ModLoaderType::Vanilla,
                        };

                        let mods_count = count_files_in_dir(&path.join("mods"), Some("jar"));
                        let saves_count = count_subdirectories(&path.join("saves"));
                        let screenshots_count =
                            count_files_in_dir(&path.join("screenshots"), Some("png"));

                        list.push(ImportableInstance {
                            id: format!(
                                "mr-{}",
                                path.file_name().and_then(|s| s.to_str()).unwrap_or("inst")
                            ),
                            launcher_id: "modrinth".to_string(),
                            name,
                            game_version,
                            loader,
                            loader_version: cfg.loader_version,
                            icon: cfg.icon,
                            source_path: path.to_string_lossy().to_string(),
                            mods_count,
                            saves_count,
                            screenshots_count,
                        });
                    }
                }
            }
        }
    }
    list
}

#[derive(Deserialize)]
struct ATLauncherInstanceJson {
    name: Option<String>,
    version: Option<String>,
    loader: Option<String>,
    #[serde(rename = "loaderVersion")]
    loader_version: Option<String>,
}

fn scan_atlauncher_instances(instances_dir: &Path) -> Vec<ImportableInstance> {
    let mut list = Vec::new();
    if !instances_dir.exists() {
        return list;
    }

    if let Ok(entries) = fs::read_dir(instances_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let json_file = path.join("instance.json");
                if json_file.exists() {
                    if let Ok(content) = fs::read_to_string(&json_file) {
                        if let Ok(cfg) = serde_json::from_str::<ATLauncherInstanceJson>(&content) {
                            let name = cfg.name.unwrap_or_else(|| {
                                path.file_name()
                                    .and_then(|s| s.to_str())
                                    .unwrap_or("ATLauncher Instance")
                                    .to_string()
                            });
                            let game_version = cfg.version.unwrap_or_else(|| "1.20.1".to_string());
                            let ldr_str = cfg.loader.unwrap_or_default().to_lowercase();
                            let loader = if ldr_str.contains("fabric") {
                                ModLoaderType::Fabric
                            } else if ldr_str.contains("quilt") {
                                ModLoaderType::Quilt
                            } else if ldr_str.contains("neoforge") {
                                ModLoaderType::NeoForge
                            } else if ldr_str.contains("forge") {
                                ModLoaderType::Forge
                            } else {
                                ModLoaderType::Vanilla
                            };

                            let mods_count = count_files_in_dir(&path.join("mods"), Some("jar"));
                            let saves_count = count_subdirectories(&path.join("saves"));
                            let screenshots_count =
                                count_files_in_dir(&path.join("screenshots"), Some("png"));

                            list.push(ImportableInstance {
                                id: format!(
                                    "atl-{}",
                                    path.file_name().and_then(|s| s.to_str()).unwrap_or("inst")
                                ),
                                launcher_id: "atlauncher".to_string(),
                                name,
                                game_version,
                                loader,
                                loader_version: cfg.loader_version,
                                icon: None,
                                source_path: path.to_string_lossy().to_string(),
                                mods_count,
                                saves_count,
                                screenshots_count,
                            });
                        }
                    }
                }
            }
        }
    }
    list
}

#[derive(Deserialize)]
struct VanillaProfilesJson {
    profiles: Option<std::collections::HashMap<String, VanillaProfileEntry>>,
}

#[derive(Deserialize)]
struct VanillaProfileEntry {
    name: Option<String>,
    #[serde(rename = "lastVersionId")]
    last_version_id: Option<String>,
    #[serde(rename = "gameDir")]
    game_dir: Option<String>,
    icon: Option<String>,
}

fn scan_vanilla_instances(minecraft_dir: &Path) -> Vec<ImportableInstance> {
    let mut list = Vec::new();
    let profiles_file = minecraft_dir.join("launcher_profiles.json");
    if !profiles_file.exists() {
        return list;
    }

    if let Ok(content) = fs::read_to_string(&profiles_file) {
        if let Ok(root) = serde_json::from_str::<VanillaProfilesJson>(&content) {
            if let Some(profiles) = root.profiles {
                for (key, profile) in profiles {
                    if let Some(last_ver) = profile.last_version_id {
                        let (game_version, loader, loader_version) =
                            parse_vanilla_version_id(&last_ver);
                        let name = profile.name.unwrap_or_else(|| last_ver.clone());

                        let source_path = if let Some(gd) = profile.game_dir {
                            let p = PathBuf::from(gd);
                            if p.exists() {
                                p
                            } else {
                                minecraft_dir.to_path_buf()
                            }
                        } else {
                            minecraft_dir.to_path_buf()
                        };

                        let mods_count = count_files_in_dir(&source_path.join("mods"), Some("jar"));
                        let saves_count = count_subdirectories(&source_path.join("saves"));
                        let screenshots_count =
                            count_files_in_dir(&source_path.join("screenshots"), Some("png"));

                        list.push(ImportableInstance {
                            id: format!("vanilla-{}", key),
                            launcher_id: "vanilla".to_string(),
                            name,
                            game_version,
                            loader,
                            loader_version,
                            icon: profile.icon,
                            source_path: source_path.to_string_lossy().to_string(),
                            mods_count,
                            saves_count,
                            screenshots_count,
                        });
                    }
                }
            }
        }
    }
    list
}

#[derive(Deserialize)]
struct LegacyVersionJson {
    id: Option<String>,
    jar: Option<String>,
    #[serde(rename = "mainClass")]
    main_class: Option<String>,
    libraries: Option<Vec<LegacyLibraryEntry>>,
}

#[derive(Deserialize)]
struct LegacyLibraryEntry {
    name: Option<String>,
}

fn scan_legacy_instances(game_dir: &Path) -> Vec<ImportableInstance> {
    let mut list = Vec::new();
    if !game_dir.exists() {
        return list;
    }

    // 1. Scan versions folder
    let versions_dir = game_dir.join("versions");
    if versions_dir.exists() {
        if let Ok(entries) = fs::read_dir(&versions_dir) {
            for entry in entries.flatten() {
                let ver_path = entry.path();
                if ver_path.is_dir() {
                    let folder_name = ver_path.file_name().and_then(|s| s.to_str()).unwrap_or("");
                    let json_file = ver_path.join(format!("{}.json", folder_name));

                    let mut game_version = "1.20.1".to_string();
                    let mut loader = ModLoaderType::Vanilla;
                    let mut loader_version: Option<String> = None;

                    if json_file.exists() {
                        if let Ok(content) = fs::read_to_string(&json_file) {
                            if let Ok(v) = serde_json::from_str::<LegacyVersionJson>(&content) {
                                if let Some(j) = v.jar {
                                    game_version = j;
                                } else if let Some(id) = &v.id {
                                    let (gv, _, _) = parse_vanilla_version_id(id);
                                    game_version = gv;
                                }

                                let mc = v.main_class.unwrap_or_default().to_lowercase();
                                if mc.contains("fabric") {
                                    loader = ModLoaderType::Fabric;
                                } else if mc.contains("quilt") {
                                    loader = ModLoaderType::Quilt;
                                } else if mc.contains("neoforge") {
                                    loader = ModLoaderType::NeoForge;
                                } else if mc.contains("minecraftforge") {
                                    loader = ModLoaderType::Forge;
                                }

                                if let Some(libs) = v.libraries {
                                    for lib in libs {
                                        if let Some(lib_name) = lib.name {
                                            if lib_name.starts_with("net.fabricmc:fabric-loader:") {
                                                loader = ModLoaderType::Fabric;
                                                loader_version = lib_name
                                                    .split(':')
                                                    .nth(2)
                                                    .map(|s| s.to_string());
                                            } else if lib_name
                                                .starts_with("net.minecraftforge:forge:")
                                            {
                                                loader = ModLoaderType::Forge;
                                                loader_version = lib_name
                                                    .split(':')
                                                    .nth(2)
                                                    .map(|s| s.to_string());
                                            } else if lib_name
                                                .starts_with("net.neoforged:neoforge:")
                                            {
                                                loader = ModLoaderType::NeoForge;
                                                loader_version = lib_name
                                                    .split(':')
                                                    .nth(2)
                                                    .map(|s| s.to_string());
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    let mods_count = count_files_in_dir(&game_dir.join("mods"), Some("jar"));
                    let saves_count = count_subdirectories(&game_dir.join("saves"));
                    let screenshots_count =
                        count_files_in_dir(&game_dir.join("screenshots"), Some("png"));

                    list.push(ImportableInstance {
                        id: format!("tl-{}", folder_name),
                        launcher_id: "legacylauncher".to_string(),
                        name: format!("{} (LegacyLauncher)", folder_name),
                        game_version,
                        loader,
                        loader_version,
                        icon: None,
                        source_path: game_dir.to_string_lossy().to_string(),
                        mods_count,
                        saves_count,
                        screenshots_count,
                    });
                }
            }
        }
    }

    list
}

// -------------------------------------------------------------------------------------------------
// Public API Functions
// -------------------------------------------------------------------------------------------------

pub fn get_detected_launchers<R: Runtime>(app: &tauri::AppHandle<R>) -> Vec<DetectedLauncher> {
    let mut launchers = Vec::new();

    // 1. CurseForge
    let mut cf_path = String::new();
    let mut cf_count = 0u32;
    let mut cf_found = false;
    for dir in get_curseforge_dirs(app) {
        if dir.exists() {
            cf_found = true;
            cf_path = dir.to_string_lossy().to_string();
            cf_count = scan_curseforge_instances(&dir).len() as u32;
            break;
        }
    }
    launchers.push(DetectedLauncher {
        id: "curseforge".to_string(),
        name: "CurseForge".to_string(),
        icon: "curseforge".to_string(),
        path: cf_path,
        instances_count: cf_count,
        available: cf_found,
    });

    // 2. Modrinth App
    let mut mr_path = String::new();
    let mut mr_count = 0u32;
    let mut mr_found = false;
    for dir in get_modrinth_dirs(app) {
        if dir.exists() {
            mr_found = true;
            mr_path = dir.to_string_lossy().to_string();
            mr_count = scan_modrinth_instances(&dir).len() as u32;
            break;
        }
    }
    launchers.push(DetectedLauncher {
        id: "modrinth".to_string(),
        name: "Modrinth App".to_string(),
        icon: "modrinth".to_string(),
        path: mr_path,
        instances_count: mr_count,
        available: mr_found,
    });

    // 3. ATLauncher
    let mut atl_path = String::new();
    let mut atl_count = 0u32;
    let mut atl_found = false;
    for dir in get_atlauncher_dirs(app) {
        if dir.exists() {
            atl_found = true;
            atl_path = dir.to_string_lossy().to_string();
            atl_count = scan_atlauncher_instances(&dir).len() as u32;
            break;
        }
    }
    launchers.push(DetectedLauncher {
        id: "atlauncher".to_string(),
        name: "ATLauncher".to_string(),
        icon: "atlauncher".to_string(),
        path: atl_path,
        instances_count: atl_count,
        available: atl_found,
    });

    // 4. LegacyLauncher
    let mut tl_path = String::new();
    let mut tl_count = 0u32;
    let mut tl_found = false;
    for dir in get_legacylauncher_dirs(app) {
        if dir.exists() {
            tl_found = true;
            tl_path = dir.to_string_lossy().to_string();
            tl_count = scan_legacy_instances(&dir).len() as u32;
            break;
        }
    }
    launchers.push(DetectedLauncher {
        id: "legacylauncher".to_string(),
        name: "LegacyLauncher".to_string(),
        icon: "legacylauncher".to_string(),
        path: tl_path,
        instances_count: tl_count,
        available: tl_found,
    });

    // 5. Vanilla Minecraft Launcher
    let mut mc_path = String::new();
    let mut mc_count = 0u32;
    let mut mc_found = false;
    for dir in get_vanilla_dirs(app) {
        if dir.exists() {
            mc_found = true;
            mc_path = dir.to_string_lossy().to_string();
            mc_count = scan_vanilla_instances(&dir).len() as u32;
            break;
        }
    }
    launchers.push(DetectedLauncher {
        id: "vanilla".to_string(),
        name: "Vanilla Minecraft Launcher".to_string(),
        icon: "vanilla".to_string(),
        path: mc_path,
        instances_count: mc_count,
        available: mc_found,
    });

    launchers
}

pub fn get_launcher_instances<R: Runtime>(
    app: &tauri::AppHandle<R>,
    launcher_id: &str,
    custom_path: Option<&str>,
) -> Result<Vec<ImportableInstance>, String> {
    match launcher_id {
        "curseforge" => {
            let path = custom_path
                .map(PathBuf::from)
                .or_else(|| get_curseforge_dirs(app).into_iter().find(|p| p.exists()));
            if let Some(p) = path {
                Ok(scan_curseforge_instances(&p))
            } else {
                Ok(Vec::new())
            }
        }
        "modrinth" => {
            let path = custom_path
                .map(PathBuf::from)
                .or_else(|| get_modrinth_dirs(app).into_iter().find(|p| p.exists()));
            if let Some(p) = path {
                Ok(scan_modrinth_instances(&p))
            } else {
                Ok(Vec::new())
            }
        }
        "atlauncher" => {
            let path = custom_path
                .map(PathBuf::from)
                .or_else(|| get_atlauncher_dirs(app).into_iter().find(|p| p.exists()));
            if let Some(p) = path {
                Ok(scan_atlauncher_instances(&p))
            } else {
                Ok(Vec::new())
            }
        }
        "legacylauncher" => {
            let path = custom_path.map(PathBuf::from).or_else(|| {
                get_legacylauncher_dirs(app)
                    .into_iter()
                    .find(|p| p.exists())
            });
            if let Some(p) = path {
                Ok(scan_legacy_instances(&p))
            } else {
                Ok(Vec::new())
            }
        }
        "vanilla" => {
            let path = custom_path
                .map(PathBuf::from)
                .or_else(|| get_vanilla_dirs(app).into_iter().find(|p| p.exists()));
            if let Some(p) = path {
                Ok(scan_vanilla_instances(&p))
            } else {
                Ok(Vec::new())
            }
        }
        _ => Err(format!("Unknown launcher ID: {}", launcher_id)),
    }
}

pub fn detect_custom_instance(path_str: &str) -> Result<ImportableInstance, String> {
    let path = PathBuf::from(path_str);
    if !path.exists() {
        return Err(format!("Path does not exist: {}", path_str));
    }

    // Check CurseForge
    if path.join("minecraftinstance.json").exists() {
        let list = scan_curseforge_instances(path.parent().unwrap_or(&path));
        if let Some(inst) = list.into_iter().find(|i| i.source_path == path_str) {
            return Ok(inst);
        }
    }

    // Check Modrinth
    if path.join("profile.json").exists() || path.join("metadata.json").exists() {
        let list = scan_modrinth_instances(path.parent().unwrap_or(&path));
        if let Some(inst) = list.into_iter().find(|i| i.source_path == path_str) {
            return Ok(inst);
        }
    }

    // Check ATLauncher
    if path.join("instance.json").exists() {
        let list = scan_atlauncher_instances(path.parent().unwrap_or(&path));
        if let Some(inst) = list.into_iter().find(|i| i.source_path == path_str) {
            return Ok(inst);
        }
    }

    // Check Legacy
    if path.join("versions").exists() {
        let list = scan_legacy_instances(&path);
        if let Some(inst) = list.into_iter().next() {
            return Ok(inst);
        }
    }

    // Raw minecraft directory fallback
    let name = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("Custom Instance")
        .to_string();

    let mods_count = count_files_in_dir(&path.join("mods"), Some("jar"));
    let saves_count = count_subdirectories(&path.join("saves"));
    let screenshots_count = count_files_in_dir(&path.join("screenshots"), Some("png"));

    Ok(ImportableInstance {
        id: format!("custom-{}", uuid::Uuid::new_v4()),
        launcher_id: "custom".to_string(),
        name,
        game_version: "1.20.1".to_string(),
        loader: if mods_count > 0 {
            ModLoaderType::Fabric
        } else {
            ModLoaderType::Vanilla
        },
        loader_version: None,
        icon: None,
        source_path: path_str.to_string(),
        mods_count,
        saves_count,
        screenshots_count,
    })
}

pub fn import_instance<R: Runtime>(
    app: &tauri::AppHandle<R>,
    options: ImportInstanceOptions,
) -> Result<ImportReport, String> {
    let source_dir = PathBuf::from(&options.source_path);
    if !source_dir.exists() {
        return Err(format!(
            "Source directory does not exist: {}",
            options.source_path
        ));
    }

    // 1. Create instance in Ingot
    let instance = crate::minecraft::instance::create_instance(
        app,
        options.name.clone(),
        options.game_version.clone(),
        options.loader.clone(),
        options.loader_version.clone(),
    )?;

    let dest_dir = crate::minecraft::instance::get_instance_dir(app, &instance.id)?;

    let mut mods_copied: u32 = 0;
    let mut saves_copied: u32 = 0;

    let mut handled_entries: std::collections::HashSet<String> = std::collections::HashSet::new();

    // 2. Copy mods
    handled_entries.insert("mods".to_string());
    if options.copy_mods {
        let src_mods = source_dir.join("mods");
        let dst_mods = dest_dir.join("mods");
        if src_mods.exists() && src_mods.is_dir() {
            let _ = fs::create_dir_all(&dst_mods);
            if let Ok(entries) = fs::read_dir(&src_mods) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_file() {
                        let fname = entry.file_name();
                        if fs::copy(&path, dst_mods.join(fname)).is_ok() {
                            mods_copied += 1;
                        }
                    }
                }
            }
        }
    }

    // 3. Copy config & defaultconfigs
    handled_entries.insert("config".to_string());
    handled_entries.insert("defaultconfigs".to_string());
    if options.copy_configs {
        let src_config = source_dir.join("config");
        let dst_config = dest_dir.join("config");
        if src_config.exists() && src_config.is_dir() {
            let _ = copy_dir_all(&src_config, &dst_config);
        }
        let src_def = source_dir.join("defaultconfigs");
        let dst_def = dest_dir.join("defaultconfigs");
        if src_def.exists() && src_def.is_dir() {
            let _ = copy_dir_all(&src_def, &dst_def);
        }
    }

    // 4. Copy saves (worlds)
    handled_entries.insert("saves".to_string());
    if options.copy_saves {
        let src_saves = source_dir.join("saves");
        let dst_saves = dest_dir.join("saves");
        if src_saves.exists() && src_saves.is_dir() {
            let _ = fs::create_dir_all(&dst_saves);
            if let Ok(entries) = fs::read_dir(&src_saves) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        let fname = entry.file_name();
                        if copy_dir_all(&path, &dst_saves.join(fname)).is_ok() {
                            saves_copied += 1;
                        }
                    }
                }
            }
        }
    }

    // 5. Copy resourcepacks & shaderpacks
    handled_entries.insert("resourcepacks".to_string());
    handled_entries.insert("shaderpacks".to_string());
    if options.copy_resource_packs {
        let src_rp = source_dir.join("resourcepacks");
        let dst_rp = dest_dir.join("resourcepacks");
        if src_rp.exists() && src_rp.is_dir() {
            let _ = copy_dir_all(&src_rp, &dst_rp);
        }
        let src_sp = source_dir.join("shaderpacks");
        let dst_sp = dest_dir.join("shaderpacks");
        if src_sp.exists() && src_sp.is_dir() {
            let _ = copy_dir_all(&src_sp, &dst_sp);
        }
    }

    // 6. Copy options.txt & optionsshaders.txt
    handled_entries.insert("options.txt".to_string());
    handled_entries.insert("optionsshaders.txt".to_string());
    if options.copy_options {
        let src_opt = source_dir.join("options.txt");
        if src_opt.exists() {
            let _ = fs::copy(&src_opt, dest_dir.join("options.txt"));
        }
        let src_opt_sh = source_dir.join("optionsshaders.txt");
        if src_opt_sh.exists() {
            let _ = fs::copy(&src_opt_sh, dest_dir.join("optionsshaders.txt"));
        }
    }

    // 7. Copy servers.dat
    handled_entries.insert("servers.dat".to_string());
    if options.copy_servers {
        let src_srv = source_dir.join("servers.dat");
        if src_srv.exists() {
            let _ = fs::copy(&src_srv, dest_dir.join("servers.dat"));
        }
    }

    // 8. Copy screenshots
    handled_entries.insert("screenshots".to_string());
    if options.copy_screenshots {
        let src_ss = source_dir.join("screenshots");
        let dst_ss = dest_dir.join("screenshots");
        if src_ss.exists() && src_ss.is_dir() {
            let _ = copy_dir_all(&src_ss, &dst_ss);
        }
    }

    // 9. Launcher & runtime directories to ignore (managed globally or temporary caches)
    const IGNORED_ENTRIES: &[&str] = &[
        "assets",
        "libraries",
        "versions",
        "natives",
        "jre",
        "java",
        "runtime",
        "launcher",
        "bin",
        ".fabric",
        ".quilt",
        ".mixin.out",
        ".gradle",
        ".cache",
        "logs",
        ".tlauncher",
        ".curseforge",
        ".modrinth",
        ".atlauncher",
        ".lock",
        "running.lock",
        ".ds_store",
        "thumbs.db",
    ];

    // 10. Automatically copy ALL other instance content (schematics, map data,
    // quests, custom mod folders, player data, etc.) so nothing is ever lost!
    if options.copy_extra_data {
        if let Ok(entries) = fs::read_dir(&source_dir) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                let name_lower = name.to_lowercase();

                // Skip already explicitly handled entries (e.g. mods, saves, options, screenshots)
                if handled_entries.contains(&name_lower) {
                    continue;
                }

                // Skip known launcher / runtime junk
                if IGNORED_ENTRIES.contains(&name_lower.as_str()) {
                    continue;
                }

                let path = entry.path();
                let target_path = dest_dir.join(&name);

                if path.is_dir() {
                    let _ = copy_dir_all(&path, &target_path);
                } else if path.is_file() {
                    let _ = fs::copy(&path, &target_path);
                }
            }
        }
    }

    Ok(ImportReport {
        success: true,
        instance_id: instance.id,
        name: options.name,
        mods_copied,
        saves_copied,
        message: format!(
            "Successfully imported instance with {mods_copied} mods and {saves_copied} worlds."
        ),
    })
}
