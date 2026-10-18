use super::super::{
    metadata::safe_jar_name,
    models::{InstallReport, PluginError, PluginSource, PluginUpdate, TrackedPlugin},
    providers::{pick_version, project_title, PluginProvider},
    tracking::{read_tracking, read_tracking_raw},
};
use super::{
    discovery::{versions, versions_with_provider},
    download::download_with_stage,
    installed::{folder, list_installed},
    publication::{publish_plugin_transaction_locked, with_plugin_lock},
};
use crate::server::config::ServerCoreType;
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};
pub async fn install(
    server_dir: &Path,
    core: &ServerCoreType,
    game_version: &str,
    source: PluginSource,
    project_id: &str,
    version_id: Option<&str>,
) -> Result<InstallReport, PluginError> {
    let provider = PluginProvider::new()?;
    install_with_provider(
        server_dir,
        core,
        game_version,
        source,
        project_id,
        version_id,
        &provider,
    )
    .await
}

pub(crate) async fn install_with_provider(
    server_dir: &Path,
    core: &ServerCoreType,
    game_version: &str,
    source: PluginSource,
    project_id: &str,
    version_id: Option<&str>,
    provider: &PluginProvider,
) -> Result<InstallReport, PluginError> {
    let dir = folder(server_dir, core)?;
    std::fs::create_dir_all(&dir).map_err(PluginError::Io)?;
    let client = &provider.client;
    let mut report = InstallReport {
        installed: Vec::new(),
        warnings: Vec::new(),
    };
    let installed_names: HashSet<String> = list_installed(server_dir, core)?
        .iter()
        .map(|p| p.name.to_lowercase())
        .collect();

    let staging = std::sync::Arc::new(
        tempfile::Builder::new()
            .prefix(".install-")
            .tempdir_in(&dir)?,
    );
    let mut pending = Vec::new();
    let mut queue: Vec<(String, Option<String>, bool)> =
        vec![(project_id.to_string(), version_id.map(str::to_string), true)];
    let mut seen = HashSet::new();
    while let Some((id, wanted_version, is_root)) = queue.pop() {
        if !seen.insert(id.clone()) {
            continue;
        }
        let tracked = read_tracking(server_dir)?;
        if !is_root
            && (tracked.iter().any(|t| t.project_id == id)
                || installed_names.contains(&id.to_lowercase()))
        {
            continue;
        }

        let (title, icon_url) = project_title(provider, source, &id).await?;
        if !is_root && installed_names.contains(&title.to_lowercase()) {
            continue;
        }
        let all = versions_with_provider(core, game_version, source, &id, true, provider).await?;
        let version = match &wanted_version {
            Some(v) => {
                let any = versions_with_provider(core, game_version, source, &id, false, provider)
                    .await?;
                any.into_iter().find(|x| &x.id == v)
            }
            None => pick_version(&all).cloned(),
        };
        let Some(version) = version else {
            return Err(PluginError::NoCompatibleVersion {
                title,
                game_version: game_version.into(),
            });
        };
        let Some(url) = version.download_url.clone() else {
            return Err(PluginError::ManualDownloadRequired(title));
        };
        let file_name = safe_jar_name(&version.file_name)?.to_string();
        let target_dest = crate::app::safe_enclosed_filename(&dir, &file_name)
            .ok_or_else(|| PluginError::InvalidJarName(file_name.clone()))?;
        let staging_file = staging.path().join(uuid::Uuid::new_v4().to_string());
        download_with_stage(
            client,
            &url,
            &staging_file,
            Some(std::sync::Arc::clone(&staging)),
        )
        .await?;
        if version.size > 0 && std::fs::metadata(&staging_file)?.len() != u64::from(version.size) {
            return Err(PluginError::InvalidPayload("download size"));
        }
        if pending
            .iter()
            .any(|item: &PreparedPlugin| item.target == target_dest)
        {
            return Err(PluginError::InvalidPayload("duplicate plugin filename"));
        }
        pending.push(PreparedPlugin {
            stage: staging_file,
            target: target_dest,
            tracking: TrackedPlugin {
                file_name,
                source,
                project_id: id.clone(),
                version_id: version.id.clone(),
                version_number: version.version_number.clone(),
                title: title.clone(),
                icon_url,
            },
        });

        report.installed.push(title);

        for dep in &version.dependencies {
            queue.push((dep.project_id.clone(), None, false));
        }
    }
    with_plugin_lock(server_dir, || {
        let mut tracked = read_tracking_raw(server_dir)?;
        let mut old_files = Vec::new();
        for item in &pending {
            for old in tracked
                .iter()
                .filter(|old| old.project_id == item.tracking.project_id)
            {
                for filename in [&old.file_name, &format!("{}.disabled", old.file_name)] {
                    let path = crate::app::safe_enclosed_filename(&dir, filename)
                        .ok_or_else(|| PluginError::InvalidJarName(filename.clone()))?;
                    if path.exists() {
                        old_files.push(path);
                    }
                }
            }
            tracked.retain(|old| {
                old.project_id != item.tracking.project_id
                    && old.file_name != item.tracking.file_name
            });
            tracked.push(item.tracking.clone());
        }
        let replacements = pending
            .iter()
            .map(|item| (item.stage.clone(), item.target.clone()))
            .collect::<Vec<_>>();
        publish_plugin_transaction_locked(server_dir, core, &replacements, &old_files, &tracked)
    })?;
    Ok(report)
}

pub async fn restore_missing(
    server_dir: &Path,
    core: &ServerCoreType,
    game_version: &str,
) -> Vec<String> {
    let Ok(dir) = folder(server_dir, core) else {
        return Vec::new();
    };
    let tracked = match read_tracking(server_dir) {
        Ok(tracked) => tracked,
        Err(error) => return vec![error.to_string()],
    };
    let missing: Vec<TrackedPlugin> = tracked
        .into_iter()
        .filter(|t| {
            !dir.join(&t.file_name).exists()
                && !dir.join(format!("{}.disabled", t.file_name)).exists()
        })
        .collect();
    let mut warnings = Vec::new();
    for t in missing {
        match install(
            server_dir,
            core,
            game_version,
            t.source,
            &t.project_id,
            Some(&t.version_id),
        )
        .await
        {
            Ok(report) => warnings.extend(report.warnings),
            Err(e) => warnings.push(format!("Couldn't download {}: {e}", t.title)),
        }
    }
    warnings
}

pub async fn check_updates(
    server_dir: &Path,
    core: &ServerCoreType,
    game_version: &str,
) -> Result<Vec<PluginUpdate>, PluginError> {
    let mut updates = Vec::new();
    for t in read_tracking(server_dir)? {
        let all = versions(core, game_version, t.source, &t.project_id, true).await?;
        if let Some(latest) = pick_version(&all) {
            if latest.id != t.version_id {
                updates.push(PluginUpdate {
                    file_name: t.file_name.clone(),
                    current_version: Some(t.version_number.clone()),
                    latest: latest.clone(),
                });
            }
        }
    }
    Ok(updates)
}

struct PreparedPlugin {
    stage: PathBuf,
    target: PathBuf,
    tracking: TrackedPlugin,
}
