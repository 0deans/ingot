use super::models::{
    dir_size_typed, safe_file_name_typed, sha1_hex, Target, VersionChangeError, WorldInfo,
};
use super::sources;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct LocalFile {
    pub folder: usize,
    pub file_name: String,
    pub sha1: String,
    pub fingerprint: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct DiscoveredEntry {
    pub path: PathBuf,
    pub file_name: String,
    pub is_dir: bool,
}

fn default_read_dir(path: &Path) -> std::io::Result<Vec<std::io::Result<DiscoveredEntry>>> {
    let entries = fs::read_dir(path)?;
    let mut out = Vec::new();
    for entry in entries {
        match entry {
            Ok(e) => {
                let is_dir = match e.file_type() {
                    Ok(ft) => ft.is_dir(),
                    Err(err) => {
                        out.push(Err(err));
                        continue;
                    }
                };
                let file_name = e.file_name().to_string_lossy().into_owned();
                out.push(Ok(DiscoveredEntry {
                    path: e.path(),
                    file_name,
                    is_dir,
                }));
            }
            Err(err) => out.push(Err(err)),
        }
    }
    Ok(out)
}

pub fn scan(target: &Target) -> Result<Vec<LocalFile>, VersionChangeError> {
    scan_with(target, default_read_dir, |p| fs::read(p))
}

pub(crate) fn scan_with<DirFn, ReadFn>(
    target: &Target,
    mut read_dir_fn: DirFn,
    mut read_file_fn: ReadFn,
) -> Result<Vec<LocalFile>, VersionChangeError>
where
    DirFn: FnMut(&Path) -> Result<Vec<std::io::Result<DiscoveredEntry>>, std::io::Error>,
    ReadFn: FnMut(&Path) -> std::io::Result<Vec<u8>>,
{
    let mut out = Vec::new();
    for (index, folder) in target.folders.iter().enumerate() {
        let folder_path = target.root.join(folder.dir);
        let entries = match read_dir_fn(&folder_path) {
            Ok(entries) => entries,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                // Missing optional directory remains acceptable
                continue;
            }
            Err(source) => {
                return Err(VersionChangeError::ScanDirectory {
                    path: folder_path,
                    source,
                });
            }
        };

        for entry in entries {
            let entry = entry.map_err(|source| VersionChangeError::ScanDirectory {
                path: folder_path.clone(),
                source,
            })?;

            if entry.is_dir {
                // Subdirectories are irrelevant entries and intentionally skipped
                continue;
            }

            let file_name = entry.file_name;
            if file_name.contains(['/', '\\', ':']) {
                // Path traversal or separator characters violate containment
                safe_file_name_typed(&file_name, folder.kind)?;
            }

            if file_name.starts_with('.') {
                // Dotfiles / hidden files are intentionally skipped
                continue;
            }

            let expected_ext = folder.kind.extension();
            if !file_name.to_ascii_lowercase().ends_with(expected_ext) {
                // Non-matching file extensions for this content kind are intentionally skipped
                continue;
            }

            // Preserves filename and containment protections (validates no traversal/separators)
            safe_file_name_typed(&file_name, folder.kind)?;

            let data =
                read_file_fn(&entry.path).map_err(|source| VersionChangeError::ScanFile {
                    path: entry.path.clone(),
                    source,
                })?;

            out.push(LocalFile {
                folder: index,
                file_name,
                sha1: sha1_hex(&data),
                fingerprint: folder
                    .scope
                    .curseforge
                    .then(|| sources::cf_fingerprint(&data)),
            });
        }
    }
    out.sort_unstable_by_key(|f| f.file_name.to_lowercase());
    Ok(out)
}

pub fn list_worlds(target: &Target) -> Result<Vec<WorldInfo>, VersionChangeError> {
    list_worlds_with(target, dir_size_typed)
}

pub(crate) fn list_worlds_with<F>(
    target: &Target,
    mut dir_size_fn: F,
) -> Result<Vec<WorldInfo>, VersionChangeError>
where
    F: FnMut(&Path) -> std::io::Result<u64>,
{
    let mut worlds = Vec::with_capacity(target.worlds.len());
    for w in &target.worlds {
        let path = target.root.join(w);
        let bytes = dir_size_fn(&path).map_err(|source| VersionChangeError::ScanDirectory {
            path: path.clone(),
            source,
        })? as f64;
        worlds.push(WorldInfo {
            folder_name: w.clone(),
            bytes,
        });
    }
    Ok(worlds)
}

#[cfg(test)]
mod tests;
