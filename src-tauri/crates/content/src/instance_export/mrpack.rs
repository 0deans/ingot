use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::archive::{ArchivePlan, ArchivePlanItem, ArchiveSource};
use super::error::InstanceExportError;
use super::models::ExportOptions;
use super::resolve::{ClassifiedModrinthFile, ResolvedModrinthFile};
use ingot_minecraft::instance::{InstanceConfig, ModLoaderType};
use ingot_minecraft::loader::normalize_loader_version;

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ModrinthIndex {
    pub format_version: u32,
    pub game: String,
    pub version_id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    pub files: Vec<ModrinthIndexFile>,
    pub dependencies: HashMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ModrinthIndexFile {
    pub path: String,
    pub hashes: HashMap<String, String>,
    pub env: HashMap<String, String>,
    pub downloads: Vec<String>,
    pub file_size: u64,
}

pub fn build_modrinth_dependencies(
    config: &InstanceConfig,
) -> Result<HashMap<String, String>, InstanceExportError> {
    if config.loader == ModLoaderType::Horizon {
        return Err(InstanceExportError::UnsupportedLoader(
            ModLoaderType::Horizon,
        ));
    }

    let mut deps = HashMap::new();
    deps.insert("minecraft".to_string(), config.game_version.clone());

    match config.loader {
        ModLoaderType::Vanilla => {}
        ModLoaderType::Fabric => {
            let loader_ver = config
                .loader_version
                .as_deref()
                .map(|v| normalize_loader_version(&config.game_version, v))
                .filter(|v| !v.trim().is_empty())
                .ok_or_else(|| {
                    InstanceExportError::Serialization(
                        "Missing loader version for Fabric".to_string(),
                    )
                })?;
            deps.insert("fabric-loader".to_string(), loader_ver);
        }
        ModLoaderType::Quilt => {
            let loader_ver = config
                .loader_version
                .as_deref()
                .map(|v| normalize_loader_version(&config.game_version, v))
                .filter(|v| !v.trim().is_empty())
                .ok_or_else(|| {
                    InstanceExportError::Serialization(
                        "Missing loader version for Quilt".to_string(),
                    )
                })?;
            deps.insert("quilt-loader".to_string(), loader_ver);
        }
        ModLoaderType::Forge => {
            let loader_ver = config
                .loader_version
                .as_deref()
                .map(|v| normalize_loader_version(&config.game_version, v))
                .filter(|v| !v.trim().is_empty())
                .ok_or_else(|| {
                    InstanceExportError::Serialization(
                        "Missing loader version for Forge".to_string(),
                    )
                })?;
            deps.insert("forge".to_string(), loader_ver);
        }
        ModLoaderType::NeoForge => {
            let loader_ver = config
                .loader_version
                .as_deref()
                .map(|v| normalize_loader_version(&config.game_version, v))
                .filter(|v| !v.trim().is_empty())
                .ok_or_else(|| {
                    InstanceExportError::Serialization(
                        "Missing loader version for NeoForge".to_string(),
                    )
                })?;
            deps.insert("neoforge".to_string(), loader_ver);
        }
        ModLoaderType::Horizon => unreachable!(),
    }

    Ok(deps)
}

pub fn build_modrinth_index(
    config: &InstanceConfig,
    options: &ExportOptions,
    resolved_files: &[ResolvedModrinthFile],
) -> Result<ModrinthIndex, InstanceExportError> {
    if options.name.trim().is_empty() {
        return Err(InstanceExportError::EmptyName);
    }

    let dependencies = build_modrinth_dependencies(config)?;

    let mut index_files = Vec::with_capacity(resolved_files.len());
    for rf in resolved_files {
        let mut hashes = HashMap::new();
        hashes.insert("sha1".to_string(), rf.sha1.clone());
        hashes.insert("sha512".to_string(), rf.sha512.clone());

        let mut env = HashMap::new();
        env.insert("client".to_string(), rf.client_env.as_str().to_string());
        env.insert("server".to_string(), rf.server_env.as_str().to_string());

        index_files.push(ModrinthIndexFile {
            path: rf.path.clone(),
            hashes,
            env,
            downloads: vec![rf.download_url.clone()],
            file_size: rf.size_bytes,
        });
    }

    Ok(ModrinthIndex {
        format_version: 1,
        game: "minecraft".to_string(),
        version_id: options.version.trim().to_string(),
        name: options.name.trim().to_string(),
        summary: options
            .summary
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(String::from),
        files: index_files,
        dependencies,
    })
}

pub struct MrpackBuildResult {
    pub plan: ArchivePlan,
    pub resolved_count: usize,
    pub embedded_count: usize,
    pub embedded_warnings: Vec<String>,
}

pub fn build_mrpack_plan(
    config: &InstanceConfig,
    options: &ExportOptions,
    classified_files: Vec<ClassifiedModrinthFile>,
) -> Result<MrpackBuildResult, InstanceExportError> {
    let mut resolved_files = Vec::new();
    let mut embedded_items = Vec::new();
    let mut embedded_warnings = Vec::new();

    for file in classified_files {
        match file {
            ClassifiedModrinthFile::Resolved(rf) => {
                resolved_files.push(rf);
            }
            ClassifiedModrinthFile::Embedded {
                source_path,
                target_override_path,
                warning,
                ..
            } => {
                if let Some(w) = warning {
                    embedded_warnings.push(w);
                }
                embedded_items.push(ArchivePlanItem {
                    zip_path: target_override_path,
                    source: ArchiveSource::File(source_path),
                });
            }
        }
    }

    let resolved_count = resolved_files.len();
    let embedded_count = embedded_items.len();

    let index = build_modrinth_index(config, options, &resolved_files)?;
    let index_bytes = serde_json::to_vec_pretty(&index)
        .map_err(|e| InstanceExportError::Serialization(e.to_string()))?;

    let mut plan = Vec::with_capacity(embedded_items.len() + 1);
    plan.push(ArchivePlanItem {
        zip_path: "modrinth.index.json".to_string(),
        source: ArchiveSource::Bytes(index_bytes),
    });
    plan.extend(embedded_items);

    Ok(MrpackBuildResult {
        plan,
        resolved_count,
        embedded_count,
        embedded_warnings,
    })
}
