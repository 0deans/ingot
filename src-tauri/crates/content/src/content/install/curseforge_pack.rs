//! CurseForge `.zip` modpack planner and mod download executor.
use super::{
    curseforge_resolver::{self, CurseForgeEndpoints},
    extract_zip_overrides, parse_curseforge_loader,
    support::{self, PreparedModpack},
    CurseForgeManifest, CurseForgeManifestFile,
};
use crate::content::error::ContentError;
use ingot_minecraft::launcher::progress::ProgressCallback;
use std::{io::Cursor, path::Path, sync::Arc};
use zip::ZipArchive;

pub(super) fn plan_curseforge(
    root: &Path,
    mut zip: ZipArchive<Cursor<Vec<u8>>>,
    requested_name: &str,
) -> Result<(PreparedModpack, Vec<CurseForgeManifestFile>), ContentError> {
    let manifest: CurseForgeManifest = support::read_zip_json_manifest(&mut zip, "manifest.json")?;
    if manifest.minecraft.version.trim().is_empty() {
        return Err(ContentError::MissingField("minecraft.version"));
    }
    let (loader, loader_version) =
        parse_curseforge_loader(&manifest.minecraft.mod_loaders.unwrap_or_default());
    let folder = manifest.overrides.unwrap_or_else(|| "overrides".into());
    let stage = tempfile::Builder::new()
        .prefix(".modpack-")
        .tempdir_in(root)
        .map_err(ContentError::Io)?;
    support::validate_stage_path(stage.path(), &folder)?;
    extract_zip_overrides(&mut zip, &[&format!("{folder}/")], stage.path())
        .map_err(ContentError::Io)?;
    let fallback_name = manifest.name.unwrap_or_else(|| "CurseForge Modpack".into());
    let prepared = PreparedModpack {
        stage: Arc::new(stage),
        name: support::resolve_pack_name(requested_name, fallback_name),
        game_version: manifest.minecraft.version,
        loader,
        loader_version,
    };
    Ok((prepared, manifest.files.unwrap_or_default()))
}

pub(super) async fn download_curseforge_files(
    client: &reqwest::Client,
    endpoints: CurseForgeEndpoints<'_>,
    stage: &Arc<tempfile::TempDir>,
    files: Vec<CurseForgeManifestFile>,
    on_progress: Option<&ProgressCallback>,
) -> Result<(), ContentError> {
    let total = files.len() as u64;
    for (idx, file) in files.into_iter().enumerate() {
        if let Some(report) = on_progress {
            report(
                "Installing Modpack",
                idx as u64,
                total,
                &format!("Downloading mod {}/{}...", idx + 1, total),
            );
        }
        let resolved = curseforge_resolver::resolve_curseforge_file_with_cdn(
            client,
            endpoints,
            file.project_id,
            file.file_id,
        )
        .await?;
        support::validate_stage_filename(stage.path(), &resolved.file_name)?;
        let destination = format!("mods/{}", resolved.file_name);
        if let Some(report) = on_progress {
            report(
                "Installing Modpack",
                idx as u64,
                total,
                &format!("Downloading {} ({}/{})", resolved.file_name, idx + 1, total),
            );
        }
        support::write_download(
            client,
            &resolved.download_url,
            stage,
            &destination,
            resolved.integrity,
        )
        .await?;
    }
    support::report_finalizing(on_progress, total);
    Ok(())
}
