use std::fs::File;
use std::io::Read;
use std::path::Path;

use serde_json::Value;

pub type RawJarMetadata = (String, Option<String>, Option<String>, Vec<String>);
pub type RawJarTuple = RawJarMetadata;

fn read_zip_text<R: Read + std::io::Seek>(
    zip: &mut zip::ZipArchive<R>,
    name: &str,
) -> Option<String> {
    let mut entry = zip.by_name(name).ok()?;
    let mut text = String::new();
    entry.read_to_string(&mut text).ok()?;
    Some(text)
}

pub fn read_raw_jar_metadata(path: &Path) -> Option<RawJarMetadata> {
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

        return Some((name, version, description, authors));
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

        return Some((name, version, description, authors));
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

        return Some((name, version, text("description"), authors));
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

                return Some((name, version, description, authors));
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

    Some((name, top("version"), top("description"), authors))
}
