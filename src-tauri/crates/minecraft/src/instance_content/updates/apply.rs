use std::fs;
use std::path::{Path, PathBuf};

use reqwest::Client;

use super::super::error::InstanceContentError;
use super::super::file_name::ContentFileName;
use super::super::models::{
    ContentApplyReport, ContentCategory, ContentUpdateCandidate, TrackedOrigin,
};
use super::super::tracking::{read_tracking, write_tracking};
use crate::artifacts::models::sha1_hex;

pub async fn apply_updates(
    instance_dir: &Path,
    category: ContentCategory,
    selections: Vec<ContentUpdateCandidate>,
    http_client: &Client,
) -> Result<ContentApplyReport, InstanceContentError> {
    if selections.is_empty() {
        return Ok(ContentApplyReport {
            updated: Vec::new(),
            failed: Vec::new(),
            warnings: Vec::new(),
        });
    }

    let op_id = uuid::Uuid::new_v4().to_string();
    let parent_staging = instance_dir.join(".ingot").join("staging");
    let staging_dir = parent_staging.join(&op_id);
    let new_dir = staging_dir.join("new");
    let old_dir = staging_dir.join("old");

    fs::create_dir_all(&new_dir).map_err(|e| InstanceContentError::io(&new_dir, e))?;
    fs::create_dir_all(&old_dir).map_err(|e| InstanceContentError::io(&old_dir, e))?;

    // Phase A: Download and verify all selections
    for sel in &selections {
        let resp = match http_client.get(&sel.download_url).send().await {
            Ok(r) => r,
            Err(e) => {
                let _ = fs::remove_dir_all(&staging_dir);
                return Err(InstanceContentError::InvalidOperation(format!(
                    "Download failed for {}: {e}",
                    sel.title
                )));
            }
        };

        let bytes = match resp.bytes().await {
            Ok(b) => b,
            Err(e) => {
                let _ = fs::remove_dir_all(&staging_dir);
                return Err(InstanceContentError::InvalidOperation(format!(
                    "Failed to read download stream for {}: {e}",
                    sel.title
                )));
            }
        };

        if let Some(expected_sha1) = &sel.sha1 {
            let actual_sha1 = sha1_hex(&bytes);
            if !actual_sha1.eq_ignore_ascii_case(expected_sha1) {
                let _ = fs::remove_dir_all(&staging_dir);
                return Err(InstanceContentError::InvalidOperation(format!(
                    "Checksum mismatch for {}: expected {}, got {}",
                    sel.title, expected_sha1, actual_sha1
                )));
            }
        }

        let staged_file = new_dir.join(&sel.new_file_name);
        if let Err(e) = fs::write(&staged_file, &bytes) {
            let _ = fs::remove_dir_all(&staging_dir);
            return Err(InstanceContentError::io(staged_file, e));
        }
    }

    // Phase B: Atomic swap with rollback protection
    let folder = instance_dir.join(category.default_subfolder());
    let mut moved_to_old: Vec<(PathBuf, PathBuf)> = Vec::new();
    let mut moved_to_live: Vec<PathBuf> = Vec::new();
    let mut updated_files: Vec<String> = Vec::new();

    let mut swap_result = Ok(());

    for sel in &selections {
        let is_disabled = sel.file_name.ends_with(".disabled");
        let dest_file_name = if is_disabled {
            format!("{}.disabled", sel.new_file_name)
        } else {
            sel.new_file_name.clone()
        };

        let src_live = folder.join(&sel.file_name);
        let backup_old = old_dir.join(&sel.file_name);
        let staged_new = new_dir.join(&sel.new_file_name);
        let dest_live = folder.join(&dest_file_name);

        // Move old live file to backup
        if src_live.exists() {
            if let Err(e) = fs::rename(&src_live, &backup_old) {
                swap_result = Err(InstanceContentError::io(src_live, e));
                break;
            }
            moved_to_old.push((src_live.clone(), backup_old));
        }

        // Move staged new file to live
        if let Err(e) = fs::rename(&staged_new, &dest_live) {
            swap_result = Err(InstanceContentError::io(dest_live, e));
            break;
        }
        moved_to_live.push(dest_live);
        updated_files.push(dest_file_name);
    }

    // Rollback if swap failed
    if let Err(swap_err) = swap_result {
        for live_path in moved_to_live {
            let _ = fs::remove_file(live_path);
        }
        for (live_path, backup_path) in moved_to_old {
            let _ = fs::rename(backup_path, live_path);
        }
        let _ = fs::remove_dir_all(&staging_dir);
        return Err(swap_err);
    }

    // Phase C: Update origin tracking
    let mut tracking = read_tracking(instance_dir).unwrap_or_default();
    for sel in &selections {
        if let Ok(old_parsed) = ContentFileName::parse(&sel.file_name, category) {
            let old_base = old_parsed.base_name();
            let prev_origin = tracking.remove(old_base);

            let new_base = sel.new_file_name.clone();
            let new_origin = TrackedOrigin {
                source: sel.source,
                project_id: prev_origin
                    .as_ref()
                    .map(|o| o.project_id.clone())
                    .unwrap_or_default(),
                project_slug: prev_origin.as_ref().and_then(|o| o.project_slug.clone()),
                version_id: sel.latest_version_id.clone(),
                version_number: sel.latest_version.clone(),
                title: sel.title.clone(),
                icon_url: prev_origin.and_then(|o| o.icon_url),
            };
            tracking.insert(new_base, new_origin);
        }
    }
    let _ = write_tracking(instance_dir, &tracking);

    // Phase D: Final cleanup of staging
    let _ = fs::remove_dir_all(&staging_dir);

    Ok(ContentApplyReport {
        updated: updated_files,
        failed: Vec::new(),
        warnings: Vec::new(),
    })
}
