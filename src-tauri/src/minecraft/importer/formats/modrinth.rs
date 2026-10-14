//! Modrinth App profile discovery and manifest parser.
use super::{count_instance_stats, ImportableInstance};
use crate::minecraft::instance::ModLoaderType;
use serde::Deserialize;
use std::fs;
use std::path::Path;

#[derive(Deserialize)]
struct ModrinthProfileJson {
    name: Option<String>,
    game_version: Option<String>,
    loader: Option<String>,
    loader_version: Option<String>,
    icon: Option<String>,
}

pub(crate) fn scan_modrinth_instances(profiles_dir: &Path) -> Vec<ImportableInstance> {
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

                        let (mods_count, saves_count, screenshots_count) =
                            count_instance_stats(&path);

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
