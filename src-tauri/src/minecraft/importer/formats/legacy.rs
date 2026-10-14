//! LegacyLauncher / TLauncher instance discovery and version JSON scanner.
use super::{count_instance_stats, ImportableInstance};
use crate::minecraft::importer::version_id::parse_vanilla_version_id;
use crate::minecraft::instance::ModLoaderType;
use serde::Deserialize;
use std::fs;
use std::path::Path;

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

pub(crate) fn scan_legacy_instances(game_dir: &Path) -> Vec<ImportableInstance> {
    let mut list = Vec::new();
    if !game_dir.exists() {
        return list;
    }

    let versions_dir = game_dir.join("versions");
    if versions_dir.exists() {
        if let Ok(entries) = fs::read_dir(&versions_dir) {
            for entry in entries.flatten() {
                let ver_path = entry.path();
                if ver_path.is_dir() {
                    let folder_name = ver_path.file_name().and_then(|s| s.to_str()).unwrap_or("");
                    let json_file = ver_path.join(format!("{folder_name}.json"));

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

                    let (mods_count, saves_count, screenshots_count) =
                        count_instance_stats(game_dir);

                    list.push(ImportableInstance {
                        id: format!("tl-{folder_name}"),
                        launcher_id: "legacylauncher".to_string(),
                        name: format!("{folder_name} (LegacyLauncher)"),
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
