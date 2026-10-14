use super::models::RawJarMetadata;
use serde_json::Value;
use std::io::Read;
use std::path::Path;

/// Only plain jar names inside the plugins/mods folder
pub fn safe_jar_name(name: &str) -> Result<&str, String> {
    let base = name.strip_suffix(".disabled").unwrap_or(name);
    let plain = Path::new(name).file_name().and_then(|f| f.to_str()) == Some(name);
    if !plain || !base.to_ascii_lowercase().ends_with(".jar") || name.starts_with('.') {
        return Err(format!("Not a plugin file: {name}"));
    }
    Ok(name)
}

/// name/version/description/authors from plugin.yml, paper-plugin.yml, fabric.mod.json,
/// quilt.mod.json or (neoforge.)mods.toml
pub(crate) fn read_jar_metadata(path: &Path) -> Option<RawJarMetadata> {
    let file = std::fs::File::open(path).ok()?;
    let mut zip = zip::ZipArchive::new(file).ok()?;
    let mut read = |name: &str| -> Option<String> {
        let mut entry = zip.by_name(name).ok()?;
        let mut text = String::new();
        entry.read_to_string(&mut text).ok()?;
        Some(text)
    };

    if let Some(json) = read("fabric.mod.json").and_then(|t| serde_json::from_str::<Value>(&t).ok())
    {
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
        let name = json
            .get("name")
            .or_else(|| json.get("id"))?
            .as_str()?
            .to_string();
        return Some((
            name,
            json.get("version")
                .and_then(Value::as_str)
                .map(str::to_string),
            json.get("description")
                .and_then(Value::as_str)
                .map(str::to_string),
            authors,
        ));
    }

    if let Some(json) = read("quilt.mod.json").and_then(|t| serde_json::from_str::<Value>(&t).ok())
    {
        let loader = json.get("quilt_loader")?;
        let meta = loader.get("metadata").cloned().unwrap_or(Value::Null);
        let authors = meta
            .get("contributors")
            .and_then(Value::as_object)
            .map(|c| c.keys().cloned().collect())
            .unwrap_or_default();
        let name = meta
            .get("name")
            .or_else(|| loader.get("id"))?
            .as_str()?
            .to_string();
        return Some((
            name,
            loader
                .get("version")
                .and_then(Value::as_str)
                .map(str::to_string),
            meta.get("description")
                .and_then(Value::as_str)
                .map(str::to_string),
            authors,
        ));
    }

    // NeoForge (neoforge.mods.toml) and Forge (mods.toml): the jar's first mod
    if let Some(doc) = read("META-INF/neoforge.mods.toml")
        .or_else(|| read("META-INF/mods.toml"))
        .and_then(|t| t.parse::<toml_edit::DocumentMut>().ok())
    {
        let first = doc.get("mods")?.as_array_of_tables()?.iter().next()?;
        let text = |key: &str| {
            first
                .get(key)
                .and_then(|v| v.as_str())
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
        };
        // "${file.jarVersion}" means the version is in the jar's manifest
        let version = text("version").and_then(|v| {
            if !v.starts_with("${") {
                return Some(v);
            }
            read("META-INF/MANIFEST.MF")?.lines().find_map(|l| {
                l.strip_prefix("Implementation-Version:")
                    .map(|v| v.trim().to_string())
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
        let name = text("displayName").or_else(|| text("modId"))?;
        return Some((name, version, text("description"), authors));
    }

    let yaml = read("paper-plugin.yml").or_else(|| read("plugin.yml"))?;
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
