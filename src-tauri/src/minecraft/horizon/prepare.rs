use super::models::HorizonError;
use crate::minecraft::downloader::download_file_chunked;
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::AsyncBufReadExt;

#[derive(Debug, Deserialize)]
pub struct ResearchLock {
    pub minecraft: MinecraftLock,
}

#[derive(Debug, Deserialize)]
pub struct MinecraftLock {
    pub version: String,
    pub manifest_sha1: String,
    pub client_sha1: String,
    pub server_sha1: String,
    #[serde(default = "default_java_major")]
    pub java_major: u32,
}

fn default_java_major() -> u32 {
    25
}

pub fn is_horizon_prepared(engine_dir: &Path, game_version: &str) -> bool {
    let manifest = engine_dir
        .join(".local")
        .join("horizon-client")
        .join(game_version)
        .join("horizon-resources.json");
    manifest.is_file()
}

pub fn preparer_binary_name() -> &'static str {
    #[cfg(target_os = "windows")]
    {
        "horizon-prepare.exe"
    }
    #[cfg(not(target_os = "windows"))]
    {
        "horizon-prepare"
    }
}

pub fn find_preparer_binary(engine_dir: &Path) -> Result<(PathBuf, PathBuf), HorizonError> {
    let binary_name = preparer_binary_name();
    let direct = engine_dir.join(binary_name);
    if direct.is_file() {
        return Ok((direct, engine_dir.to_path_buf()));
    }

    if let Ok(entries) = fs::read_dir(engine_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let nested = path.join(binary_name);
                if nested.is_file() {
                    return Ok((nested, path));
                }
            }
        }
    }

    Err(HorizonError::ExecutableNotFound(format!(
        "Preparer {binary_name} not found in {}",
        engine_dir.display()
    )))
}

pub fn find_research_lock(root: &Path) -> Result<PathBuf, HorizonError> {
    let direct = root.join("research-lock.json");
    if direct.is_file() {
        return Ok(direct);
    }
    if let Some(parent) = root.parent() {
        let parent_lock = parent.join("research-lock.json");
        if parent_lock.is_file() {
            return Ok(parent_lock);
        }
    }
    Err(HorizonError::Preparation(
        "research-lock.json was not found in Horizon engine folder".into(),
    ))
}

async fn ensure_pinned_manifest(
    client: &reqwest::Client,
    preparer_root: &Path,
    version: &str,
    manifest_sha1: &str,
) -> Result<PathBuf, HorizonError> {
    let manifest_path = preparer_root
        .join(".local")
        .join("minecraft")
        .join(version)
        .join("manifest.json");
    if manifest_path.is_file() {
        return Ok(manifest_path);
    }

    if let Some(parent) = manifest_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let url = format!("https://piston-meta.mojang.com/v1/packages/{manifest_sha1}/{version}.json");
    let bytes = client
        .get(&url)
        .header("User-Agent", "Ingot-Launcher")
        .send()
        .await?
        .error_for_status()
        .map_err(HorizonError::Transport)?
        .bytes()
        .await?;

    fs::write(&manifest_path, &bytes)?;
    Ok(manifest_path)
}

async fn ensure_server_jar<FProg>(
    client: &reqwest::Client,
    data_dir: &Path,
    version: &str,
    server_sha1: &str,
    report_prog: Arc<FProg>,
) -> Result<PathBuf, HorizonError>
where
    FProg: Fn(&str, usize, usize, &str) + Send + Sync + 'static,
{
    let server_jars_dir = data_dir.join("cache").join("server-jars");
    let server_jar_path = server_jars_dir.join(format!("{version}.jar"));
    if server_jar_path.is_file() {
        return Ok(server_jar_path);
    }

    fs::create_dir_all(&server_jars_dir)?;

    let url = format!("https://piston-data.mojang.com/v1/objects/{server_sha1}/server.jar");

    let dl_cb = {
        let rp = report_prog.clone();
        Arc::new(move |cur: u64, total: u64| {
            let detail = format!(
                "server.jar ({:.1}MB / {:.1}MB)",
                cur as f64 / 1_048_576.0,
                total as f64 / 1_048_576.0
            );
            let percent = (cur * 100).checked_div(total).unwrap_or(0) as usize;
            rp("Downloading Server", percent, 100, &detail);
        })
    };

    download_file_chunked(
        client,
        &url,
        &server_jar_path,
        Some(server_sha1),
        Some(dl_cb),
    )
    .await
    .map_err(|e| HorizonError::Preparation(format!("Failed to download server jar: {e}")))?;

    Ok(server_jar_path)
}

