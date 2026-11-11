mod artifacts;
mod runner;

#[cfg(test)]
mod tests;

#[cfg(test)]
pub(crate) use artifacts::{collect_asset_tasks, collect_library_tasks};

use super::models::HorizonError;
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

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

pub fn is_horizon_complete(resources_dir: &Path) -> bool {
    resources_dir.join("horizon-resources.json").is_file()
        && resources_dir.join("asset-index.json").is_file()
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

    if is_horizon_complete(&resources_dir) {
        return Ok(resources_dir);
    }

    let pinned_manifest = artifacts::ensure_pinned_manifest(
        client,
        &preparer_root,
        &lock.minecraft.version,
        &lock.minecraft.manifest_sha1,
    )
    .await?;

    report_prog(
        "Verifying Files",
        15,
        100,
        "Checking Minecraft game files...",
    );
    artifacts::ensure_minecraft_files(
        client,
        data_dir,
        &lock.minecraft.version,
        &lock.minecraft.client_sha1,
        &pinned_manifest,
        report_prog.clone(),
    )
    .await?;

    let is_prepared = is_horizon_prepared(&preparer_root, &lock.minecraft.version);
    if !is_prepared {
        report_prog("Java Setup", 30, 100, "Checking Java 25 runtime...");
        let java_home =
            artifacts::resolve_java_home(client, data_dir, lock.minecraft.java_major).await?;

        report_prog(
            "Downloading Server",
            45,
            100,
            "Checking Minecraft server engine...",
        );
        let server_jar = artifacts::ensure_server_jar(
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

        runner::run_preparer_step_streaming(
            &preparer_bin,
            &preparer_root,
            Some(&java_home),
            &[
                "--prepare",
                "--server-jar",
                &server_jar_str,
                "--minecraft-dir",
                &data_dir_str,
                "--root",
                &root_str,
            ],
            runner::StepProgressScope {
                phase: "Generating Game Data",
                base_percent: 55,
                max_percent: 72,
            },
            report_prog.as_ref(),
        )
        .await?;
    }

    runner::import_horizon_assets(
        &preparer_bin,
        &preparer_root,
        data_dir,
        None,
        report_prog.as_ref(),
    )
    .await?;

    if !is_horizon_complete(&resources_dir) {
        return Err(HorizonError::Preparation(
            "horizon-prepare completed but resources are still incomplete".into(),
        ));
    }

    report_prog("Starting Game", 100, 100, "Horizon resources ready");

    Ok(resources_dir)
}
