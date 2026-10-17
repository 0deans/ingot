use super::super::models::VersionChangeError;
use std::{fs, path::Path};

pub(super) fn rooted_path(
    root: &Path,
    relative: impl AsRef<Path>,
) -> Result<std::path::PathBuf, VersionChangeError> {
    let relative = relative.as_ref();
    let text = relative
        .to_str()
        .ok_or_else(|| VersionChangeError::InvalidPath(relative.display().to_string()))?;
    super::super::models::safe_rel_typed(text)?;
    let mut ancestor = root.to_path_buf();
    reject_link_ancestor(&ancestor)?;
    let mut components = relative.components().peekable();
    while let Some(component) = components.next() {
        ancestor.push(component);
        if components.peek().is_some() {
            reject_link_ancestor(&ancestor)?;
        }
    }
    Ok(ancestor)
}

pub(in crate::version_change) fn checked_path(
    root: &Path,
    path: &Path,
) -> Result<std::path::PathBuf, VersionChangeError> {
    let relative = path
        .strip_prefix(root)
        .map_err(|_| VersionChangeError::InvalidPath(path.display().to_string()))?;
    rooted_path(root, relative)
}

fn reject_link_ancestor(path: &Path) -> Result<(), VersionChangeError> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => {
            return Err(VersionChangeError::FileOp(format!(
                "Cannot inspect {}: {error}",
                path.display()
            )))
        }
    };
    #[cfg(windows)]
    let is_link = {
        use std::os::windows::fs::MetadataExt;
        metadata.file_attributes() & 0x400 != 0
    };
    #[cfg(not(windows))]
    let is_link = metadata.file_type().is_symlink();
    if is_link {
        return Err(VersionChangeError::InvalidPath(format!(
            "Linked transaction ancestor: {}",
            path.display()
        )));
    }
    Ok(())
}

fn normalize_path(path: &Path) -> std::path::PathBuf {
    let mut normalized = std::path::PathBuf::new();
    for comp in path.components() {
        match comp {
            std::path::Component::ParentDir => {
                normalized.pop();
            }
            std::path::Component::CurDir => {}
            c => normalized.push(c),
        }
    }
    normalized
}

#[cfg(windows)]
fn components_match(p_comp: &std::path::Component, r_comp: &std::path::Component) -> bool {
    use std::path::{Component, Prefix};
    match (p_comp, r_comp) {
        (Component::Prefix(p1), Component::Prefix(p2)) => match (p1.kind(), p2.kind()) {
            (Prefix::VerbatimDisk(d1), Prefix::VerbatimDisk(d2))
            | (Prefix::Disk(d1), Prefix::Disk(d2))
            | (Prefix::VerbatimDisk(d1), Prefix::Disk(d2))
            | (Prefix::Disk(d1), Prefix::VerbatimDisk(d2)) => d1.eq_ignore_ascii_case(&d2),
            _ => p1
                .as_os_str()
                .to_string_lossy()
                .eq_ignore_ascii_case(&p2.as_os_str().to_string_lossy()),
        },
        (Component::RootDir, Component::RootDir) => true,
        (Component::Normal(s1), Component::Normal(s2)) => s1
            .to_string_lossy()
            .eq_ignore_ascii_case(&s2.to_string_lossy()),
        _ => false,
    }
}

pub(crate) fn strip_contained_root(path: &Path, root: &Path) -> Option<std::path::PathBuf> {
    let norm_path = normalize_path(path);
    let norm_root = normalize_path(root);

    #[cfg(windows)]
    {
        let mut path_comps = norm_path.components();
        let mut root_comps = norm_root.components();

        loop {
            match (path_comps.next(), root_comps.next()) {
                (Some(p_comp), Some(r_comp)) => {
                    if !components_match(&p_comp, &r_comp) {
                        return None;
                    }
                }
                (Some(p_comp), None) => {
                    let mut remainder = std::path::PathBuf::from(p_comp.as_os_str());
                    for comp in path_comps {
                        remainder.push(comp.as_os_str());
                    }
                    return Some(remainder);
                }
                (None, None) => {
                    return Some(std::path::PathBuf::new());
                }
                (None, Some(_)) => {
                    return None;
                }
            }
        }
    }
    #[cfg(not(windows))]
    {
        norm_path
            .strip_prefix(&norm_root)
            .ok()
            .map(|p| p.to_path_buf())
    }
}

