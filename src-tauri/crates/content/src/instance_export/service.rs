use std::path::Path;

use super::archive::write_archive;
use super::curseforge::{build_curseforge_plan, classify_curseforge_files};
use super::error::InstanceExportError;
use super::models::{ExportFormat, ExportOptions, ExportPreview, ExportReport};
use super::mrpack::build_mrpack_plan;
use super::prism::build_prism_plan;
use super::resolve::{classify_modrinth_files, ContentResolver};
use super::selection::{collect_preview_entries, validate_included_paths};
use ingot_minecraft::instance::{InstanceConfig, ModLoaderType};
use ingot_minecraft::instance_content::tracking::read_tracking;

/// Generate preview of exportable files and metadata for an instance.
pub fn preview_export(
    instance_dir: &Path,
    config: &InstanceConfig,
) -> Result<ExportPreview, InstanceExportError> {
    let (entries, _) = collect_preview_entries(instance_dir)?;
    let total_size_bytes = entries
        .iter()
        .filter(|e| e.default_included)
        .map(|e| e.size_bytes)
        .sum();

    Ok(ExportPreview {
        default_name: config.name.clone(),
        default_version: "1.0.0".to_string(),
        entries,
        total_size_bytes,
    })
}

/// Export an instance in the requested format to the target destination path.
pub async fn export_instance<R: ContentResolver>(
    instance_dir: &Path,
    config: &InstanceConfig,
    options: ExportOptions,
    dest: &Path,
    resolver: &R,
) -> Result<ExportReport, InstanceExportError> {
    if options.name.trim().is_empty() {
        return Err(InstanceExportError::EmptyName);
    }

    if config.loader == ModLoaderType::Horizon {
        return Err(InstanceExportError::UnsupportedLoader(
            ModLoaderType::Horizon,
        ));
    }

    let instance_dir_buf = instance_dir.to_path_buf();
    let included_paths = options.included_paths.clone();
    let validated_paths = tokio::task::spawn_blocking(move || {
        validate_included_paths(&instance_dir_buf, &included_paths)
    })
    .await??;

    let dest_buf = dest.to_path_buf();
    let total_files = validated_paths.len();

    match options.format {
        ExportFormat::Prism => {
            let config_clone = config.clone();
            let name_clone = options.name.clone();
            let plan = tokio::task::spawn_blocking(move || {
                build_prism_plan(&config_clone, &name_clone, &validated_paths)
            })
            .await??;

            tokio::task::spawn_blocking(move || write_archive(&dest_buf, plan)).await??;

            Ok(ExportReport {
                output_path: dest.to_string_lossy().to_string(),
                total_files,
                resolved_files: 0,
                embedded_files: total_files,
                embedded_warnings: Vec::new(),
                warnings: Vec::new(),
            })
        }
        ExportFormat::Modrinth => {
            let classified = classify_modrinth_files(resolver, &options, &validated_paths).await?;

            let config_clone = config.clone();
            let options_clone = options.clone();
            let mrpack_res = tokio::task::spawn_blocking(move || {
                build_mrpack_plan(&config_clone, &options_clone, classified)
            })
            .await??;

            let plan = mrpack_res.plan;
            tokio::task::spawn_blocking(move || write_archive(&dest_buf, plan)).await??;

            Ok(ExportReport {
                output_path: dest.to_string_lossy().to_string(),
                total_files,
                resolved_files: mrpack_res.resolved_count,
                embedded_files: mrpack_res.embedded_count,
                embedded_warnings: mrpack_res.embedded_warnings,
                warnings: Vec::new(),
            })
        }
        ExportFormat::CurseForge => {
            let tracking = read_tracking(instance_dir).unwrap_or_default();
            let classification =
                classify_curseforge_files(resolver, &tracking, &validated_paths).await?;

            let config_clone = config.clone();
            let options_clone = options.clone();
            let cf_res = tokio::task::spawn_blocking(move || {
                build_curseforge_plan(&config_clone, &options_clone, classification)
            })
            .await??;

            let plan = cf_res.plan;
            tokio::task::spawn_blocking(move || write_archive(&dest_buf, plan)).await??;

            Ok(ExportReport {
                output_path: dest.to_string_lossy().to_string(),
                total_files,
                resolved_files: cf_res.resolved_count,
                embedded_files: cf_res.embedded_count,
                embedded_warnings: cf_res.embedded_warnings,
                warnings: Vec::new(),
            })
        }
    }
}
