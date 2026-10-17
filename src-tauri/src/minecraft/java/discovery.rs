use super::JavaError;
use std::path::{Path, PathBuf};

pub(crate) trait JavaProbe: Send + Sync {
    fn major_version(&self, binary: &Path) -> Result<u32, JavaError>;
}

pub(crate) struct NativeProbe;

impl JavaProbe for NativeProbe {
    fn major_version(&self, binary: &Path) -> Result<u32, JavaError> {
        let console_binary = binary.with_file_name("java.exe");
        let probe_binary = if cfg!(windows) && console_binary.is_file() {
            console_binary.as_path()
        } else {
            binary
        };
        let mut command = std::process::Command::new(probe_binary);
        command.arg("-version");
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x08000000);
        }
        let output = command.output()?;
        if !output.status.success() {
            return Err(JavaError::ProbeFailed);
        }
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        parse_version(&stderr)
            .or_else(|| parse_version(&stdout))
            .ok_or(JavaError::InvalidVersion)
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

pub(super) fn validate_binary(
    binary: PathBuf,
    required: u32,
    probe: &dyn JavaProbe,
) -> Result<PathBuf, JavaError> {
    let actual = probe.major_version(&binary)?;
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