async fn ensure_minecraft_files<FProg>(
    client: &reqwest::Client,
    data_dir: &Path,
    version: &str,
    client_sha1: &str,
    pinned_manifest_path: &Path,
    report_prog: Arc<FProg>,
) -> Result<(), HorizonError>
where
    FProg: Fn(&str, usize, usize, &str) + Send + Sync + 'static,
{
    let cache_version_dir = data_dir.join("cache").join("versions").join(version);
    let cached_jar = cache_version_dir.join(format!("{version}.jar"));

    if !cached_jar.is_file() {
        fs::create_dir_all(&cache_version_dir)?;
        let url = format!("https://piston-data.mojang.com/v1/objects/{client_sha1}/client.jar");

        let dl_cb = {
            let rp = report_prog.clone();
            Arc::new(move |cur: u64, total: u64| {
                let detail = format!(
                    "client.jar ({:.1}MB / {:.1}MB)",
                    cur as f64 / 1_048_576.0,
                    total as f64 / 1_048_576.0
                );
                let percent = (cur * 100).checked_div(total).unwrap_or(0) as usize;
                rp("Downloading Client", percent, 100, &detail);
            })
        };

        download_file_chunked(client, &url, &cached_jar, Some(client_sha1), Some(dl_cb))
            .await
            .map_err(|e| {
                HorizonError::Preparation(format!("Failed to download client jar: {e}"))
            })?;
    }

    let mc_version_dir = data_dir.join("versions").join(version);
    fs::create_dir_all(&mc_version_dir)?;

    let target_jar = mc_version_dir.join(format!("{version}.jar"));
    if !target_jar.is_file()
        && cached_jar.is_file()
        && fs::hard_link(&cached_jar, &target_jar).is_err()
    {
        let _ = fs::copy(&cached_jar, &target_jar);
    }

    let target_json = mc_version_dir.join(format!("{version}.json"));
    if !target_json.is_file() {
        let cached_json = cache_version_dir.join(format!("{version}.json"));
        if cached_json.is_file() {
            if fs::hard_link(&cached_json, &target_json).is_err() {
                let _ = fs::copy(&cached_json, &target_json);
            }
        } else if pinned_manifest_path.is_file()
            && fs::hard_link(pinned_manifest_path, &target_json).is_err()
        {
            let _ = fs::copy(pinned_manifest_path, &target_json);
        }
    }

    Ok(())
}

async fn resolve_java_home(
    client: &reqwest::Client,
    data_dir: &Path,
    major_version: u32,
) -> Result<PathBuf, HorizonError> {
    let java_bin = crate::app::java::ensure_java_runtime_in_dir(
        data_dir.to_path_buf(),
        client,
        major_version,
        None,
        None,
    )
    .await
    .map_err(|e| {
        HorizonError::Preparation(format!("Failed to ensure Java {major_version}: {e}"))
    })?;

    let bin_dir = java_bin.parent().ok_or_else(|| {
        HorizonError::Preparation("Java executable parent directory not found".into())
    })?;

    let java_home = bin_dir
        .parent()
        .ok_or_else(|| HorizonError::Preparation("Java home directory not found".into()))?;

    Ok(java_home.to_path_buf())
}

struct StepProgressScope {
    phase: &'static str,
    base_percent: usize,
    max_percent: usize,
}

