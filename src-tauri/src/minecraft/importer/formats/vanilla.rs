//! Vanilla Minecraft Launcher profiles scanner.
use super::{count_instance_stats, ImportableInstance};
use crate::minecraft::importer::version_id::parse_vanilla_version_id;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Deserialize)]
struct VanillaProfilesJson {
    profiles: Option<HashMap<String, VanillaProfileEntry>>,
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

pub(crate) fn scan_vanilla_instances(minecraft_dir: &Path) -> Vec<ImportableInstance> {
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

                        let (mods_count, saves_count, screenshots_count) =
                            count_instance_stats(&source_path);

                        list.push(ImportableInstance {
                            id: format!("vanilla-{key}"),
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
