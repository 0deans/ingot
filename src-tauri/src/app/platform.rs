use std::{path::Path, process::Command};

#[derive(Debug, thiserror::Error)]
pub(crate) enum OpenDirectoryError {
    #[error("Failed to access the file manager target: {0}")]
    Target(#[source] std::io::Error),
    #[error("Failed to open the system file manager: {0}")]
    Spawn(#[source] std::io::Error),
    #[error("The system file manager is unsupported on this platform")]
    Unsupported,
}

#[derive(Clone, Copy)]
enum Platform {
    Windows,
    Macos,
    Linux,
    Unsupported,
}

impl Platform {
    fn current() -> Self {
        match std::env::consts::OS {
            "windows" => Self::Windows,
            "macos" => Self::Macos,
            "linux" => Self::Linux,
            _ => Self::Unsupported,
        }
    }
}

#[derive(Clone, Copy)]
enum Action<'a> {
    Open(&'a Path),
    Reveal(&'a Path),
}

fn command(action: Action<'_>, platform: Platform) -> Result<Command, OpenDirectoryError> {
    let program = match platform {
        Platform::Windows => "explorer",
        Platform::Macos => "open",
        Platform::Linux => "xdg-open",
        Platform::Unsupported => return Err(OpenDirectoryError::Unsupported),
    };
    let mut command = Command::new(program);
    match (action, platform) {
        (Action::Reveal(path), Platform::Windows) => {
            command.arg("/select,").arg(path);
        }
        (Action::Reveal(path), Platform::Macos) => {
            command.arg("-R").arg(path);
        }
        (Action::Reveal(path), Platform::Linux) => {
            command.arg(path.parent().unwrap_or(path));
        }
        (Action::Open(path), _) => {
            command.arg(path);
        }
        (_, Platform::Unsupported) => return Err(OpenDirectoryError::Unsupported),
    }
    Ok(command)
}

fn execute(
    action: Action<'_>,
    platform: Platform,
    spawn: impl FnOnce(&mut Command) -> std::io::Result<()>,
) -> Result<(), OpenDirectoryError> {
    let mut command = command(action, platform)?;
    spawn(&mut command).map_err(OpenDirectoryError::Spawn)
}

fn native_execute(action: Action<'_>) -> Result<(), OpenDirectoryError> {
    execute(action, Platform::current(), |command| {
        command.spawn().map(|_| ())
    })
}

pub(crate) fn open_directory(directory: &Path) -> Result<(), OpenDirectoryError> {
    native_execute(Action::Open(directory))
}

pub(crate) fn reveal_file(path: &Path) -> Result<(), OpenDirectoryError> {
    let exists = path.try_exists().map_err(OpenDirectoryError::Target)?;
    if !exists {
        return Err(OpenDirectoryError::Target(std::io::Error::from(
            std::io::ErrorKind::NotFound,
        )));
    }
    native_execute(Action::Reveal(path))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsStr;

    #[test]
    fn reveal_commands_preserve_paths_and_platform_selection() {
        let path = Path::new("captures/a shot.png");
        for (platform, program, args) in [
            (
                Platform::Windows,
                "explorer",
                vec![OsStr::new("/select,"), path.as_os_str()],
            ),
            (
                Platform::Macos,
                "open",
                vec![OsStr::new("-R"), path.as_os_str()],
            ),
            (Platform::Linux, "xdg-open", vec![OsStr::new("captures")]),
        ] {
            let command = command(Action::Reveal(path), platform).unwrap();
            assert_eq!(command.get_program(), program);
            assert_eq!(command.get_args().collect::<Vec<_>>(), args);
        }
    }

    #[test]
    fn unsupported_platform_does_not_spawn_or_report_success() {
        for action in [
            Action::Open(Path::new("captures")),
            Action::Reveal(Path::new("shot.png")),
        ] {
            let result = execute(action, Platform::Unsupported, |_| panic!("must not spawn"));
            assert!(matches!(result, Err(OpenDirectoryError::Unsupported)));
        }
    }

    #[test]
    fn spawn_failure_retains_original_io_error() {
        let error = execute(Action::Open(Path::new("captures")), Platform::Linux, |_| {
            Err(std::io::Error::from(std::io::ErrorKind::PermissionDenied))
        })
        .unwrap_err();
        assert!(
            matches!(error, OpenDirectoryError::Spawn(source) if source.kind() == std::io::ErrorKind::PermissionDenied)
        );
    }
    #[test]
    fn revealing_a_missing_file_preserves_not_found_without_spawning() {
        let directory = tempfile::tempdir().unwrap();
        let result = reveal_file(&directory.path().join("missing.png"));
        assert!(
            matches!(result, Err(OpenDirectoryError::Target(source)) if source.kind() == std::io::ErrorKind::NotFound)
        );
    }
}
