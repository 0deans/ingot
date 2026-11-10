use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::archive::{ArchivePlan, ArchivePlanItem, ArchiveSource};
use super::error::InstanceExportError;
use crate::minecraft::instance::{InstanceConfig, ModLoaderType};
use crate::minecraft::loader::normalize_loader_version;

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MmcPack {
    pub format_version: u32,
    pub components: Vec<MmcComponent>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MmcComponent {
    pub cached_name: String,
    pub cached_version: String,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub important: bool,
    pub uid: String,
    pub version: String,
}

/// Generate Prism / MultiMC instance.cfg content.
pub fn generate_instance_cfg(config: &InstanceConfig, export_name: &str) -> String {
    let mut cfg = String::new();
    cfg.push_str("[General]\n");
    cfg.push_str("InstanceType=OneSix\n");
    cfg.push_str(&format!("name={}\n", export_name.trim()));

    let icon = config
        .icon
        .as_deref()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or("default");
    cfg.push_str(&format!("iconKey={icon}\n"));

    if let Some(jvm_args) = &config.jvm_args {
        if !jvm_args.is_empty() {
            cfg.push_str("OverrideJavaArgs=true\n");
            cfg.push_str(&format!("JvmArgs={}\n", jvm_args.join(" ")));
        } else {
            cfg.push_str("OverrideJavaArgs=false\n");
        }
    } else {
        cfg.push_str("OverrideJavaArgs=false\n");
    }

    if config.memory_min_mb.is_some() || config.memory_max_mb.is_some() {
        cfg.push_str("OverrideMemory=true\n");
        if let Some(min) = config.memory_min_mb {
            cfg.push_str(&format!("MinMemAlloc={min}\n"));
        }
        if let Some(max) = config.memory_max_mb {
            cfg.push_str(&format!("MaxMemAlloc={max}\n"));
        }
    } else {
        cfg.push_str("OverrideMemory=false\n");
    }

    cfg
}

/// Generate Prism / MultiMC mmc-pack.json content.
pub fn generate_mmc_pack(config: &InstanceConfig) -> Result<MmcPack, InstanceExportError> {
    if config.loader == ModLoaderType::Horizon {
        return Err(InstanceExportError::UnsupportedLoader(
            ModLoaderType::Horizon,
        ));
    }

    let mut components = Vec::new();

    // 1. Minecraft base component
    components.push(MmcComponent {
        cached_name: "Minecraft".to_string(),
        cached_version: config.game_version.clone(),
        important: true,
        uid: "net.minecraft".to_string(),
        version: config.game_version.clone(),
    });

    // 2. Mod loader and intermediate mapping components
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

            components.push(MmcComponent {
                cached_name: "Intermediary Mappings".to_string(),
                cached_version: config.game_version.clone(),
                important: false,
                uid: "net.fabricmc.intermediary".to_string(),
                version: config.game_version.clone(),
            });

            components.push(MmcComponent {
                cached_name: "Fabric Loader".to_string(),
                cached_version: loader_ver.clone(),
                important: false,
                uid: "net.fabricmc.fabric-loader".to_string(),
                version: loader_ver,
            });
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

            components.push(MmcComponent {
                cached_name: "Hashed Mojmap".to_string(),
                cached_version: config.game_version.clone(),
                important: false,
                uid: "org.quiltmc.hashed".to_string(),
                version: config.game_version.clone(),
            });

            components.push(MmcComponent {
                cached_name: "Quilt Loader".to_string(),
                cached_version: loader_ver.clone(),
                important: false,
                uid: "org.quiltmc.quilt-loader".to_string(),
                version: loader_ver,
            });
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

            components.push(MmcComponent {
                cached_name: "Forge".to_string(),
                cached_version: loader_ver.clone(),
                important: false,
                uid: "net.minecraftforge".to_string(),
                version: loader_ver,
            });
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

            components.push(MmcComponent {
                cached_name: "NeoForge".to_string(),
                cached_version: loader_ver.clone(),
                important: false,
                uid: "net.neoforged".to_string(),
                version: loader_ver,
            });
        }
        ModLoaderType::Horizon => unreachable!(),
    }

    Ok(MmcPack {
        format_version: 1,
        components,
    })
}

/// Build the full archive plan for Prism / MultiMC format export.
pub fn build_prism_plan(
    config: &InstanceConfig,
    export_name: &str,
    included_files: &[(String, PathBuf)],
) -> Result<ArchivePlan, InstanceExportError> {
    if export_name.trim().is_empty() {
        return Err(InstanceExportError::EmptyName);
    }

    let instance_cfg = generate_instance_cfg(config, export_name);
    let mmc_pack = generate_mmc_pack(config)?;
    let mmc_pack_bytes = serde_json::to_vec_pretty(&mmc_pack)
        .map_err(|e| InstanceExportError::Serialization(e.to_string()))?;

    let mut plan = Vec::with_capacity(included_files.len() + 2);
    plan.push(ArchivePlanItem {
        zip_path: "instance.cfg".to_string(),
        source: ArchiveSource::Bytes(instance_cfg.into_bytes()),
    });
    plan.push(ArchivePlanItem {
        zip_path: "mmc-pack.json".to_string(),
        source: ArchiveSource::Bytes(mmc_pack_bytes),
    });

    for (rel_path, full_path) in included_files {
        plan.push(ArchivePlanItem {
            zip_path: format!(".minecraft/{rel_path}"),
            source: ArchiveSource::File(full_path.clone()),
        });
    }

    Ok(plan)
}
