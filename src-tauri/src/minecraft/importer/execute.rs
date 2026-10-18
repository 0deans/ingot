//! Staged instance import; publication is supplied by the application repository.
use super::models::{ImportError, ImportInstanceOptions, ImportReport};
use crate::minecraft::instance::InstanceConfig;
use crate::minecraft::metadata::MetadataError;
use std::{fs, io, path::Path};

pub(crate) const IGNORED_ENTRIES: &[&str] = &[
    "assets",
    "libraries",
    "versions",
    "natives",
    "jre",
    "java",
    "runtime",
    "launcher",
    "bin",
    ".fabric",
    ".quilt",
    ".mixin.out",
    ".gradle",
    ".cache",
    "logs",
    ".tlauncher",
    ".curseforge",
    ".modrinth",
    ".atlauncher",
    ".lock",
    "running.lock",
    ".ds_store",
    "thumbs.db",
];

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct CopyCounts {
    pub(crate) mods: u32,
    pub(crate) saves: u32,
}

fn invalid_entry() -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        "Import entries must be regular files or directories without links",
    )
}

fn directory_exists(path: &Path) -> io::Result<bool> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_dir() => {
            if crate::app::safe_enclosed_filename(path, ".import-check").is_none() {
                return Err(invalid_entry());
            }
            Ok(true)
        }
        Ok(_) => Err(invalid_entry()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error),
    }
}

fn checked_entry(entry: &fs::DirEntry, destination: &Path) -> io::Result<std::path::PathBuf> {
    let name = entry.file_name();
    let name = name.to_str().ok_or_else(invalid_entry)?;
    let path = entry.path();
    let source_parent = path.parent().ok_or_else(invalid_entry)?;
    if crate::app::safe_enclosed_filename(source_parent, name).is_none() {
        return Err(invalid_entry());
    }
    crate::app::safe_enclosed_filename(destination, name).ok_or_else(invalid_entry)
}

pub(crate) fn copy_dir_all(source: &Path, destination: &Path) -> io::Result<()> {
    if !directory_exists(source)? {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "Import source directory disappeared",
        ));
    }
    fs::create_dir_all(destination)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let target = checked_entry(&entry, destination)?;
        let kind = entry.file_type()?;
        if kind.is_dir() {
            copy_dir_all(&entry.path(), &target)?;
        } else if kind.is_file() {
            fs::copy(entry.path(), target)?;
        } else {
            return Err(invalid_entry());
        }
    }
    Ok(())
}

fn copy_optional_directory(source: &Path, destination: &Path) -> io::Result<()> {
    if directory_exists(source)? {
        copy_dir_all(source, destination)?;
    }
    Ok(())
}

fn copy_optional_file(source: &Path, destination: &Path) -> io::Result<()> {
    for path in [source, destination] {
        let parent = path.parent().ok_or_else(invalid_entry)?;
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(invalid_entry)?;
        if crate::app::safe_enclosed_filename(parent, name).is_none() {
            return Err(invalid_entry());
        }
    }
    match fs::symlink_metadata(source) {
        Ok(metadata) if metadata.is_file() => {
            fs::copy(source, destination)?;
        }
        Ok(_) => return Err(invalid_entry()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error),
    }
    Ok(())
}

#[derive(Clone, Copy)]
enum CountedEntry {
    Mod,
    World,
}

fn copy_counted_entries(
    source: &Path,
    destination: &Path,
    selection: CountedEntry,
) -> io::Result<u32> {
    if !directory_exists(source)? {
        return Ok(0);
    }
    fs::create_dir_all(destination)?;
    let mut count = 0u32;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let target = checked_entry(&entry, destination)?;
        let kind = entry.file_type()?;
        if !kind.is_dir() && !kind.is_file() {
            return Err(invalid_entry());
        }
        match selection {
            CountedEntry::World if kind.is_dir() => copy_dir_all(&entry.path(), &target)?,
            CountedEntry::Mod if kind.is_file() => {
                fs::copy(entry.path(), target)?;
            }
            _ => continue,
        }
        count = count.checked_add(1).ok_or_else(invalid_entry)?;
    }
    Ok(count)
}

