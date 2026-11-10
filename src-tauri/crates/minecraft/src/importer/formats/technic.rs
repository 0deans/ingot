//! Technic Launcher instance scanner and parser.
use super::{count_instance_stats, ImportableInstance};
use crate::minecraft::{importer::version_id::parse_vanilla_version_id, instance::ModLoaderType};
use serde::Deserialize;
use std::fs;
use std::io::Read;
use std::path::Path;
use zip::ZipArchive;

#[derive(Deserialize)]
struct TechnicVersionJson {
    id: Option<String>,
}

fn read_version_id_from_jar(jar_path: &Path) -> Option<String> {
    let file = fs::File::open(jar_path).ok()?;
    let mut zip = ZipArchive::new(file).ok()?;
    let mut entry = zip.by_name("version.json").ok()?;
    let mut content = Vec::new();
    if let Err(err) = entry.read_to_end(&mut content) {
        if !err
            .to_string()
            .to_ascii_lowercase()
            .contains("invalid checksum")
        {
            return None;
        }
    }
    let vj = serde_json::from_slice::<TechnicVersionJson>(&content).ok()?;
    vj.id.filter(|s| !s.trim().is_empty())
}

pub(crate) fn scan_technic_instances(modpacks_dir: &Path) -> Vec<ImportableInstance> {
    let mut list = Vec::new();
    if !modpacks_dir.exists() {
        return list;
    }

    if let Ok(entries) = fs::read_dir(modpacks_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let bin_dir = path.join("bin");
                let mods_dir = path.join("mods");
                if !bin_dir.exists() && !mods_dir.exists() {
                    continue;
                }

                let slug = path
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("technic-pack")
                    .to_string();

                let mut game_version = "1.20.1".to_string();
                let mut loader = ModLoaderType::Forge;
                let mut loader_version = None;

                let mut found = false;
                let bin_version = bin_dir.join("version.json");
                let root_version = path.join("version.json");
                for candidate in [&bin_version, &root_version] {
                    if candidate.exists() {
                        if let Ok(content) = fs::read_to_string(candidate) {
                            if let Ok(vj) = serde_json::from_str::<TechnicVersionJson>(&content) {
                                if let Some(id) = vj.id.filter(|s| !s.trim().is_empty()) {
                                    let (gv, ldr, lv) = parse_vanilla_version_id(&id);
                                    if !gv.is_empty() {
                                        game_version = gv;
                                    }
                                    loader = ldr;
                                    loader_version = lv;
                                    found = true;
                                    break;
                                }
                            }
                        }
                    }
                }

                if !found {
                    let modpack_jar = bin_dir.join("modpack.jar");
                    if modpack_jar.exists() {
                        if let Some(id) = read_version_id_from_jar(&modpack_jar) {
                            let (gv, ldr, lv) = parse_vanilla_version_id(&id);
                            if !gv.is_empty() {
                                game_version = gv;
                            }
                            loader = ldr;
                            loader_version = lv;
                        }
                    }
                }

                let icon = if path.join("icon.png").exists() {
                    Some(path.join("icon.png").to_string_lossy().to_string())
                } else {
                    None
                };

                let name = prettify_slug(&slug);
                let (mods_count, saves_count, screenshots_count) = count_instance_stats(&path);

                list.push(ImportableInstance {
                    id: format!("technic-{slug}"),
                    launcher_id: "technic".to_string(),
                    name,
                    game_version,
                    loader,
                    loader_version,
                    icon,
                    source_path: path.to_string_lossy().to_string(),
                    mods_count,
                    saves_count,
                    screenshots_count,
                });
            }
        }
    }
    list
}

fn prettify_slug(slug: &str) -> String {
    slug.split(['-', '_'])
        .filter(|part| !part.is_empty())
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}
