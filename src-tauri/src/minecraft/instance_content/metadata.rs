use std::fs::File;
use std::io::Read;
use std::path::Path;

use base64::Engine;
use serde_json::Value;

use super::models::{ContentDependency, ContentMetadata};

const MAX_ICON_BYTES: usize = 256 * 1024; // 256 KB max for embedded base64 icons

pub type RawJarTuple = (String, Option<String>, Option<String>, Vec<String>);

pub fn read_content_metadata(path: &Path, is_pack: bool) -> Option<ContentMetadata> {
    if is_pack {
        read_pack_metadata(path)
    } else {
        read_jar_metadata(path)
    }
}

pub fn read_raw_jar_metadata(path: &Path) -> Option<RawJarTuple> {
    let meta = read_jar_metadata(path)?;
    Some((meta.name, meta.version, meta.description, meta.authors))
}

fn read_zip_text<R: Read + std::io::Seek>(
    zip: &mut zip::ZipArchive<R>,
    name: &str,
) -> Option<String> {
    let mut entry = zip.by_name(name).ok()?;
    let mut text = String::new();
    entry.read_to_string(&mut text).ok()?;
    Some(text)
}

fn read_zip_icon<R: Read + std::io::Seek>(
    zip: &mut zip::ZipArchive<R>,
    icon_path: &str,
) -> Option<String> {
    let clean = icon_path.trim_start_matches('/');
    let mut entry = zip.by_name(clean).ok()?;
    if entry.size() > MAX_ICON_BYTES as u64 {
        return None;
    }
    let mut bytes = Vec::new();
    entry.read_to_end(&mut bytes).ok()?;
    let encoded = base64::engine::general_purpose::STANDARD.encode(&bytes);
    Some(format!("data:image/png;base64,{encoded}"))
}

