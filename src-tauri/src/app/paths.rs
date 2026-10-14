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

    pub(crate) fn accounts_file(&self) -> PathBuf {
        self.data_dir.join("accounts.json")
    }
}

/// Safely joins a relative path to `base_dir`, guaranteeing that the target path
/// stays completely enclosed within `base_dir` and does not escape via `..`, absolute paths,
/// Windows drive prefixes, UNC paths, alternate data streams, or trailing dot/space bypasses.
pub(crate) fn safe_enclosed_path(base_dir: &Path, rel: &str) -> Option<PathBuf> {
    if rel.is_empty() || rel.contains('\0') || rel.contains(':') {
        return None;
    }
    let p = Path::new(rel);
    for c in p.components() {
        match c {
            Component::Normal(part) => {
                let part_str = part.to_string_lossy();
                if part_str.ends_with(' ') || part_str.ends_with('.') {
                    return None;
                }
            }
            _ => return None,
        }
    }
    let full = base_dir.join(p);
    if full.starts_with(base_dir) {
        Some(full)
    } else {
        None
    }
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
            let full = base_dir.join(part);
            if full.starts_with(base_dir) {
                Some(full)
            } else {
                None
            }
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
