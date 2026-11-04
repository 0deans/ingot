use std::fs;
use std::path::Path;
use std::time::UNIX_EPOCH;

use super::dependencies::check_dependencies;
use super::error::InstanceContentError;
use super::file_name::ContentFileName;
use super::metadata::read_content_metadata;
use super::models::{
    ContentCategory, ContentListResult, ContentMetadata, ContentOrigin, ContentState,
    InstalledContent,
};
use super::tracking::read_tracking;

pub fn list_content(
    instance_dir: &Path,
    category: ContentCategory,
) -> Result<ContentListResult, InstanceContentError> {
    let subfolder = category.default_subfolder();
    let folder_path = instance_dir.join(subfolder);

    let entries = match fs::read_dir(&folder_path) {
        Ok(e) => e,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Ok(ContentListResult {
                items: Vec::new(),
                dependency_issues: Vec::new(),
            });
        }
        Err(e) => return Err(InstanceContentError::io(folder_path, e)),
    };

    let tracking = read_tracking(instance_dir).unwrap_or_default();
    let is_pack = category != ContentCategory::Mod;
    let mut items = Vec::new();

    for entry_res in entries {
        let entry = match entry_res {
            Ok(e) => e,
            Err(_) => continue,
        };

        let raw_name = entry.file_name().to_string_lossy().into_owned();
        let parsed_name = match ContentFileName::parse(&raw_name, category) {
            Ok(name) => name,
            Err(_) => continue,
        };

        let fs_meta = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };

        // For mods, only files are valid.
        if category == ContentCategory::Mod && !fs_meta.is_file() {
            continue;
        }

        let size_bytes = if fs_meta.is_dir() {
            // For directories, calculate total size or use 0
            0
        } else {
            fs_meta.len()
        };

        let modified_at = fs_meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let state = if parsed_name.is_disabled() {
            ContentState::Disabled
        } else {
            ContentState::Enabled
        };

        let meta =
            read_content_metadata(&entry.path(), is_pack).unwrap_or_else(|| ContentMetadata {
                name: parsed_name.base_name().to_string(),
                mod_id: None,
                version: None,
                description: None,
                authors: Vec::new(),
                icon_data_url: None,
                dependencies: Vec::new(),
            });

        let origin = tracking
            .get(parsed_name.base_name())
            .cloned()
            .map(ContentOrigin::Tracked)
            .unwrap_or(ContentOrigin::Unknown);

        items.push(InstalledContent {
            file_name: raw_name,
            category,
            state,
            size_bytes,
            modified_at,
            metadata: meta,
            origin,
        });
    }

    items.sort_by_key(|a| a.metadata.name.to_lowercase());

    let dependency_issues = if category == ContentCategory::Mod {
        check_dependencies(&items)
    } else {
        Vec::new()
    };

    Ok(ContentListResult {
        items,
        dependency_issues,
    })
}