fn read_jar_metadata(path: &Path) -> Option<ContentMetadata> {
    let file = File::open(path).ok()?;
    let mut zip = zip::ZipArchive::new(file).ok()?;

    // 1. Fabric mod
    if let Some(json) = read_zip_text(&mut zip, "fabric.mod.json")
        .and_then(|t| serde_json::from_str::<Value>(&t).ok())
    {
        let id = json.get("id").and_then(Value::as_str).map(str::to_string);
        let name = json
            .get("name")
            .and_then(Value::as_str)
            .map(str::to_string)
            .or_else(|| id.clone())?;

        let version = json
            .get("version")
            .and_then(Value::as_str)
            .map(str::to_string);
        let description = json
            .get("description")
            .and_then(Value::as_str)
            .map(str::to_string);

        let authors = json
            .get("authors")
            .and_then(Value::as_array)
            .map(|a| {
                a.iter()
                    .filter_map(|x| {
                        x.as_str()
                            .map(str::to_string)
                            .or_else(|| x.get("name")?.as_str().map(str::to_string))
                    })
                    .collect()
            })
            .unwrap_or_default();

        let icon_data_url = json
            .get("icon")
            .and_then(Value::as_str)
            .and_then(|p| read_zip_icon(&mut zip, p))
            .or_else(|| read_zip_icon(&mut zip, "assets/icon.png"));

        let mut dependencies = Vec::new();
        if let Some(depends) = json.get("depends").and_then(Value::as_object) {
            for dep_id in depends.keys() {
                if !matches!(
                    dep_id.as_str(),
                    "minecraft" | "fabricloader" | "fabric" | "java"
                ) {
                    dependencies.push(ContentDependency {
                        id: dep_id.clone(),
                        name: None,
                        required: true,
                    });
                }
            }
        }

        return Some(ContentMetadata {
            name,
            mod_id: id,
            version,
            description,
            authors,
            icon_data_url,
            dependencies,
        });
    }

    // 2. Quilt mod
    if let Some(json) = read_zip_text(&mut zip, "quilt.mod.json")
        .and_then(|t| serde_json::from_str::<Value>(&t).ok())
    {
        let loader = json.get("quilt_loader")?;
        let meta = loader.get("metadata").cloned().unwrap_or(Value::Null);
        let id = loader.get("id").and_then(Value::as_str).map(str::to_string);
        let name = meta
            .get("name")
            .and_then(Value::as_str)
            .map(str::to_string)
            .or_else(|| id.clone())?;

        let version = loader
            .get("version")
            .and_then(Value::as_str)
            .map(str::to_string);
        let description = meta
            .get("description")
            .and_then(Value::as_str)
            .map(str::to_string);

        let authors = meta
            .get("contributors")
            .and_then(Value::as_object)
            .map(|c| c.keys().cloned().collect())
            .unwrap_or_default();

        let icon_data_url = meta
            .get("icon")
            .and_then(Value::as_str)
            .and_then(|p| read_zip_icon(&mut zip, p));

        let mut dependencies = Vec::new();
        if let Some(depends) = loader.get("depends").and_then(Value::as_array) {
            for item in depends {
                if let Some(dep_id) = item
                    .as_str()
                    .or_else(|| item.get("id").and_then(Value::as_str))
                {
                    if !matches!(dep_id, "minecraft" | "quilt_loader" | "java") {
                        dependencies.push(ContentDependency {
                            id: dep_id.to_string(),
                            name: None,
                            required: true,
                        });
                    }
                }
            }
        }

        return Some(ContentMetadata {
            name,
            mod_id: id,
            version,
            description,
            authors,
            icon_data_url,
            dependencies,
        });
    }

    // 3. Forge (mods.toml) / NeoForge (neoforge.mods.toml)
    let manifest_mf = read_zip_text(&mut zip, "META-INF/MANIFEST.MF");
    let forge_doc = read_zip_text(&mut zip, "META-INF/neoforge.mods.toml")
        .or_else(|| read_zip_text(&mut zip, "META-INF/mods.toml"));

    if let Some(doc) = forge_doc.and_then(|t| t.parse::<toml_edit::DocumentMut>().ok()) {
        let first = doc.get("mods")?.as_array_of_tables()?.iter().next()?;
        let text = |key: &str| {
            first
                .get(key)
                .and_then(|v| v.as_str())
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
        };

        let mod_id = text("modId");
        let name = text("displayName").or_else(|| mod_id.clone())?;
        let version = text("version").and_then(|v| {
            if !v.starts_with("${") {
                return Some(v);
            }
            manifest_mf.as_deref()?.lines().find_map(|l| {
                l.strip_prefix("Implementation-Version:")
                    .map(|ver| ver.trim().to_string())
            })
        });

        let authors = doc
            .get("authors")
            .and_then(|v| v.as_str())
            .or_else(|| first.get("authors").and_then(|v| v.as_str()));
        let authors = authors
            .map(|a| {
                a.split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect()
            })
            .unwrap_or_default();

        let logo_file = text("logoFile");
        let icon_data_url = logo_file
            .as_deref()
            .and_then(|p| read_zip_icon(&mut zip, p));

        let mut dependencies = Vec::new();
        if let Some(mod_id_str) = &mod_id {
            let dep_key = format!("dependencies.{mod_id_str}");
            if let Some(dep_arr) = doc.get(&dep_key).and_then(|v| v.as_array_of_tables()) {
                for dep in dep_arr.iter() {
                    let dep_id = dep
                        .get("modId")
                        .and_then(|v| v.as_str())
                        .map(str::to_string);
                    let mandatory = dep
                        .get("mandatory")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(true);
                    if let Some(did) = dep_id {
                        if !matches!(did.as_str(), "minecraft" | "forge" | "neoforge") {
                            dependencies.push(ContentDependency {
                                id: did,
                                name: None,
                                required: mandatory,
                            });
                        }
                    }
                }
            }
        }

        return Some(ContentMetadata {
            name,
            mod_id,
            version,
            description: text("description"),
            authors,
            icon_data_url,
            dependencies,
        });
    }

    // 4. Legacy Forge mcmod.info
    if let Some(text) = read_zip_text(&mut zip, "mcmod.info") {
        if let Ok(info_val) = serde_json::from_str::<Value>(&text) {
            let item = info_val.as_array().and_then(|arr| arr.first()).or_else(|| {
                info_val
                    .get("modList")
                    .and_then(Value::as_array)
                    .and_then(|arr| arr.first())
            });

            if let Some(first) = item {
                let mod_id = first
                    .get("modid")
                    .and_then(Value::as_str)
                    .map(str::to_string);
                let name = first
                    .get("name")
                    .and_then(Value::as_str)
                    .map(str::to_string)
                    .or_else(|| mod_id.clone())?;

                let version = first
                    .get("version")
                    .and_then(Value::as_str)
                    .map(str::to_string);
                let description = first
                    .get("description")
                    .and_then(Value::as_str)
                    .map(str::to_string);

                let authors = first
                    .get("authorList")
                    .and_then(Value::as_array)
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(str::to_string))
                            .collect()
                    })
                    .unwrap_or_default();

                let logo_file = first.get("logoFile").and_then(Value::as_str);
                let icon_data_url = logo_file.and_then(|p| read_zip_icon(&mut zip, p));

                return Some(ContentMetadata {
                    name,
                    mod_id,
                    version,
                    description,
                    authors,
                    icon_data_url,
                    dependencies: Vec::new(),
                });
            }
        }
    }

    // 5. Bukkit / Spigot / Paper plugins
    let yaml = read_zip_text(&mut zip, "paper-plugin.yml")
        .or_else(|| read_zip_text(&mut zip, "plugin.yml"))?;
    let top = |key: &str| -> Option<String> {
        yaml.lines().find_map(|line| {
            let rest = line.strip_prefix(key)?.strip_prefix(':')?;
            let value = rest.trim().trim_matches(|c| c == '"' || c == '\'').trim();
            (!value.is_empty() && value != "|" && value != ">").then(|| value.to_string())
        })
    };
    let name = top("name")?;
    let mut authors: Vec<String> = top("author").into_iter().collect();
    if let Some(list) = top("authors") {
        authors.extend(
            list.trim_matches(|c| c == '[' || c == ']')
                .split(',')
                .map(|a| a.trim().trim_matches(|c| c == '"' || c == '\'').to_string())
                .filter(|a| !a.is_empty()),
        );
    }

    Some(ContentMetadata {
        name,
        mod_id: None,
        version: top("version"),
        description: top("description"),
        authors,
        icon_data_url: None,
        dependencies: Vec::new(),
    })
}

