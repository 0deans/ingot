use std::path::Path;
use std::process::Stdio;

pub fn build_horizon_command(
    executable: &Path,
    instance_dir: &Path,
    resources_dir: Option<&Path>,
    session_file: Option<&Path>,
    username: Option<&str>,
    fullscreen: bool,
) -> tokio::process::Command {
    let mut command = tokio::process::Command::new(executable);
    command
        .current_dir(instance_dir)
        .arg("--game-dir")
        .arg(instance_dir)
        .arg("--home")
        .arg(instance_dir)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());

    if let Some(resources) = resources_dir {
        command.arg("--resources").arg(resources);
    }

    if let Some(session_path) = session_file {
        command.arg("--session-file").arg(session_path);
    } else if let Some(user) = username {
        command.arg("--username").arg(user);
    }

    if fullscreen {
        command.arg("--fullscreen");
    }

    command
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_horizon_command() {
        let exe = Path::new("horizon-client.exe");
        let inst = Path::new("/instances/test");
        let res = Path::new("/engines/horizon/resources");
        let session = Path::new("/instances/test/sessions/session-123.json");

        let cmd = build_horizon_command(exe, inst, Some(res), Some(session), None, true);
        let debug = format!("{:?}", cmd);
        assert!(debug.contains("--game-dir"));
        assert!(debug.contains("--resources"));
        assert!(debug.contains("--session-file"));
        assert!(debug.contains("--fullscreen"));
    }
}