async fn run_preparer_step_streaming<FProg>(
    preparer_bin: &Path,
    preparer_root: &Path,
    java_home: &Path,
    args: &[&str],
    scope: StepProgressScope,
    report_prog: &FProg,
) -> Result<(), HorizonError>
where
    FProg: Fn(&str, usize, usize, &str) + Send + Sync,
{
    let java_bin_dir = java_home.join("bin");
    let current_path = std::env::var_os("PATH").unwrap_or_default();
    let mut new_path = std::ffi::OsString::new();
    new_path.push(&java_bin_dir);
    #[cfg(target_os = "windows")]
    new_path.push(";");
    #[cfg(not(target_os = "windows"))]
    new_path.push(":");
    new_path.push(current_path);

    let mut child = tokio::process::Command::new(preparer_bin)
        .current_dir(preparer_root)
        .args(args)
        .env("JAVA_HOME", java_home)
        .env("PATH", new_path)
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
                match err_res {
                    Ok(Some(text)) => {
                        let trimmed = text.trim().to_string();
                        if !trimmed.is_empty() {
                            stderr_lines.push(trimmed);
                        }
                    }
                    Ok(None) => {},
                    Err(_) => {},
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

pub async fn prepare_horizon_resources<FProg>(
    client: &reqwest::Client,
    engine_dir: &Path,
    data_dir: &Path,
    report_prog: FProg,
) -> Result<PathBuf, HorizonError>
where
    FProg: Fn(&str, usize, usize, &str) + Send + Sync + 'static,
{
    let report_prog = Arc::new(report_prog);
    let (preparer_bin, preparer_root) = find_preparer_binary(engine_dir)?;
    let lock_path = find_research_lock(&preparer_root)?;
    let lock_content = fs::read_to_string(&lock_path)?;
    let lock: ResearchLock = serde_json::from_str(&lock_content)?;

    let resources_dir = preparer_root
        .join(".local")
        .join("horizon-client")
        .join(&lock.minecraft.version);

    if is_horizon_prepared(&preparer_root, &lock.minecraft.version) {
        return Ok(resources_dir);
    }

    report_prog("Java Setup", 10, 100, "Checking Java 25 runtime...");
    let java_home = resolve_java_home(client, data_dir, lock.minecraft.java_major).await?;

    report_prog(
        "Resolving Metadata",
        25,
        100,
        "Verifying 26.3 game manifest...",
    );
    let pinned_manifest = ensure_pinned_manifest(
        client,
        &preparer_root,
        &lock.minecraft.version,
        &lock.minecraft.manifest_sha1,
    )
    .await?;

    report_prog(
        "Verifying Files",
        40,
        100,
        "Checking Minecraft game files...",
    );
    ensure_minecraft_files(
        client,
        data_dir,
        &lock.minecraft.version,
        &lock.minecraft.client_sha1,
        &pinned_manifest,
        report_prog.clone(),
    )
    .await?;

    report_prog(
        "Downloading Server",
        50,
        100,
        "Checking Minecraft server engine...",
    );
    let server_jar = ensure_server_jar(
        client,
        data_dir,
        &lock.minecraft.version,
        &lock.minecraft.server_sha1,
        report_prog.clone(),
    )
    .await?;

    report_prog(
        "Generating Game Data",
        55,
        100,
        "Starting Horizon data tables generation...",
    );

    let server_jar_str = server_jar.to_string_lossy();
    let data_dir_str = data_dir.to_string_lossy();
    let root_str = preparer_root.to_string_lossy();

    run_preparer_step_streaming(
        &preparer_bin,
        &preparer_root,
        &java_home,
        &[
            "--prepare",
            "--server-jar",
            &server_jar_str,
            "--minecraft-dir",
            &data_dir_str,
            "--root",
            &root_str,
        ],
        StepProgressScope {
            phase: "Generating Game Data",
            base_percent: 55,
            max_percent: 85,
        },
        report_prog.as_ref(),
    )
    .await?;

    report_prog(
        "Importing Audio",
        85,
        100,
        "Importing sound assets from Minecraft...",
    );

    let _ = run_preparer_step_streaming(
        &preparer_bin,
        &preparer_root,
        &java_home,
        &[
            "sounds",
            "--minecraft-dir",
            &data_dir_str,
            "--root",
            &root_str,
        ],
        StepProgressScope {
            phase: "Importing Audio",
            base_percent: 85,
            max_percent: 98,
        },
        report_prog.as_ref(),
    )
    .await;

    if !is_horizon_prepared(&preparer_root, &lock.minecraft.version) {
        return Err(HorizonError::Preparation(
            "horizon-prepare completed but horizon-resources.json was not created".into(),
        ));
    }

    report_prog("Starting Game", 100, 100, "Horizon resources ready");

    Ok(resources_dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_preparer_binary_name() {
        let name = preparer_binary_name();
        #[cfg(target_os = "windows")]
        assert_eq!(name, "horizon-prepare.exe");
        #[cfg(not(target_os = "windows"))]
        assert_eq!(name, "horizon-prepare");
    }

    #[test]
    fn test_is_horizon_prepared_false_for_empty_dir() {
        let temp = tempfile::tempdir().unwrap();
        assert!(!is_horizon_prepared(temp.path(), "26.3"));
    }
}
