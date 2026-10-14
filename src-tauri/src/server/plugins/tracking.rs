use super::models::{PluginSource, TrackedPlugin};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub fn tracking_path(server_dir: &Path) -> PathBuf {
    server_dir.join(".ingot").join("plugins.json")
}

pub fn read_tracking(server_dir: &Path) -> Vec<TrackedPlugin> {
    std::fs::read_to_string(tracking_path(server_dir))
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default()
}

pub fn write_tracking_typed(server_dir: &Path, tracked: &[TrackedPlugin]) -> Result<(), std::io::Error> {
    use std::io::Write;
    let path = tracking_path(server_dir);
    let parent = path.parent().unwrap_or(server_dir);
    std::fs::create_dir_all(parent)?;
    let raw = serde_json::to_string_pretty(tracked)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

    let mut staged = tempfile::NamedTempFile::new_in(parent)?;
    staged.write_all(raw.as_bytes())?;
    staged.as_file().sync_all()?;
    staged.persist(&path).map_err(|e| e.error)?;
    Ok(())
}

pub fn write_tracking(server_dir: &Path, tracked: &[TrackedPlugin]) {
    let _ = write_tracking_typed(server_dir, tracked);
}

/// Plugins installed from Hangar: file name -> (project, version, title, icon). Hangar has
/// no lookup by file, so this is how a version change recognises them.
pub(crate) fn tracked_hangar(
    server_dir: &Path,
) -> HashMap<String, (String, String, String, Option<String>)> {
    read_tracking(server_dir)
        .into_iter()
        .filter(|t| t.source == PluginSource::Hangar)
        .map(|t| {
            (
                t.file_name,
                (t.project_id, t.version_id, t.title, t.icon_url),
            )
        })
        .collect()
}

/// After a version change: remembers the files it installed (from Modrinth or Hangar)
/// in place of the ones they replaced
pub(crate) fn track_version_change(server_dir: &Path, items: &[crate::version_change::PlanItem]) {
    use crate::version_change::{ItemAction, Source};
    let mut tracked = read_tracking(server_dir);
    for item in items
        .iter()
        .filter(|i| matches!(i.action, ItemAction::Update | ItemAction::Add))
    {
        let source = match item.source {
            Some(Source::Modrinth) => PluginSource::Modrinth,
            Some(Source::Hangar) => PluginSource::Hangar,
            _ => continue,
        };
        let (Some(target), Some(project)) = (&item.target, &item.project_id) else {
            continue;
        };
        tracked
            .retain(|t| Some(&t.file_name) != item.file_name.as_ref() && &t.project_id != project);
        tracked.push(TrackedPlugin {
            file_name: target.file_name.clone(),
            source,
            project_id: project.clone(),
            version_id: target.version_id.clone(),
            version_number: target.version_number.clone(),
            title: item.title.clone(),
            icon_url: item.icon_url.clone(),
        });
    }
    write_tracking(server_dir, &tracked);
}