pub(crate) fn copy_instance_contents(
    source: &Path,
    destination: &Path,
    options: &ImportInstanceOptions,
) -> io::Result<CopyCounts> {
    let mut counts = CopyCounts::default();
    if options.copy_mods {
        counts.mods = copy_counted_entries(
            &source.join("mods"),
            &destination.join("mods"),
            CountedEntry::Mod,
        )?;
    }
    if options.copy_saves {
        counts.saves = copy_counted_entries(
            &source.join("saves"),
            &destination.join("saves"),
            CountedEntry::World,
        )?;
    }
    for (enabled, name) in [
        (options.copy_configs, "config"),
        (options.copy_configs, "defaultconfigs"),
        (options.copy_resource_packs, "resourcepacks"),
        (options.copy_resource_packs, "shaderpacks"),
        (options.copy_screenshots, "screenshots"),
    ] {
        if enabled {
            copy_optional_directory(&source.join(name), &destination.join(name))?;
        }
    }
    for (enabled, name) in [
        (options.copy_options, "options.txt"),
        (options.copy_options, "optionsshaders.txt"),
        (options.copy_servers, "servers.dat"),
    ] {
        if enabled {
            copy_optional_file(&source.join(name), &destination.join(name))?;
        }
    }
    if options.copy_extra_data {
        copy_extra_data(source, destination)?;
    }
    Ok(counts)
}

fn copy_extra_data(source: &Path, destination: &Path) -> io::Result<()> {
    const HANDLED: &[&str] = &[
        "mods",
        "saves",
        "config",
        "defaultconfigs",
        "resourcepacks",
        "shaderpacks",
        "options.txt",
        "optionsshaders.txt",
        "servers.dat",
        "screenshots",
    ];
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let name = entry.file_name();
        let name = name.to_str().ok_or_else(invalid_entry)?.to_lowercase();
        if HANDLED.contains(&name.as_str()) || IGNORED_ENTRIES.contains(&name.as_str()) {
            continue;
        }
        let target = checked_entry(&entry, destination)?;
        let kind = entry.file_type()?;
        if kind.is_dir() {
            copy_dir_all(&entry.path(), &target)?;
        } else if kind.is_file() {
            fs::copy(entry.path(), target)?;
        } else {
            return Err(invalid_entry());
        }
    }
    Ok(())
}

pub(crate) fn execute_import_instance(
    instances_dir: &Path,
    options: ImportInstanceOptions,
    publish: impl FnOnce(&Path, InstanceConfig) -> Result<(), MetadataError>,
) -> Result<ImportReport, ImportError> {
    let source = Path::new(&options.source_path);
    if !directory_exists(source).map_err(ImportError::Io)? {
        return Err(ImportError::SourceNotFound(options.source_path));
    }
    let config = crate::minecraft::instance::prepare_instance(
        options.name.clone(),
        options.game_version.clone(),
        options.loader.clone(),
        options.loader_version.clone(),
    )
    .map_err(ImportError::InstanceCreate)?;
    let instance_id = config.id.clone();
    fs::create_dir_all(instances_dir).map_err(ImportError::Io)?;
    let source_root = source.canonicalize().map_err(ImportError::Io)?;
    let destination_root = instances_dir.canonicalize().map_err(ImportError::Io)?;
    if destination_root.starts_with(&source_root) {
        return Err(ImportError::OverlappingPaths);
    }
    let staging = tempfile::Builder::new()
        .prefix(".staging-import-")
        .tempdir_in(instances_dir)
        .map_err(ImportError::Io)?;
    let prepared = (|| {
        let counts =
            copy_instance_contents(source, staging.path(), &options).map_err(ImportError::Io)?;
        for directory in ["mods", "config", "saves", "resourcepacks"] {
            fs::create_dir_all(staging.path().join(directory)).map_err(ImportError::Io)?;
        }
        publish(staging.path(), config).map_err(ImportError::Metadata)?;
        Ok(counts)
    })();
    let counts = match prepared {
        Ok(counts) => counts,
        Err(primary) => {
            if let Err(cleanup) = staging.close() {
                // A durable publication intent may already own the renamed tree.
                if cleanup.kind() != std::io::ErrorKind::NotFound {
                    return Err(ImportError::Cleanup {
                        primary: Box::new(primary),
                        cleanup,
                    });
                }
            }
            return Err(primary);
        }
    };
    // Publication consumes the directory through rename.
    let _ = staging.keep();
    let message = format!(
        "Successfully imported instance with {} mods and {} worlds.",
        counts.mods, counts.saves
    );
    Ok(ImportReport {
        success: true,
        instance_id,
        name: options.name,
        mods_copied: counts.mods,
        saves_copied: counts.saves,
        message,
    })
}
