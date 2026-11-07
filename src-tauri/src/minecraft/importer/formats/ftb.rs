//! FTB App instance scanner and manifest parser.
use super::{count_instance_stats, ImportableInstance};
use crate::minecraft::instance::ModLoaderType;
use serde::Deserialize;
use std::fs;
use std::path::Path;

#[derive(Deserialize)]
struct FtbInstanceJson {
    uuid: Option<String>,
    name: Option<String>,
    #[serde(alias = "gameVersion")]
    #[serde(rename = "mcVersion")]
    mc_version: Option<String>,
    #[serde(alias = "loader")]
    #[serde(rename = "modLoader")]
    mod_loader: Option<String>,
    #[serde(alias = "loaderVersion")]
    #[serde(rename = "modLoaderVersion")]
    mod_loader_version: Option<String>,
    icon: Option<String>,
}

pub(crate) fn scan_ftb_instances(instances_dir: &Path) -> Vec<ImportableInstance> {
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
                        if let Ok(cfg) = serde_json::from_str::<FtbInstanceJson>(&content) {
                            let folder_name =
                                path.file_name().and_then(|s| s.to_str()).unwrap_or("inst");

                            let id = cfg
                                .uuid
                                .filter(|s| !s.trim().is_empty())
                                .unwrap_or_else(|| folder_name.to_string());

                            let name = cfg.name.unwrap_or_else(|| folder_name.to_string());
                            let game_version =
                                cfg.mc_version.unwrap_or_else(|| "1.20.1".to_string());

                            let ldr_str = cfg.mod_loader.unwrap_or_default().to_lowercase();
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

                            let (mods_count, saves_count, screenshots_count) =
                                count_instance_stats(&path);

                            list.push(ImportableInstance {
                                id: format!("ftb-{id}"),
                                launcher_id: "ftb".to_string(),
                                name,
                                game_version,
                                loader,
                                loader_version: cfg.mod_loader_version,
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
    }
    list
}
