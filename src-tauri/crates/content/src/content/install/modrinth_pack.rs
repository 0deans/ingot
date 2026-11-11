//! Modrinth `.mrpack` modpack planner and client download executor.
use super::{
    extract_zip_overrides, parse_modrinth_loader,
    support::{self, PreparedModpack},
    ModrinthIndex, ModrinthIndexFile,
};
use crate::content::error::ContentError;
use ingot_minecraft::launcher::progress::ProgressCallback;
use std::{io::Cursor, path::Path, sync::Arc};
use zip::ZipArchive;

pub(super) fn plan_modrinth(
    root: &Path,
    mut zip: ZipArchive<Cursor<Vec<u8>>>,
    requested_name: &str,
) -> Result<(PreparedModpack, Vec<ModrinthIndexFile>), ContentError> {
    let index: ModrinthIndex = support::read_zip_json_manifest(&mut zip, "modrinth.index.json")?;
    let dependencies = index.dependencies.unwrap_or_default();
    let game_version = dependencies
        .get("minecraft")
        .filter(|value| !value.trim().is_empty())
        .cloned()
        .ok_or(ContentError::MissingField("minecraft"))?;
    let (loader, loader_version) = parse_modrinth_loader(&dependencies);
    let stage = tempfile::Builder::new()
        .prefix(".modpack-")
        .tempdir_in(root)
        .map_err(ContentError::Io)?;
    extract_zip_overrides(&mut zip, &["overrides/", "client-overrides/"], stage.path())
        .map_err(ContentError::Io)?;
    let fallback_name = index.name.unwrap_or_else(|| "Modpack Instance".into());
    let prepared = PreparedModpack {
        stage: Arc::new(stage),
        name: support::resolve_pack_name(requested_name, fallback_name),
        game_version,
        loader,
        loader_version,
    };
    Ok((prepared, index.files.unwrap_or_default()))
}

pub(super) async fn download_modrinth_files(
    client: &reqwest::Client,
    stage: &Arc<tempfile::TempDir>,
    files: Vec<ModrinthIndexFile>,
    on_progress: Option<&ProgressCallback>,
) -> Result<(), ContentError> {
    let client_files: Vec<_> = files
        .into_iter()
        .filter(|file| {
            !file
                .env
                .as_ref()
                .and_then(|env| env.get("client"))
                .is_some_and(|value| value == "unsupported")
        })
        .collect();
    let total = client_files.len() as u64;
    for (idx, file) in client_files.into_iter().enumerate() {
        if let Some(report) = on_progress {
            report(
                "Installing Modpack",
                idx as u64,
                total,
                &format!("Downloading {} ({}/{})", file.path, idx + 1, total),
            );
        }
        let url = file
            .downloads
            .first()
            .ok_or(ContentError::MissingField("downloads"))?;
        support::write_download(client, url, stage, &file.path, file.integrity).await?;
    }
    support::report_finalizing(on_progress, total);
    Ok(())
}
