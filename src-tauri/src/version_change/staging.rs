use super::models::{
    safe_file_name_typed, sha1_hex, sha256_hex, PlanItem, TargetFile, VersionChangeError,
    ALLOWED_HOSTS,
};
use std::fs;
use std::path::Path;

pub fn verify(path: &Path, file: &TargetFile) -> bool {
    let Ok(data) = fs::read(path) else {
        return false;
    };
    match (&file.sha1, &file.sha256) {
        (Some(sha1), _) => sha1_hex(&data).eq_ignore_ascii_case(sha1),
        (None, Some(sha256)) => sha256_hex(&data).eq_ignore_ascii_case(sha256),
        (None, None) => false,
    }
}

/// Downloads everything to install into the staging folder, checking each file's hash
pub async fn stage_typed(
    client: &reqwest::Client,
    staging: &Path,
    items: &[&PlanItem],
    progress: &(impl Fn(String) + Sync),
) -> Result<(), VersionChangeError> {
    let _ = fs::remove_dir_all(staging);
    fs::create_dir_all(staging).map_err(|e| {
        VersionChangeError::DownloadPrep(format!("Failed to prepare downloads: {e}"))
    })?;
    let mut jobs = tokio::task::JoinSet::new();
    let total = items.len();
    let mut done = 0;
    let mut queue = items.iter();
    loop {
        while jobs.len() < 6 {
            let Some(item) = queue.next() else {
                break;
            };
            let file = item.target.clone().ok_or_else(|| {
                VersionChangeError::PlanError(format!("{} has nothing to download", item.title))
            })?;
            safe_file_name_typed(&file.file_name, item.kind)?;
            let host = reqwest::Url::parse(&file.url)
                .ok()
                .and_then(|u| u.host_str().map(str::to_string));
            if !host.as_deref().is_some_and(|h| ALLOWED_HOSTS.contains(&h)) {
                return Err(VersionChangeError::UnknownHost(item.title.clone()));
            }
            if file.sha1.is_none() && file.sha256.is_none() {
                return Err(VersionChangeError::NoChecksum(item.title.clone()));
            }
            let dest = staging.join(&item.folder).join(&file.file_name);
            let (client, title) = (client.clone(), item.title.clone());
            jobs.spawn(async move {
                if let Some(parent) = dest.parent() {
                    let _ = fs::create_dir_all(parent);
                }
                let result = crate::minecraft::downloader::download_file_chunked(
                    &client, &file.url, &dest, None, None,
                )
                .await;
                match result {
                    Ok(_) if verify(&dest, &file) => Ok(()),
                    Ok(_) => Err(VersionChangeError::ChecksumMismatch(title)),
                    Err(e) => Err(VersionChangeError::DownloadFailed(title, e.to_string())),
                }
            });
        }
        let Some(result) = jobs.join_next().await else {
            break;
        };
        result.map_err(|e| VersionChangeError::Custom(e.to_string()))??;
        done += 1;
        progress(format!("Downloading {done} of {total}..."));
    }
    Ok(())
}

pub async fn stage(
    client: &reqwest::Client,
    staging: &Path,
    items: &[&PlanItem],
    progress: &(impl Fn(String) + Sync),
) -> Result<(), String> {
    stage_typed(client, staging, items, progress)
        .await
        .map_err(|e| e.to_string())
}
