use super::{probe::JavaProbe, JavaError};
use std::path::{Path, PathBuf};

pub(crate) fn resolve_probe_binary(binary: &Path) -> PathBuf {
    let console_binary = binary.with_file_name("java.exe");
    if cfg!(windows) && console_binary.is_file() {
        console_binary
    } else {
        binary.to_path_buf()
    }
}

pub(super) fn executable_name() -> &'static str {
    if cfg!(windows) {
        "javaw.exe"
    } else {
        "java"
    }
}

pub(super) fn find_binary(directory: &Path) -> Result<Option<PathBuf>, JavaError> {
    Ok(find_binaries(directory)?.into_iter().next())
}

pub(super) fn find_binaries(directory: &Path) -> Result<Vec<PathBuf>, JavaError> {
    let mut binaries = Vec::new();
    let direct = directory.join("bin").join(executable_name());
    if direct.is_file() {
        binaries.push(direct);
    }
    let bundle = directory.join("Contents/Home/bin").join(executable_name());
    if bundle.is_file() {
        binaries.push(bundle);
    }
    let entries = match std::fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(binaries),
        Err(error) => return Err(error.into()),
    };
    for entry in entries {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let nested = entry.path().join("bin").join(executable_name());
        if nested.is_file() {
            binaries.push(nested);
        }
        // macOS JDK bundles place the executable below Contents/Home.
        let bundle = entry
            .path()
            .join("Contents/Home/bin")
            .join(executable_name());
        if bundle.is_file() {
            binaries.push(bundle);
        }
    }
    binaries.sort();
    Ok(binaries)
}

pub(super) async fn validate_binary(
    binary: PathBuf,
    required: u32,
    probe: &dyn JavaProbe,
) -> Result<PathBuf, JavaError> {
    let actual = probe.major_version(&binary).await?;
    if actual != required {
        return Err(JavaError::VersionMismatch { required, actual });
    }
    Ok(binary)
}

pub(super) fn parse_version(text: &str) -> Option<u32> {
    for line in text.lines().filter(|line| line.contains("version")) {
        let (_, quoted) = line.split_once('"')?;
        let (version, _) = quoted.split_once('"')?;
        let version = version.strip_prefix("1.").unwrap_or(version);
        let major = version.split(['.', '-', '_']).next()?.parse().ok()?;
        if major > 0 {
            return Some(major);
        }
    }
    None
}