fn read_pack_metadata(path: &Path) -> Option<ContentMetadata> {
    if path.is_dir() {
        let mcmeta_path = path.join("pack.mcmeta");
        let mcmeta_text = std::fs::read_to_string(&mcmeta_path).ok()?;
        let pack_val: Value = serde_json::from_str(&mcmeta_text).ok()?;
        let pack = pack_val.get("pack")?;

        let description = pack.get("description").and_then(|d| {
            d.as_str()
                .map(str::to_string)
                .or_else(|| d.get("text").and_then(Value::as_str).map(str::to_string))
        });

        let icon_path = path.join("pack.png");
        let icon_data_url = std::fs::read(&icon_path)
            .ok()
            .filter(|b| b.len() <= MAX_ICON_BYTES)
            .map(|b| {
                let encoded = base64::engine::general_purpose::STANDARD.encode(&b);
                format!("data:image/png;base64,{encoded}")
            });

        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "Pack".to_string());

        Some(ContentMetadata {
            name,
            mod_id: None,
            version: None,
            description,
            authors: Vec::new(),
            icon_data_url,
            dependencies: Vec::new(),
        })
    } else {
        let file = File::open(path).ok()?;
        let mut zip = zip::ZipArchive::new(file).ok()?;

        let description = {
            let mut entry = zip.by_name("pack.mcmeta").ok()?;
            let mut text = String::new();
            entry.read_to_string(&mut text).ok()?;
            let pack_val: Value = serde_json::from_str(&text).ok()?;
            let pack = pack_val.get("pack")?;
            pack.get("description").and_then(|d| {
                d.as_str()
                    .map(str::to_string)
                    .or_else(|| d.get("text").and_then(Value::as_str).map(str::to_string))
            })
        };

        let icon_data_url = {
            if let Ok(mut entry) = zip.by_name("pack.png") {
                if entry.size() <= MAX_ICON_BYTES as u64 {
                    let mut bytes = Vec::new();
                    if entry.read_to_end(&mut bytes).is_ok() {
                        let encoded = base64::engine::general_purpose::STANDARD.encode(&bytes);
                        Some(format!("data:image/png;base64,{encoded}"))
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            }
        };

        let name = path
            .file_name()
            .map(|n| {
                let s = n.to_string_lossy();
                s.strip_suffix(".zip")
                    .or_else(|| s.strip_suffix(".zip.disabled"))
                    .unwrap_or(&s)
                    .to_string()
            })
            .unwrap_or_else(|| "Pack".to_string());

        Some(ContentMetadata {
            name,
            mod_id: None,
            version: None,
            description,
            authors: Vec::new(),
            icon_data_url,
            dependencies: Vec::new(),
        })
    }
}
