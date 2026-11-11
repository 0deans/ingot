use super::super::models::HorizonError;
use std::path::Path;
use std::process::Stdio;
use tokio::io::AsyncBufReadExt;

pub(super) struct StepProgressScope {
    pub(super) phase: &'static str,
    pub(super) base_percent: usize,
    pub(super) max_percent: usize,
}

pub(super) async fn run_preparer_step_streaming<FProg>(
    preparer_bin: &Path,
    preparer_root: &Path,
    java_home: Option<&Path>,
    args: &[&str],
    scope: StepProgressScope,
    report_prog: &FProg,
) -> Result<(), HorizonError>
where
    FProg: Fn(&str, usize, usize, &str) + Send + Sync,
{
    let mut cmd = tokio::process::Command::new(preparer_bin);
    cmd.current_dir(preparer_root).args(args);

    if let Some(java_home) = java_home {
        let java_bin_dir = java_home.join("bin");
        let current_path = std::env::var_os("PATH").unwrap_or_default();
        let mut new_path = std::ffi::OsString::new();
        new_path.push(&java_bin_dir);
        #[cfg(target_os = "windows")]
        new_path.push(";");
        #[cfg(not(target_os = "windows"))]
        new_path.push(":");
        new_path.push(current_path);

        cmd.env("JAVA_HOME", java_home).env("PATH", new_path);
    }

    #[cfg(windows)]
    cmd.creation_flags(0x08000000);

    let mut child = cmd
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| {
            HorizonError::Preparation(format!("Failed to execute {}: {e}", preparer_bin.display()))
        })?;

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| HorizonError::Preparation("Failed to open child stdout".into()))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| HorizonError::Preparation("Failed to open child stderr".into()))?;

    let mut stdout_reader = tokio::io::BufReader::new(stdout).lines();
    let mut stderr_reader = tokio::io::BufReader::new(stderr).lines();

    let mut stderr_lines = Vec::new();
    let mut current_percent = scope.base_percent;

    loop {
        tokio::select! {
            line_res = stdout_reader.next_line() => {
                match line_res {
                    Ok(Some(text)) => {
                        let trimmed = text.trim();
                        if !trimmed.is_empty() {
                            if current_percent < scope.max_percent {
                                current_percent += 1;
                            }
                            report_prog(scope.phase, current_percent, 100, trimmed);
                        }
                    }
                    Ok(None) => break,
                    Err(_) => break,
                }
            }
            err_res = stderr_reader.next_line() => {
                if let Ok(Some(text)) = err_res {
                    let trimmed = text.trim().to_string();
                    if !trimmed.is_empty() {
                        stderr_lines.push(trimmed);
                    }
                }
            }
        }
    }

    let status = child.wait().await?;
    if !status.success() {
        let err_msg = stderr_lines.join("\n");
        return Err(HorizonError::Preparation(format!(
            "horizon-prepare exited with code {}: {}",
            status.code().unwrap_or(-1),
            err_msg
        )));
    }

    Ok(())
}

pub(super) async fn import_horizon_assets<FProg>(
    preparer_bin: &Path,
    preparer_root: &Path,
    data_dir: &Path,
    java_home: Option<&Path>,
    report_prog: &FProg,
) -> Result<(), HorizonError>
where
    FProg: Fn(&str, usize, usize, &str) + Send + Sync,
{
    let data_dir_str = data_dir.to_string_lossy();
    let root_str = preparer_root.to_string_lossy();

    let steps = [
        (
            "packs",
            "Importing Resource Packs",
            "Importing default packs...",
            72,
            77,
        ),
        (
            "panorama",
            "Importing Panorama",
            "Importing menu panorama...",
            77,
            82,
        ),
        (
            "fonts",
            "Importing Fonts",
            "Importing font assets...",
            82,
            87,
        ),
        (
            "languages",
            "Importing Languages",
            "Importing language assets...",
            87,
            92,
        ),
        (
            "sounds",
            "Importing Audio",
            "Importing sound assets from Minecraft...",
            92,
            99,
        ),
    ];

    for (cmd, phase, msg, base_pct, max_pct) in steps {
        report_prog(phase, base_pct, 100, msg);
        run_preparer_step_streaming(
            preparer_bin,
            preparer_root,
            java_home,
            &[cmd, "--minecraft-dir", &data_dir_str, "--root", &root_str],
            StepProgressScope {
                phase,
                base_percent: base_pct,
                max_percent: max_pct,
            },
            report_prog,
        )
        .await?;
    }

    Ok(())
}
