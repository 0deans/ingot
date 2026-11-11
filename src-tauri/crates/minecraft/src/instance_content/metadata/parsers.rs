//! Pure metadata parsers for JAR mod manifests, plugin descriptors, and pack metadata.

use base64::Engine;
use serde_json::Value;

use crate::instance_content::models::{ContentDependency, ContentMetadata};

pub(super) const MAX_ICON_BYTES: usize = 256 * 1024; // 256 KB max for embedded base64 icons

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ParsedJarMetadata {
    pub(super) metadata: ContentMetadata,
    pub(super) icon_candidates: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum ManifestParse {
    /// Text was not valid JSON/TOML or did not match this manifest shape; try fallback formats.
    NotMatched,
    /// Manifest format matched cleanly but lacked mandatory fields (e.g. `name`/`id`), halting lookup.
    MissingRequired,
    /// Manifest parsed into content metadata and ordered archive icon candidates.
    Parsed(ParsedJarMetadata),
}

pub(super) fn encode_png_data_url(bytes: &[u8]) -> Option<String> {
    if bytes.len() > MAX_ICON_BYTES {
        return None;
    }
    let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
    Some(format!("data:image/png;base64,{encoded}"))
}

pub(super) fn pack_name_from_file_name(file_name: Option<&str>, strip_zip_suffix: bool) -> String {
    match file_name {
        Some(raw) if strip_zip_suffix => raw
            .strip_suffix(".zip")
            .or_else(|| raw.strip_suffix(".zip.disabled"))
            .unwrap_or(raw)
            .to_string(),
        Some(raw) => raw.to_string(),
        None => "Pack".to_string(),
    }
}

/// Parses `pack.mcmeta` JSON and returns `Some(optional_description)` when a `pack` object exists.
pub(super) fn parse_pack_mcmeta_description(text: &str) -> Option<Option<String>> {
    let pack_val: Value = serde_json::from_str(text).ok()?;
    let pack = pack_val.get("pack")?;
    let description = pack.get("description").and_then(|d| {
        d.as_str()
            .map(str::to_string)
            .or_else(|| d.get("text").and_then(Value::as_str).map(str::to_string))
    });
    Some(description)
}

pub(super) fn parse_fabric_mod_json(text: &str) -> ManifestParse {
    let Ok(json) = serde_json::from_str::<Value>(text) else {
        return ManifestParse::NotMatched;
    };

    let id = json.get("id").and_then(Value::as_str).map(str::to_string);
    let Some(name) = json
        .get("name")
        .and_then(Value::as_str)
        .map(str::to_string)
        .or_else(|| id.clone())
    else {
        return ManifestParse::MissingRequired;
    };

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

    let mut icon_candidates = Vec::new();
    if let Some(icon_path) = json.get("icon").and_then(Value::as_str) {
        icon_candidates.push(icon_path.to_string());
    }
    icon_candidates.push("assets/icon.png".to_string());

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

    ManifestParse::Parsed(ParsedJarMetadata {
        metadata: ContentMetadata {
            name,
            mod_id: id,
            version,
            description,
            authors,
            icon_data_url: None,
            dependencies,
        },
        icon_candidates,
    })
}

pub(super) fn parse_quilt_mod_json(text: &str) -> ManifestParse {
    let Ok(json) = serde_json::from_str::<Value>(text) else {
        return ManifestParse::NotMatched;
    };

    let Some(loader) = json.get("quilt_loader") else {
        return ManifestParse::MissingRequired;
    };
    let meta = loader.get("metadata").cloned().unwrap_or(Value::Null);
    let id = loader.get("id").and_then(Value::as_str).map(str::to_string);
    let Some(name) = meta
        .get("name")
        .and_then(Value::as_str)
        .map(str::to_string)
        .or_else(|| id.clone())
    else {
        return ManifestParse::MissingRequired;
    };

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

    let icon_candidates = meta
        .get("icon")
        .and_then(Value::as_str)
        .map(|p| vec![p.to_string()])
        .unwrap_or_default();

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

    ManifestParse::Parsed(ParsedJarMetadata {
        metadata: ContentMetadata {
            name,
            mod_id: id,
            version,
            description,
            authors,
            icon_data_url: None,
            dependencies,
        },
        icon_candidates,
    })
}

pub(super) fn parse_forge_mods_toml(toml_text: &str, manifest_mf: Option<&str>) -> ManifestParse {
    let Ok(doc) = toml_text.parse::<toml_edit::DocumentMut>() else {
        return ManifestParse::NotMatched;
    };

    let Some(first) = doc
        .get("mods")
        .and_then(|v| v.as_array_of_tables())
        .and_then(|arr| arr.iter().next())
    else {
        return ManifestParse::MissingRequired;
    };

    let text = |key: &str| {
        first
            .get(key)
            .and_then(|v| v.as_str())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
    };

    let mod_id = text("modId");
    let Some(name) = text("displayName").or_else(|| mod_id.clone()) else {
        return ManifestParse::MissingRequired;
    };

    let version = text("version").and_then(|v| {
        if !v.starts_with("${") {
            return Some(v);
        }
        manifest_mf?.lines().find_map(|l| {
            l.strip_prefix("Implementation-Version:")
                .map(|ver| ver.trim().to_string())
        })
    });

    let raw_authors = doc
        .get("authors")
        .and_then(|v| v.as_str())
        .or_else(|| first.get("authors").and_then(|v| v.as_str()));
    let authors = raw_authors
        .map(|a| {
            a.split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()
        })
        .unwrap_or_default();

    let icon_candidates = text("logoFile").into_iter().collect();

    let mut dependencies = Vec::new();
    if let Some(mod_id_str) = &mod_id {
        let dep_key = format!("dependencies.{mod_id_str}");
        let dep_arr = doc
            .get("dependencies")
            .and_then(|deps| deps.as_table())
            .and_then(|deps| deps.get(mod_id_str))
            .or_else(|| doc.get(&dep_key))
            .and_then(|v| v.as_array_of_tables());
        if let Some(dep_arr) = dep_arr {
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

    ManifestParse::Parsed(ParsedJarMetadata {
        metadata: ContentMetadata {
            name,
            mod_id,
            version,
            description: text("description"),
            authors,
            icon_data_url: None,
            dependencies,
        },
        icon_candidates,
    })
}

pub(super) fn parse_mcmod_info(text: &str) -> ManifestParse {
    let Ok(info_val) = serde_json::from_str::<Value>(text) else {
        return ManifestParse::NotMatched;
    };

    let item = info_val.as_array().and_then(|arr| arr.first()).or_else(|| {
        info_val
            .get("modList")
            .and_then(Value::as_array)
            .and_then(|arr| arr.first())
    });

    let Some(first) = item else {
        return ManifestParse::NotMatched;
    };

    let mod_id = first
        .get("modid")
        .and_then(Value::as_str)
        .map(str::to_string);
    let Some(name) = first
        .get("name")
        .and_then(Value::as_str)
        .map(str::to_string)
        .or_else(|| mod_id.clone())
    else {
        return ManifestParse::MissingRequired;
    };

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

    let icon_candidates = first
        .get("logoFile")
        .and_then(Value::as_str)
        .map(|p| vec![p.to_string()])
        .unwrap_or_default();

    ManifestParse::Parsed(ParsedJarMetadata {
        metadata: ContentMetadata {
            name,
            mod_id,
            version,
            description,
            authors,
            icon_data_url: None,
            dependencies: Vec::new(),
        },
        icon_candidates,
    })
}

pub(super) fn parse_bukkit_plugin_yml(yaml: &str) -> Option<ContentMetadata> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_fabric_and_quilt_manifests_and_filters_builtin_deps() {
        let fabric_json = r#"{
            "id": "sodium",
            "name": "Sodium",
            "version": "0.6.0",
            "authors": ["jellysquid3", {"name": "IMS212"}],
            "icon": "assets/sodium/icon.png",
            "depends": {
                "minecraft": ">=1.21",
                "fabricloader": ">=0.16",
                "fabric-api": "*",
                "java": ">=21"
            }
        }"#;
        let ManifestParse::Parsed(fabric) = parse_fabric_mod_json(fabric_json) else {
            panic!("expected parsed fabric manifest");
        };
        assert_eq!(fabric.metadata.name, "Sodium");
        assert_eq!(fabric.metadata.authors, vec!["jellysquid3", "IMS212"]);
        assert_eq!(
            fabric.icon_candidates,
            vec!["assets/sodium/icon.png", "assets/icon.png"]
        );
        assert_eq!(
            fabric.metadata.dependencies,
            vec![ContentDependency {
                id: "fabric-api".to_string(),
                name: None,
                required: true,
            }]
        );

        let quilt_json = r#"{
            "quilt_loader": {
                "id": "qsl",
                "version": "9.0.0",
                "metadata": {
                    "name": "Quilt Standard Libraries",
                    "contributors": {"QuiltMC": "Owner"},
                    "icon": "assets/qsl/icon.png"
                },
                "depends": ["quilt_loader", {"id": "quilt_base"}, "minecraft"]
            }
        }"#;
        let ManifestParse::Parsed(quilt) = parse_quilt_mod_json(quilt_json) else {
            panic!("expected parsed quilt manifest");
        };
        assert_eq!(quilt.metadata.mod_id.as_deref(), Some("qsl"));
        assert_eq!(quilt.metadata.authors, vec!["QuiltMC"]);
        assert_eq!(quilt.icon_candidates, vec!["assets/qsl/icon.png"]);
        assert_eq!(quilt.metadata.dependencies.len(), 1);
        assert_eq!(quilt.metadata.dependencies[0].id, "quilt_base");
    }

    #[test]
    fn parses_forge_mods_toml_manifest_version_placeholder_and_deps() {
        let toml = r#"
authors = "Author One, Author Two"
[[mods]]
modId = "examplemod"
displayName = "Example Mod"
version = "${file.jarVersion}"
logoFile = "logo.png"

[[dependencies.examplemod]]
modId = "minecraft"
mandatory = true

[[dependencies.examplemod]]
modId = "curios"
mandatory = false
"#;
        let manifest = "Manifest-Version: 1.0\nImplementation-Version: 4.2.1\n";

        let ManifestParse::Parsed(parsed) = parse_forge_mods_toml(toml, Some(manifest)) else {
            panic!("expected parsed forge manifest");
        };
        assert_eq!(parsed.metadata.version.as_deref(), Some("4.2.1"));
        assert_eq!(parsed.metadata.authors, vec!["Author One", "Author Two"]);
        assert_eq!(parsed.icon_candidates, vec!["logo.png"]);
        assert_eq!(
            parsed.metadata.dependencies,
            vec![ContentDependency {
                id: "curios".to_string(),
                name: None,
                required: false,
            }]
        );
    }

    #[test]
    fn parses_mcmod_info_and_bukkit_plugin_yml() {
        let mcmod = r#"{"modList":[{"modid":"legacy","name":"Legacy Mod","authorList":["Dev"],"logoFile":"icon.png"}]}"#;
        let ManifestParse::Parsed(parsed) = parse_mcmod_info(mcmod) else {
            panic!("expected parsed mcmod.info");
        };
        assert_eq!(parsed.metadata.name, "Legacy Mod");
        assert_eq!(parsed.icon_candidates, vec!["icon.png"]);

        let ManifestParse::NotMatched = parse_mcmod_info("[]") else {
            panic!("empty mcmod array must fall through");
        };

        let yml = "name: 'PaperPlugin'\nversion: 1.4.0\nauthor: Alice\nauthors: [Bob, \"Carol\"]\ndescription: >\n";
        let plugin = parse_bukkit_plugin_yml(yml).unwrap();
        assert_eq!(plugin.name, "PaperPlugin");
        assert_eq!(plugin.version.as_deref(), Some("1.4.0"));
        assert_eq!(plugin.authors, vec!["Alice", "Bob", "Carol"]);
        assert_eq!(plugin.description, None);
    }

    #[test]
    fn parses_pack_mcmeta_pack_names_and_icon_size_limit() {
        assert_eq!(
            parse_pack_mcmeta_description(r#"{"pack":{"description":{"text":"HD Textures"}}}"#),
            Some(Some("HD Textures".to_string()))
        );
        assert_eq!(parse_pack_mcmeta_description(r#"{"meta":{}}"#), None);
        assert_eq!(
            pack_name_from_file_name(Some("Faithful.zip.disabled"), true),
            "Faithful"
        );
        assert_eq!(
            pack_name_from_file_name(Some("Faithful.zip"), false),
            "Faithful.zip"
        );
        assert_eq!(pack_name_from_file_name(None, true), "Pack");

        let within_limit = vec![0u8; MAX_ICON_BYTES];
        assert!(encode_png_data_url(&within_limit).is_some());
        let over_limit = vec![0u8; MAX_ICON_BYTES + 1];
        assert!(encode_png_data_url(&over_limit).is_none());
    }
}