pub(crate) fn is_contained(source_entry: &Path, link_target: &Path, root: &Path) -> bool {
    let resolved = if link_target.is_absolute() {
        link_target.to_path_buf()
    } else if let Some(parent) = source_entry.parent() {
        parent.join(link_target)
    } else {
        source_entry.join(link_target)
    };

    strip_contained_root(&resolved, root).is_some()
}

fn copy_symlink(from: &Path, to: &Path, src_root: &Path, dst_root: &Path) -> std::io::Result<()> {
    let link_target = fs::read_link(from)?;
    if !is_contained(from, &link_target, src_root) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!(
                "Directory contains uncontained link {:?} -> {:?} escaping root {:?}",
                from, link_target, src_root
            ),
        ));
    }

    let remapped_target = if link_target.is_absolute() {
        let rel = strip_contained_root(&link_target, src_root).ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!(
                    "Directory contains uncontained link {:?} -> {:?} escaping root {:?}",
                    from, link_target, src_root
                ),
            )
        })?;
        dst_root.join(rel)
    } else {
        link_target.clone()
    };

    remove_path(to)?;

    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(&remapped_target, to)?;
    }
    #[cfg(windows)]
    {
        let is_dir = if link_target.is_absolute() {
            link_target.is_dir()
        } else if let Some(parent) = from.parent() {
            parent.join(&link_target).is_dir()
        } else {
            false
        };
        if is_dir {
            std::os::windows::fs::symlink_dir(&remapped_target, to)?;
        } else {
            std::os::windows::fs::symlink_file(&remapped_target, to)?;
        }
    }
    #[cfg(not(any(unix, windows)))]
    {
        return Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "Symbolic links are not supported on this platform",
        ));
    }
    Ok(())
}

/// Copies a file or a folder, preserving contained symlinks and rejecting uncontained links.
/// Contained relative symlinks are preserved. Contained absolute symlinks are remapped between
/// `src_root` and `dst_root`.
pub fn copy_path(from: &Path, to: &Path) -> std::io::Result<()> {
    copy_path_contained(from, to, from, to)
}

pub fn copy_path_contained(
    from: &Path,
    to: &Path,
    src_root: &Path,
    dst_root: &Path,
) -> std::io::Result<()> {
    let meta = fs::symlink_metadata(from)?;
    let kind = meta.file_type();
    if kind.is_symlink() {
        if let Some(parent) = to.parent() {
            fs::create_dir_all(parent)?;
        }
        return copy_symlink(from, to, src_root, dst_root);
    }
    if meta.is_file() {
        if let Some(parent) = to.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(from, to)?;
        return Ok(());
    }
    fs::create_dir_all(to)?;
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let entry_kind = entry.file_type()?;
        let dest = to.join(entry.file_name());
        if entry_kind.is_symlink() {
            if let Some(parent) = dest.parent() {
                fs::create_dir_all(parent)?;
            }
            copy_symlink(&entry.path(), &dest, src_root, dst_root)?;
        } else if entry_kind.is_dir() {
            copy_path_contained(&entry.path(), &dest, src_root, dst_root)?;
        } else if entry_kind.is_file() {
            if let Some(parent) = dest.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(entry.path(), &dest)?;
        }
    }
    Ok(())
}

pub fn remove_path(path: &Path) -> std::io::Result<()> {
    match fs::symlink_metadata(path) {
        Ok(meta) if meta.is_dir() => fs::remove_dir_all(path),
        Ok(_) => fs::remove_file(path),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e),
    }
}

/// Moves a file or folder, creating the destination's parent
pub fn move_path(from: &Path, to: &Path) -> Result<(), VersionChangeError> {
    if let Some(parent) = to.parent() {
        fs::create_dir_all(parent).map_err(|e| {
            VersionChangeError::FileOp(format!("Failed to create {}: {e}", parent.display()))
        })?;
    }
    fs::rename(from, to)
        .map_err(|e| VersionChangeError::FileOp(format!("Failed to move {}: {e}", from.display())))
}
