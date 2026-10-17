use super::models::{PluginSource, TrackedPlugin};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub fn tracking_path(server_dir: &Path) -> PathBuf {
    server_dir.join(".ingot").join("plugins.json")
}

#[derive(Debug, thiserror::Error)]
pub enum TrackingError {
    #[error("Cannot read plugin tracking at {path}: {source}")]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("Invalid plugin tracking at {path}: {source}")]
    Decode {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
    #[error("Cannot publish plugin tracking: {0}")]
    Write(#[source] std::io::Error),
    #[error("Invalid plugin tracking entry: {0}")]
    InvalidEntry(&'static str),
}

pub fn read_tracking(server_dir: &Path) -> Result<Vec<TrackedPlugin>, TrackingError> {
    let path = tracking_path(server_dir);
    let bytes = match std::fs::read(&path) {
        Ok(bytes) => bytes,
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(source) => return Err(TrackingError::Read { path, source }),
    };
    let tracked: Vec<TrackedPlugin> =
        serde_json::from_slice(&bytes).map_err(|source| TrackingError::Decode { path, source })?;
    for item in &tracked {
        if super::metadata::safe_jar_name(&item.file_name).is_err()
            || crate::app::safe_enclosed_filename(server_dir, &item.file_name).is_none()
        {
            return Err(TrackingError::InvalidEntry("file_name"));
        }
        if item.project_id.trim().is_empty() || item.version_id.trim().is_empty() {
            return Err(TrackingError::InvalidEntry("project_id or version_id"));
        }
    }
    Ok(tracked)
}

pub fn write_tracking_typed(
    server_dir: &Path,
    tracked: &[TrackedPlugin],
) -> Result<(), std::io::Error> {
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

pub fn write_tracking(server_dir: &Path, tracked: &[TrackedPlugin]) -> Result<(), TrackingError> {
    write_tracking_typed(server_dir, tracked).map_err(TrackingError::Write)
}

/// Hangar has no lookup by file, so version changes need the installed project's identity.
pub(crate) fn tracked_hangar(
    server_dir: &Path,
) -> Result<HashMap<String, crate::version_change::models::TrackedHangarPlugin>, TrackingError> {
    Ok(read_tracking(server_dir)?
        .into_iter()
        .filter(|t| t.source == PluginSource::Hangar)
        .map(|t| {
            (
                t.file_name,
                crate::version_change::models::TrackedHangarPlugin {
                    project_id: t.project_id,
                    version_id: t.version_id,
                    title: t.title,
                    icon_url: t.icon_url,
                },
            )
        })
        .collect())
}

/// After a version change: remembers the files it installed (from Modrinth or Hangar)
/// in place of the ones they replaced
pub(crate) fn track_version_change(
    server_dir: &Path,
    items: &[crate::version_change::PlanItem],
) -> Result<(), TrackingError> {
    use crate::version_change::{ItemAction, Source};
    let mut tracked = read_tracking(server_dir)?;
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
    write_tracking(server_dir, &tracked)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_tracking_is_distinct_from_corrupt_and_unreadable_tracking() {
        let directory = tempfile::tempdir().unwrap();
        assert!(read_tracking(directory.path()).unwrap().is_empty());
        let path = tracking_path(directory.path());
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, b"{ incomplete").unwrap();
        assert!(matches!(
            read_tracking(directory.path()),
            Err(TrackingError::Decode { .. })
        ));
        assert_eq!(std::fs::read(&path).unwrap(), b"{ incomplete");
        std::fs::remove_file(&path).unwrap();
        std::fs::create_dir(&path).unwrap();
        assert!(matches!(
            read_tracking(directory.path()),
            Err(TrackingError::Read { .. })
        ));
    }

    #[test]
    fn rejected_version_tracking_keeps_corrupt_bytes() {
        let directory = tempfile::tempdir().unwrap();
        let path = tracking_path(directory.path());
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, b"invalid").unwrap();
        assert!(track_version_change(directory.path(), &[]).is_err());
        assert_eq!(std::fs::read(path).unwrap(), b"invalid");
    }
}
