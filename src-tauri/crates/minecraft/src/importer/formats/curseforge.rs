//! CurseForge instance discovery and manifest parser.
use super::{count_instance_stats, ImportableInstance};
use crate::minecraft::instance::ModLoaderType;
use serde::Deserialize;
use std::fs;
use std::path::Path;

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

pub(crate) fn scan_curseforge_instances(instances_dir: &Path) -> Vec<ImportableInstance> {
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

                            let (mods_count, saves_count, screenshots_count) =
                                count_instance_stats(&path);

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
