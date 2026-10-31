use super::preparation::NativePreparation;
use crate::minecraft::horizon::{
    build_horizon_command, create_session_file, ensure_horizon_engine, fetch_horizon_releases,
    prepare_horizon_resources, HorizonChannel, HorizonError, HorizonRelease, HorizonSessionGuard,
};
use crate::minecraft::instance::InstanceConfig;
use crate::minecraft::launcher::models::LaunchError;
use crate::minecraft::launcher::prepare::PreparationEffects;
use crate::minecraft::launcher::progress::ProgressCallback;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Runtime};

pub(super) async fn prepare_horizon_game_command<R: Runtime>(
    app: &AppHandle<R>,
    client: &reqwest::Client,
    instance: &InstanceConfig,
    instance_dir: &Path,
    data_dir: &Path,
    report_prog: &ProgressCallback,
) -> Result<(tokio::process::Command, Option<HorizonSessionGuard>), LaunchError> {
    let cache_dir = data_dir.join("cache");
    let engines_dir = data_dir.join("engines");

    report_prog(
        "Resolving Horizon",
        1,
        10,
        "Fetching Horizon release metadata...",
    );

    let releases = fetch_horizon_releases(client, &cache_dir).await?;

    let selected_release = if let Some(ref ver) = instance.loader_version {
        releases
            .iter()
            .find(|r| r.tag == *ver || r.name == *ver)
            .cloned()
            .unwrap_or_else(|| HorizonRelease {
                tag: ver.clone(),
                name: ver.clone(),
                published_at: None,
                channel: HorizonChannel::Nightly,
                target_game_version: instance.game_version.clone(),
                asset_url: Some(format!(
                    "https://github.com/horizon-rust/horizon/releases/download/{}/{}",
                    ver,
                    crate::minecraft::horizon::manifest::target_platform_asset_name()
                )),
                asset_name: Some(
                    crate::minecraft::horizon::manifest::target_platform_asset_name().to_string(),
                ),
                asset_size: None,
            })
    } else {
        releases
            .first()
            .cloned()
            .ok_or_else(|| HorizonError::AssetNotFound("latest".to_string()))?
    };

    let prog = report_prog.clone();
    let executable: PathBuf = ensure_horizon_engine(
        client,
        &engines_dir,
        &selected_release,
        move |step, curr, total, msg| {
            prog(step, curr as u64, total as u64, msg);
        },
    )
    .await?;

    let engine_folder = executable
        .parent()
        .ok_or_else(|| HorizonError::ExecutableNotFound(executable.display().to_string()))?;

    let prep_prog = report_prog.clone();
    let resources_dir = prepare_horizon_resources(
        client,
        engine_folder,
        data_dir,
        move |step, curr, total, msg| {
            prep_prog(step, curr as u64, total as u64, msg);
        },
    )
    .await?;

    link_instance_resources(&resources_dir, instance_dir, &instance.game_version);

    let prep_effects = NativePreparation::new(app);
    let auth = prep_effects.resolve_auth().await?;

    let (session_guard, session_path, username) =
        if auth.user_type == "msa" && !auth.access_token.is_empty() && auth.access_token != "0" {
            let guard = create_session_file(
                instance_dir,
                &auth.player_name,
                &auth.uuid_str,
                &auth.access_token,
                auth.xuid.as_deref(),
            )?;
            let path = guard.path().to_path_buf();
            (Some(guard), Some(path), None)
        } else {
            (None, None, Some(auth.player_name.as_str()))
        };

    let fullscreen = instance.fullscreen.unwrap_or(false);
    let command = build_horizon_command(
        &executable,
        instance_dir,
        Some(&resources_dir),
        session_path.as_deref(),
        username,
        fullscreen,
    );

    Ok((command, session_guard))
}

fn link_instance_resources(prepared_dir: &Path, instance_dir: &Path, version: &str) {
    let target = instance_dir
        .join(".local")
        .join("horizon-client")
        .join(version);
    if target.exists() {
        return;
    }
    if let Some(parent) = target.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    #[cfg(windows)]
    {
        let _ = std::os::windows::fs::symlink_dir(prepared_dir, &target);
    }
    #[cfg(unix)]
    {
        let _ = std::os::unix::fs::symlink(prepared_dir, &target);
    }
}
