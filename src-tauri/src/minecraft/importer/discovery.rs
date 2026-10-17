//! Platform filesystem discovery and launcher detection.
use super::{
    formats::{
        count_instance_stats, scan_atlauncher_instances, scan_curseforge_instances,
        scan_legacy_instances, scan_modrinth_instances, scan_vanilla_instances,
    },
    models::{DetectedLauncher, ImportError, ImportableInstance},
};
use crate::minecraft::instance::ModLoaderType;
use std::path::PathBuf;

pub(crate) struct DiscoveryRoots {
    pub(crate) app_data: Option<PathBuf>,
    pub(crate) home: Option<PathBuf>,
}

pub(crate) fn get_curseforge_dirs(roots: &DiscoveryRoots) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(home) = roots.home.clone() {
        dirs.push(home.join("curseforge").join("minecraft").join("Instances"));
        dirs.push(home.join("CurseForge").join("minecraft").join("Instances"));
        dirs.push(
            home.join("Documents")
                .join("curseforge")
                .join("minecraft")
                .join("Instances"),
        );
        dirs.push(
            home.join("Documents")
                .join("CurseForge")
                .join("minecraft")
                .join("Instances"),
        );
    }
    dirs
}

pub(crate) fn get_modrinth_dirs(roots: &DiscoveryRoots) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(appdata) = roots.app_data.clone() {
        dirs.push(appdata.join("com.modrinth.theseus").join("profiles"));
        dirs.push(appdata.join("Modrinth App").join("profiles"));
    }
    if let Some(home) = roots.home.clone() {
        dirs.push(
            home.join(".local")
                .join("share")
                .join("com.modrinth.theseus")
                .join("profiles"),
        );
        dirs.push(
            home.join("Library")
                .join("Application Support")
                .join("com.modrinth.theseus")
                .join("profiles"),
        );
    }
    dirs
}

pub(crate) fn get_atlauncher_dirs(roots: &DiscoveryRoots) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(appdata) = roots.app_data.clone() {
        dirs.push(appdata.join("ATLauncher").join("instances"));
    }
    if let Some(home) = roots.home.clone() {
        dirs.push(home.join(".atlauncher").join("instances"));
        dirs.push(home.join("ATLauncher").join("instances"));
        dirs.push(
            home.join("Library")
                .join("Application Support")
                .join("ATLauncher")
                .join("instances"),
        );
    }
    dirs
}

pub(crate) fn get_legacylauncher_dirs(roots: &DiscoveryRoots) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(appdata) = roots.app_data.clone() {
        dirs.push(
            appdata
                .join(".tlauncher")
                .join("legacy")
                .join("Minecraft")
                .join("game"),
        );
        dirs.push(appdata.join(".minecraft"));
    }
    if let Some(home) = roots.home.clone() {
        dirs.push(
            home.join(".tlauncher")
                .join("legacy")
                .join("Minecraft")
                .join("game"),
        );
        dirs.push(home.join(".minecraft"));
    }
    dirs
}

pub(crate) fn get_vanilla_dirs(roots: &DiscoveryRoots) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(appdata) = roots.app_data.clone() {
        dirs.push(appdata.join(".minecraft"));
    }
    if let Some(home) = roots.home.clone() {
        dirs.push(home.join(".minecraft"));
        dirs.push(
            home.join("Library")
                .join("Application Support")
                .join("minecraft"),
        );
    }
    dirs
}

pub fn get_detected_launchers(roots: &DiscoveryRoots) -> Vec<DetectedLauncher> {
    let mut launchers = Vec::new();

    // 1. CurseForge
    let mut cf_path = String::new();
    let mut cf_count = 0u32;
    let mut cf_found = false;
    for dir in get_curseforge_dirs(roots) {
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
    for dir in get_modrinth_dirs(roots) {
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
    for dir in get_atlauncher_dirs(roots) {
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
    for dir in get_legacylauncher_dirs(roots) {
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
    for dir in get_vanilla_dirs(roots) {
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

pub fn get_launcher_instances(
    roots: &DiscoveryRoots,
    launcher_id: &str,
    custom_path: Option<&str>,
) -> Result<Vec<ImportableInstance>, ImportError> {
    match launcher_id {
        "curseforge" => {
            let path = custom_path
                .map(PathBuf::from)
                .or_else(|| get_curseforge_dirs(roots).into_iter().find(|p| p.exists()));
            if let Some(p) = path {
                Ok(scan_curseforge_instances(&p))
            } else {
                Ok(Vec::new())
            }
        }
        "modrinth" => {
            let path = custom_path
                .map(PathBuf::from)
                .or_else(|| get_modrinth_dirs(roots).into_iter().find(|p| p.exists()));
            if let Some(p) = path {
                Ok(scan_modrinth_instances(&p))
            } else {
                Ok(Vec::new())
            }
        }
        "atlauncher" => {
            let path = custom_path
                .map(PathBuf::from)
                .or_else(|| get_atlauncher_dirs(roots).into_iter().find(|p| p.exists()));
            if let Some(p) = path {
                Ok(scan_atlauncher_instances(&p))
            } else {
                Ok(Vec::new())
            }
        }
        "legacylauncher" => {
            let path = custom_path.map(PathBuf::from).or_else(|| {
                get_legacylauncher_dirs(roots)
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
                .or_else(|| get_vanilla_dirs(roots).into_iter().find(|p| p.exists()));
            if let Some(p) = path {
                Ok(scan_vanilla_instances(&p))
            } else {
                Ok(Vec::new())
            }
        }
        _ => Err(ImportError::UnknownLauncher(launcher_id.to_string())),
    }
}

pub fn detect_custom_instance(path_str: &str) -> Result<ImportableInstance, ImportError> {
    let path = PathBuf::from(path_str);
    if !path.exists() {
        return Err(ImportError::SourceNotFound(path_str.to_string()));
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

    let (mods_count, saves_count, screenshots_count) = count_instance_stats(&path);

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
