use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::archive::{ArchivePlan, ArchivePlanItem, ArchiveSource};
use super::error::InstanceExportError;
use super::hashing::hash_file;
use super::models::ExportOptions;
use super::resolve::{is_candidate_for_lookup, ContentResolver};
use ingot_minecraft::instance::{InstanceConfig, ModLoaderType};
use ingot_minecraft::instance_content::models::{ProviderSource, TrackedOrigin};
use ingot_minecraft::loader::normalize_loader_version;

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CurseForgeManifest {
    pub minecraft: CurseForgeMinecraft,
    pub manifest_type: String,
    pub manifest_version: u32,
    pub name: String,
    pub version: String,
    pub author: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    pub files: Vec<CurseForgeFile>,
    pub overrides: String,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CurseForgeMinecraft {
    pub version: String,
    pub mod_loaders: Vec<CurseForgeModLoader>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CurseForgeModLoader {
    pub id: String,
    pub primary: bool,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CurseForgeFile {
    #[serde(rename = "projectID")]
    pub project_id: u32,
    #[serde(rename = "fileID")]
    pub file_id: u32,
    pub required: bool,
}

pub fn html_escape(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for c in input.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            other => out.push(other),
        }
    }
    out
}

pub fn generate_modlist_html(
    resolved_entries: &[(String, u32)],
    embedded_paths: &[String],
) -> String {
    let mut html = String::new();
    html.push_str("<!DOCTYPE html>\n<html>\n<head><title>Mod List</title></head>\n<body>\n<ul>\n");

    for (name, project_id) in resolved_entries {
        let safe_name = html_escape(name);
        html.push_str(&format!(
            "<li><a href=\"https://www.curseforge.com/projects/{project_id}\">{safe_name}</a></li>\n"
        ));
    }

    for path in embedded_paths {
        let safe_name = html_escape(path);
        html.push_str(&format!("<li>{safe_name}</li>\n"));
    }

    html.push_str("</ul>\n</body>\n</html>");
    html
}

pub fn build_curseforge_modloaders(
    config: &InstanceConfig,
) -> Result<Vec<CurseForgeModLoader>, InstanceExportError> {
    if config.loader == ModLoaderType::Horizon {
        return Err(InstanceExportError::UnsupportedLoader(
            ModLoaderType::Horizon,
        ));
    }

    let loader_str = match config.loader {
        ModLoaderType::Vanilla => return Ok(Vec::new()),
        ModLoaderType::Fabric => "fabric",
        ModLoaderType::Quilt => "quilt",
        ModLoaderType::Forge => "forge",
        ModLoaderType::NeoForge => "neoforge",
        ModLoaderType::Horizon => unreachable!(),
    };

    let loader_ver = config
        .loader_version
        .as_deref()
        .map(|v| normalize_loader_version(&config.game_version, v))
        .filter(|v| !v.trim().is_empty())
        .ok_or_else(|| {
            InstanceExportError::Serialization(format!(
                "Missing loader version for {:?}",
                config.loader
            ))
        })?;

    Ok(vec![CurseForgeModLoader {
        id: format!("{loader_str}-{loader_ver}"),
        primary: true,
    }])
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedCurseForgeItem {
    pub file_name: String,
    pub project_id: u32,
    pub file_id: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmbeddedCurseForgeItem {
    pub rel_path: String,
    pub full_path: PathBuf,
    pub warning: Option<String>,
}

pub struct CurseForgeClassification {
    pub resolved: Vec<ResolvedCurseForgeItem>,
    pub embedded: Vec<EmbeddedCurseForgeItem>,
}

pub async fn classify_curseforge_files<R: ContentResolver>(
    resolver: &R,
    tracking: &HashMap<String, TrackedOrigin>,
    included_files: &[(String, PathBuf)],
) -> Result<CurseForgeClassification, InstanceExportError> {
    let mut resolved = Vec::new();
    let mut embedded = Vec::new();

    let mut needs_fingerprint = Vec::new();

    for (rel_path, full_path) in included_files {
        let file_name = Path::new(rel_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(rel_path.as_str());

        // 1. Check if tracked CurseForge origin exists
        if let Some(origin) = tracking.get(file_name) {
            if origin.source == ProviderSource::CurseForge {
                if let (Ok(project_id), Ok(file_id)) = (
                    origin.project_id.parse::<u32>(),
                    origin.version_id.parse::<u32>(),
                ) {
                    resolved.push(ResolvedCurseForgeItem {
                        file_name: file_name.to_string(),
                        project_id,
                        file_id,
                    });
                    continue;
                }
            }
        }

        // 2. Candidate check
        if is_candidate_for_lookup(rel_path) {
            let hashes = hash_file(full_path, true)?;
            if let Some(fp) = hashes.cf_fingerprint {
                needs_fingerprint.push((
                    rel_path.clone(),
                    full_path.clone(),
                    file_name.to_string(),
                    fp,
                ));
            } else {
                embedded.push(EmbeddedCurseForgeItem {
                    rel_path: rel_path.clone(),
                    full_path: full_path.clone(),
                    warning: Some(format!("{rel_path}: Fingerprint computation failed")),
                });
            }
        } else {
            embedded.push(EmbeddedCurseForgeItem {
                rel_path: rel_path.clone(),
                full_path: full_path.clone(),
                warning: None,
            });
        }
    }

    // 3. Batch lookup for files needing fingerprint identification
    if !needs_fingerprint.is_empty() {
        let fps: Vec<u32> = needs_fingerprint.iter().map(|(_, _, _, fp)| *fp).collect();
        let matches = resolver.resolve_curseforge(&fps).await?;

        for (rel_path, full_path, file_name, fp) in needs_fingerprint {
            if let Some(matched) = matches.get(&fp) {
                resolved.push(ResolvedCurseForgeItem {
                    file_name,
                    project_id: matched.project_id,
                    file_id: matched.file_id,
                });
            } else {
                embedded.push(EmbeddedCurseForgeItem {
                    rel_path: rel_path.clone(),
                    full_path,
                    warning: Some(format!("{rel_path}: Not found on CurseForge")),
                });
            }
        }
    }

    Ok(CurseForgeClassification { resolved, embedded })
}

pub struct CurseForgeBuildResult {
    pub plan: ArchivePlan,
    pub resolved_count: usize,
    pub embedded_count: usize,
    pub embedded_warnings: Vec<String>,
}

pub fn build_curseforge_plan(
    config: &InstanceConfig,
    options: &ExportOptions,
    classification: CurseForgeClassification,
) -> Result<CurseForgeBuildResult, InstanceExportError> {
    if options.name.trim().is_empty() {
        return Err(InstanceExportError::EmptyName);
    }

    let mod_loaders = build_curseforge_modloaders(config)?;

    let mut manifest_files = Vec::with_capacity(classification.resolved.len());
    let mut modlist_resolved = Vec::with_capacity(classification.resolved.len());

    for item in &classification.resolved {
        manifest_files.push(CurseForgeFile {
            project_id: item.project_id,
            file_id: item.file_id,
            required: true,
        });
        modlist_resolved.push((item.file_name.clone(), item.project_id));
    }

    let manifest = CurseForgeManifest {
        minecraft: CurseForgeMinecraft {
            version: config.game_version.clone(),
            mod_loaders,
        },
        manifest_type: "minecraftModpack".to_string(),
        manifest_version: 1,
        name: options.name.trim().to_string(),
        version: options.version.trim().to_string(),
        author: options.author.as_deref().unwrap_or("").trim().to_string(),
        summary: options
            .summary
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(String::from),
        files: manifest_files,
        overrides: "overrides".to_string(),
    };

    let manifest_bytes = serde_json::to_vec_pretty(&manifest)
        .map_err(|e| InstanceExportError::Serialization(e.to_string()))?;

    let mut modlist_embedded = Vec::with_capacity(classification.embedded.len());
    let mut embedded_warnings = Vec::new();
    let mut plan = Vec::with_capacity(classification.embedded.len() + 2);

    plan.push(ArchivePlanItem {
        zip_path: "manifest.json".to_string(),
        source: ArchiveSource::Bytes(manifest_bytes),
    });

    for item in classification.embedded {
        if let Some(w) = item.warning {
            embedded_warnings.push(w);
        }
        modlist_embedded.push(item.rel_path.clone());
        plan.push(ArchivePlanItem {
            zip_path: format!("overrides/{}", item.rel_path),
            source: ArchiveSource::File(item.full_path),
        });
    }

    let modlist_html = generate_modlist_html(&modlist_resolved, &modlist_embedded);
    plan.push(ArchivePlanItem {
        zip_path: "modlist.html".to_string(),
        source: ArchiveSource::Bytes(modlist_html.into_bytes()),
    });

    Ok(CurseForgeBuildResult {
        plan,
        resolved_count: classification.resolved.len(),
        embedded_count: modlist_embedded.len(),
        embedded_warnings,
    })
}
