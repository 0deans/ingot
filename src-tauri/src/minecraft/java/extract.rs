use super::{ArchiveFormat, JavaError};
use std::path::Path;
pub(super) fn extract_runtime(
    archive: &Path,
    destination: &Path,
    format: ArchiveFormat,
) -> Result<(), JavaError> {
    let file = std::fs::File::open(archive)?;
    match format {
        ArchiveFormat::Zip => {
            let mut archive = zip::ZipArchive::new(file)?;
            crate::app::extract_zip_entries(&mut archive, destination, "", None).map_err(
                |error| match error {
                    crate::app::ArchiveError::UnsafeEntry => JavaError::UnsafeArchive,
                    crate::app::ArchiveError::Zip(error) => JavaError::Zip(error),
                    crate::app::ArchiveError::Io(error) => JavaError::Io(error),
                },
            )?;
        }
        ArchiveFormat::TarGzip => {
            let gzip = flate2::read::GzDecoder::new(file);
            let mut archive = tar::Archive::new(gzip);
            let mut links = Vec::new();
            for entry in archive.entries()? {
                let mut entry = entry?;
                let kind = entry.header().entry_type();
                if !kind.is_file() && !kind.is_dir() && !kind.is_symlink() && !kind.is_hard_link() {
                    return Err(JavaError::UnsafeArchive);
                }
                let path = entry.path()?;
                let relative = path
                    .to_str()
                    .ok_or(JavaError::UnsafeArchive)?
                    .trim_end_matches('/');
                let output = crate::app::safe_enclosed_path(destination, relative)
                    .ok_or(JavaError::UnsafeArchive)?;
                if kind.is_symlink() || kind.is_hard_link() {
                    let target = entry.link_name()?.ok_or(JavaError::UnsafeArchive)?;
                    let target = target.to_str().ok_or(JavaError::UnsafeArchive)?;
                    let parent = if kind.is_symlink() {
                        Path::new(relative)
                            .parent()
                            .ok_or(JavaError::UnsafeArchive)?
                    } else {
                        Path::new("")
                    };
                    let resolved = resolve_link(parent, target)?;
                    crate::app::safe_enclosed_path(destination, &resolved)
                        .ok_or(JavaError::UnsafeArchive)?;
                    links.push(FileLink {
                        name: relative.to_owned(),
                        target: resolved,
                    });
                    continue;
                }
                if kind.is_dir() {
                    std::fs::create_dir_all(output)?;
                    continue;
                }
                if let Some(parent) = output.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                let mut file = std::fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(output)?;
                std::io::copy(&mut entry, &mut file)?;
                file.sync_all()?;
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    file.set_permissions(std::fs::Permissions::from_mode(
                        entry.header().mode()? & 0o777,
                    ))?;
                }
            }
            materialize_links(destination, links)?;
        }
    }
    Ok(())
}

struct FileLink {
    name: String,
    target: String,
}

fn resolve_link(parent: &Path, target: &str) -> Result<String, JavaError> {
    let target = target.replace('\\', "/");
    if target.is_empty() || target.starts_with('/') || target.contains(':') {
        return Err(JavaError::UnsafeArchive);
    }
    let parent = parent.to_str().ok_or(JavaError::UnsafeArchive)?;
    let mut segments: Vec<&str> = parent.split('/').filter(|part| !part.is_empty()).collect();
    for segment in target.split('/') {
        match segment {
            "" | "." => {}
            ".." => {
                segments.pop().ok_or(JavaError::UnsafeArchive)?;
            }
            _ => segments.push(segment),
        }
    }
    if segments.is_empty() {
        return Err(JavaError::UnsafeArchive);
    }
    Ok(segments.join("/"))
}

fn materialize_links(destination: &Path, mut pending: Vec<FileLink>) -> Result<(), JavaError> {
    // JDK bundles contain file aliases. Copy only resolved, contained regular
    // files; never create filesystem links or allow directory aliases.
    while !pending.is_empty() {
        let previous_count = pending.len();
        let mut unresolved = Vec::new();
        for link in pending {
            let target = crate::app::safe_enclosed_path(destination, &link.target)
                .ok_or(JavaError::UnsafeArchive)?;
            let metadata = match std::fs::symlink_metadata(&target) {
                Ok(metadata) => metadata,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    unresolved.push(link);
                    continue;
                }
                Err(error) => return Err(error.into()),
            };
            if !metadata.is_file() {
                return Err(JavaError::UnsafeArchive);
            }
            let output = crate::app::safe_enclosed_path(destination, &link.name)
                .ok_or(JavaError::UnsafeArchive)?;
            if let Some(parent) = output.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let mut file = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&output)?;
            let mut source = std::fs::File::open(&target)?;
            std::io::copy(&mut source, &mut file)?;
            file.set_permissions(metadata.permissions())?;
            file.sync_all()?;
        }
        if unresolved.len() == previous_count {
            return Err(JavaError::UnsafeArchive);
        }
        pending = unresolved;
    }
    Ok(())
}
