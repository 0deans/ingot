use std::path::{Component, Path, PathBuf};

pub(crate) struct AppPaths {
    data_dir: PathBuf,
}

impl AppPaths {
    pub(crate) fn new(data_dir: PathBuf) -> Self {
        Self { data_dir }
    }

    pub(crate) fn settings_file(&self) -> PathBuf {
        self.data_dir.join("settings.json")
    }

    pub(crate) fn instances_file(&self) -> PathBuf {
        self.data_dir.join("instances.json")
    }

    pub(crate) fn instances_dir(&self) -> PathBuf {
        self.data_dir.join("instances")
    }

    pub(crate) fn servers_file(&self) -> PathBuf {
        self.data_dir.join("servers.json")
    }

    pub(crate) fn servers_dir(&self) -> PathBuf {
        self.data_dir.join("servers")
    }

    pub(crate) fn accounts_file(&self) -> PathBuf {
        self.data_dir.join("accounts.json")
    }
}

/// Uses a portable archive policy: both separators are recognized, Windows
/// aliases are rejected on every platform. The trusted root and its descendants
/// may not be links/reparse points; OS aliases above the root remain supported.
/// Callers exclusively own the destination while checking and writing.
pub(crate) fn safe_enclosed_path(base_dir: &Path, rel: &str) -> Option<PathBuf> {
    if rel.is_empty() || rel.contains('\0') || rel.contains(':') {
        return None;
    }
    let normalized = rel.replace('\\', "/");
    if normalized.starts_with('/') {
        return None;
    }
    if normalized
        .split('/')
        .any(|part| matches!(part, "" | "." | ".."))
    {
        return None;
    }
    let p = Path::new(&normalized);
    for c in p.components() {
        match c {
            Component::Normal(part) => {
                let part_str = part.to_string_lossy();
                let stem = part_str.split('.').next()?.to_ascii_uppercase();
                let reserved = matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
                    || (stem.len() == 4
                        && (stem.starts_with("COM") || stem.starts_with("LPT"))
                        && matches!(stem.as_bytes()[3], b'1'..=b'9'));
                if part_str.ends_with(' ') || part_str.ends_with('.') || reserved {
                    return None;
                }
            }
            _ => return None,
        }
    }
    let full = base_dir.join(p);
    if full.starts_with(base_dir) && existing_components_are_unlinked(base_dir, p) {
        Some(full)
    } else {
        None
    }
}

fn existing_components_are_unlinked(base: &Path, relative: &Path) -> bool {
    let mut current = base.to_path_buf();
    let components = std::iter::once(None).chain(relative.components().map(Some));
    for component in components {
        if let Some(component) = component {
            current.push(component);
        }
        match std::fs::symlink_metadata(&current) {
            Ok(metadata) => {
                if metadata.file_type().is_symlink() {
                    return false;
                }
                #[cfg(windows)]
                {
                    use std::os::windows::fs::MetadataExt;
                    if metadata.file_attributes() & 0x400 != 0 {
                        return false;
                    }
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => return false,
        }
    }
    true
}

/// Safely validates that `name` is a single filename (not containing directory separators
/// or path traversal) and joins it to `base_dir`.
pub(crate) fn safe_enclosed_filename(base_dir: &Path, name: &str) -> Option<PathBuf> {
    if name.is_empty() || name.contains(['/', '\\', ':', '\0']) {
        return None;
    }
    let p = Path::new(name);
    let mut components = p.components();
    match (components.next(), components.next()) {
        (Some(Component::Normal(part)), None) => {
            let part_str = part.to_string_lossy();
            if part_str.ends_with(' ') || part_str.ends_with('.') {
                return None;
            }
            safe_enclosed_path(base_dir, &part_str)
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_windows_device_and_backslash_traversal_on_every_platform() {
        let temp = tempfile::tempdir().unwrap();
        for name in [
            "..\\escape",
            "mods\\..\\escape",
            "//host/share",
            "CON",
            "aux.txt",
            "COM1.jar",
            "a/./b",
            "a//b",
        ] {
            assert!(safe_enclosed_path(temp.path(), name).is_none(), "{name}");
        }
        assert_eq!(
            safe_enclosed_path(temp.path(), "mods\\valid.jar"),
            Some(temp.path().join("mods/valid.jar"))
        );
    }

    #[cfg(unix)]
    #[test]
    fn rejects_linked_destination_and_linked_root() {
        let temp = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        std::os::unix::fs::symlink(outside.path(), temp.path().join("linked")).unwrap();
        assert!(safe_enclosed_path(temp.path(), "linked/escape").is_none());
        assert!(safe_enclosed_path(&temp.path().join("linked"), "escape").is_none());
        assert!(!outside.path().join("escape").exists());
    }

    #[test]
    fn safe_enclosed_path_validates_containment() {
        let base = Path::new("/var/data/minecraft");

        // Valid paths
        assert_eq!(
            safe_enclosed_path(base, "mods/create.jar"),
            Some(base.join("mods/create.jar"))
        );
        assert_eq!(
            safe_enclosed_path(base, "config/sub/options.txt"),
            Some(base.join("config/sub/options.txt"))
        );
        assert_eq!(
            safe_enclosed_path(base, "manifest.json"),
            Some(base.join("manifest.json"))
        );

        // Path traversal / zip slip attacks
        assert_eq!(safe_enclosed_path(base, "../evil.exe"), None);
        assert_eq!(safe_enclosed_path(base, "mods/../../evil.exe"), None);
        assert_eq!(safe_enclosed_path(base, "/etc/passwd"), None);
        assert_eq!(safe_enclosed_path(base, "\\Windows\\System32"), None);
        assert_eq!(safe_enclosed_path(base, "C:\\Windows\\System32"), None);
        assert_eq!(safe_enclosed_path(base, "D:file.txt"), None);
        assert_eq!(safe_enclosed_path(base, "file.txt:stream"), None);
        assert_eq!(safe_enclosed_path(base, "file.txt\0null"), None);
        assert_eq!(safe_enclosed_path(base, ""), None);
        assert_eq!(safe_enclosed_path(base, "trailing. "), None);
        assert_eq!(safe_enclosed_path(base, "trailing."), None);
    }

    #[test]
    fn safe_enclosed_filename_validates_single_file() {
        let base = Path::new("/var/data/minecraft/mods");

        // Valid single filenames
        assert_eq!(
            safe_enclosed_filename(base, "create.jar"),
            Some(base.join("create.jar"))
        );
        assert_eq!(
            safe_enclosed_filename(base, "fabric-api-0.90.jar"),
            Some(base.join("fabric-api-0.90.jar"))
        );

        // Subdirectories / path traversal rejected
        assert_eq!(safe_enclosed_filename(base, "sub/create.jar"), None);
        assert_eq!(safe_enclosed_filename(base, "sub\\create.jar"), None);
        assert_eq!(safe_enclosed_filename(base, "../create.jar"), None);
        assert_eq!(safe_enclosed_filename(base, "create.jar:stream"), None);
        assert_eq!(safe_enclosed_filename(base, "create.jar\0null"), None);
        assert_eq!(safe_enclosed_filename(base, ""), None);
        assert_eq!(safe_enclosed_filename(base, "create.jar "), None);
        assert_eq!(safe_enclosed_filename(base, "create.jar."), None);
    }
}
