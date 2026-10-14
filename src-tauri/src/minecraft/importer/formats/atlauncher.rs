//! ATLauncher instance discovery and manifest parser.
use super::{count_instance_stats, ImportableInstance};
use crate::minecraft::instance::ModLoaderType;
use serde::Deserialize;
use std::fs;
use std::path::Path;

#[derive(Deserialize)]
struct ATLauncherInstanceJson {
    name: Option<String>,
    version: Option<String>,
    loader: Option<String>,
    #[serde(rename = "loaderVersion")]
    loader_version: Option<String>,
}

pub(crate) fn scan_atlauncher_instances(instances_dir: &Path) -> Vec<ImportableInstance> {
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

                            let (mods_count, saves_count, screenshots_count) =
                                count_instance_stats(&path);

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
