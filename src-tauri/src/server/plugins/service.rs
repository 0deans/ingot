use super::metadata::{read_jar_metadata, safe_jar_name};
use super::models::{
    platform, InstallReport, InstalledPlugin, PluginError, PluginProject, PluginSearchResult,
    PluginSource, PluginUpdate, PluginVersion, TrackedPlugin,
};
use super::providers::{
    client, get_json, hangar_project, hangar_version, modrinth_version, n, pick_version,
    project_title, s, strings, HANGAR, MODRINTH,
};
use super::tracking::{read_tracking, write_tracking, write_tracking_typed};
use crate::server::config::ServerCoreType;
use serde_json::Value;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

pub fn folder(server_dir: &Path, core: &ServerCoreType) -> Result<PathBuf, String> {
    let platform = platform(core).ok_or("This server type doesn't support plugins or mods")?;
    Ok(server_dir.join(platform.folder))
}

pub fn list_installed(
    server_dir: &Path,
    core: &ServerCoreType,
) -> Result<Vec<InstalledPlugin>, String> {
    let dir = folder(server_dir, core)?;
    let tracked = read_tracking(server_dir);
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return Ok(Vec::new());
    };
    let mut plugins: Vec<InstalledPlugin> = entries
        .flatten()
        .filter_map(|entry| {
            let file_name = entry.file_name().to_string_lossy().into_owned();
            safe_jar_name(&file_name).ok()?;
            let meta = entry.metadata().ok()?;
            if !meta.is_file() {
                return None;
            }
            let enabled = !file_name.ends_with(".disabled");
            let jar_name = file_name.trim_end_matches(".disabled");
            let track = tracked.iter().find(|t| t.file_name == jar_name);
            let (name, version, description, authors) = read_jar_metadata(&entry.path())
                .unwrap_or_else(|| {
                    (
                        jar_name.trim_end_matches(".jar").to_string(),
                        None,
                        None,
                        Vec::new(),
                    )
                });
            Some(InstalledPlugin {
                file_name: file_name.clone(),
                enabled,
                size: meta.len().min(u32::MAX as u64) as u32,
                version: version.or_else(|| track.map(|t| t.version_number.clone())),
                description,
                authors,
                source: track.map(|t| t.source),
                project_id: track.map(|t| t.project_id.clone()),
                icon_url: track.and_then(|t| t.icon_url.clone()),
                system: name == crate::server::companion::PLUGIN_NAME,
                name,
            })
        })
        .collect();
    plugins.sort_by_key(|p| p.name.to_lowercase());
    Ok(plugins)
}

pub fn set_enabled(
    server_dir: &Path,
    core: &ServerCoreType,
    file_name: &str,
    enabled: bool,
) -> Result<(), String> {
    let dir = folder(server_dir, core)?;
    let file_name = safe_jar_name(file_name)?;
    let jar = file_name.trim_end_matches(".disabled");
    let (from, to) = if enabled {
        (format!("{jar}.disabled"), jar.to_string())
    } else {
        (jar.to_string(), format!("{jar}.disabled"))
    };
    if dir.join(&to).exists() {
        return Ok(());
    }
    std::fs::rename(dir.join(&from), dir.join(&to))
        .map_err(|e| format!("Failed to rename {from}: {e}"))
}

pub fn remove(server_dir: &Path, core: &ServerCoreType, file_name: &str) -> Result<(), String> {
    let dir = folder(server_dir, core)?;
    let file_name = safe_jar_name(file_name)?;
    std::fs::remove_file(dir.join(file_name))
        .map_err(|e| format!("Failed to delete {file_name}: {e}"))?;
    let jar = file_name.trim_end_matches(".disabled");
    let mut tracked = read_tracking(server_dir);
    tracked.retain(|t| t.file_name != jar);
    write_tracking(server_dir, &tracked);
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub async fn search(
    core: &ServerCoreType,
    game_version: &str,
    source: PluginSource,
    query: &str,
    sort: &str,
    compatible_only: bool,
    page: u32,
    page_size: u32,
) -> Result<PluginSearchResult, String> {
    let platform = platform(core).ok_or("This server type doesn't support plugins or mods")?;
    let client = client()?;
    let offset = (page * page_size).to_string();
    let limit = page_size.to_string();

    match source {
        PluginSource::Modrinth => {
            let mut facets = vec![
                vec![format!("project_type:{}", platform.modrinth_type)],
                platform
                    .modrinth_loaders
                    .iter()
                    .map(|l| format!("categories:{l}"))
                    .collect(),
            ];
            if platform.modrinth_type == "mod" {
                facets.push(vec![
                    "server_side:required".into(),
                    "server_side:optional".into(),
                ]);
            }
            if compatible_only {
                facets.push(vec![format!("versions:{game_version}")]);
            }
            let index = match sort {
                "relevance" => "relevance",
                "updated" => "updated",
                "newest" => "newest",
                _ if !query.trim().is_empty() => "relevance",
                _ => "downloads",
            };
            let mut params = vec![
                ("facets", serde_json::to_string(&facets).unwrap_or_default()),
                ("index", index.to_string()),
                ("offset", offset),
                ("limit", limit),
            ];
            if !query.trim().is_empty() {
                params.push(("query", query.trim().to_string()));
            }
            let res = get_json(&client, &format!("{MODRINTH}/search"), &params).await?;
            let items = res
                .get("hits")
                .and_then(Value::as_array)
                .map(|hits| {
                    hits.iter()
                        .map(|h| PluginProject {
                            id: s(h, "project_id"),
                            source: PluginSource::Modrinth,
                            slug: s(h, "slug"),
                            title: s(h, "title"),
                            description: s(h, "description"),
                            author: s(h, "author"),
                            icon_url: h
                                .get("icon_url")
                                .and_then(Value::as_str)
                                .filter(|u| !u.is_empty())
                                .map(str::to_string),
                            downloads: n(h, "downloads"),
                            categories: strings(h, "display_categories"),
                            page_url: format!(
                                "https://modrinth.com/{}/{}",
                                platform.modrinth_type,
                                s(h, "slug")
                            ),
                            players_need_it: platform.modrinth_type == "mod"
                                && s(h, "client_side") == "required",
                        })
                        .collect()
                })
                .unwrap_or_default();
            Ok(PluginSearchResult {
                items,
                total: n(&res, "total_hits"),
            })
        }
        PluginSource::Hangar => {
            if !platform.hangar {
                return Ok(PluginSearchResult {
                    items: Vec::new(),
                    total: 0,
                });
            }
            let sort = match sort {
                "updated" => "-updated",
                "newest" => "-newest",
                "stars" => "-stars",
                _ => "-downloads",
            };
            let mut params = vec![
                ("platform", "PAPER".to_string()),
                ("sort", sort.to_string()),
                ("offset", offset),
                ("limit", limit),
            ];
            if !query.trim().is_empty() {
                params.push(("q", query.trim().to_string()));
            }
            if compatible_only {
                params.push(("version", game_version.to_string()));
            }
            let res = get_json(&client, &format!("{HANGAR}/projects"), &params).await?;
            let items = res
                .get("result")
                .and_then(Value::as_array)
                .map(|projects| projects.iter().map(hangar_project).collect())
                .unwrap_or_default();
            let total = res.get("pagination").map(|p| n(p, "count")).unwrap_or(0);
            Ok(PluginSearchResult { items, total })
        }
    }
}

pub async fn versions(
    core: &ServerCoreType,
    game_version: &str,
    source: PluginSource,
    project_id: &str,
    compatible_only: bool,
) -> Result<Vec<PluginVersion>, String> {
    let platform = platform(core).ok_or("This server type doesn't support plugins or mods")?;
    let client = client()?;
    match source {
        PluginSource::Modrinth => {
            let mut params = vec![(
                "loaders",
                serde_json::to_string(platform.modrinth_loaders).unwrap_or_default(),
            )];
            if compatible_only {
                params.push((
                    "game_versions",
                    serde_json::to_string(&[game_version]).unwrap_or_default(),
                ));
            }
            let res = get_json(
                &client,
                &format!("{MODRINTH}/project/{project_id}/version"),
                &params,
            )
            .await?;
            Ok(res
                .as_array()
                .map(|vs| vs.iter().filter_map(modrinth_version).collect())
                .unwrap_or_default())
        }
        PluginSource::Hangar => {
            let mut params = vec![
                ("platform", "PAPER".to_string()),
                ("limit", "25".to_string()),
            ];
            if compatible_only {
                params.push(("platformVersion", game_version.to_string()));
            }
            let res = get_json(
                &client,
                &format!("{HANGAR}/projects/{project_id}/versions"),
                &params,
            )
            .await?;
            Ok(res
                .get("result")
                .and_then(Value::as_array)
                .map(|vs| vs.iter().filter_map(hangar_version).collect())
                .unwrap_or_default())
        }
    }
}

pub async fn download(client: &reqwest::Client, url: &str, dest: &Path) -> Result<(), String> {
    let res = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("Download failed: {e}"))?;
    if !res.status().is_success() {
        return Err(format!("Download failed: HTTP {}", res.status()));
    }
    let bytes = res
        .bytes()
        .await
        .map_err(|e| format!("Download failed: {e}"))?;
    let tmp = dest.with_extension("jar.part");
    std::fs::write(&tmp, &bytes).map_err(|e| format!("Failed to save plugin: {e}"))?;
    std::fs::rename(&tmp, dest).map_err(|e| format!("Failed to save plugin: {e}"))
}

pub async fn install(
    server_dir: &Path,
    core: &ServerCoreType,
    game_version: &str,
    source: PluginSource,
    project_id: &str,
    version_id: Option<&str>,
) -> Result<InstallReport, String> {
    let dir = folder(server_dir, core)?;
    std::fs::create_dir_all(&dir)
        .map_err(|e| format!("Failed to create {}: {e}", dir.display()))?;
    let client = client()?;
    let mut report = InstallReport {
        installed: Vec::new(),
        warnings: Vec::new(),
    };
    let installed_names: HashSet<String> = list_installed(server_dir, core)?
        .iter()
        .map(|p| p.name.to_lowercase())
        .collect();

    let mut queue: Vec<(String, Option<String>, bool)> =
        vec![(project_id.to_string(), version_id.map(str::to_string), true)];
    let mut seen = HashSet::new();
    while let Some((id, wanted_version, is_root)) = queue.pop() {
        if !seen.insert(id.clone()) {
            continue;
        }
        let tracked = read_tracking(server_dir);
        if !is_root
            && (tracked.iter().any(|t| t.project_id == id)
                || installed_names.contains(&id.to_lowercase()))
        {
            continue;
        }

        let (title, icon_url) = project_title(&client, source, &id).await;
        if !is_root && installed_names.contains(&title.to_lowercase()) {
            continue;
        }
        let all = versions(core, game_version, source, &id, true)
            .await
            .unwrap_or_default();
        let version = match &wanted_version {
            Some(v) => {
                let any = versions(core, game_version, source, &id, false)
                    .await
                    .unwrap_or_default();
                any.into_iter().find(|x| &x.id == v)
            }
            None => pick_version(&all).cloned(),
        };
        let Some(version) = version else {
            let msg = format!("{title} has no downloadable version for Minecraft {game_version}");
            if is_root {
                return Err(msg);
            }
            report.warnings.push(msg);
            continue;
        };
        let Some(url) = version.download_url.clone() else {
            report
                .warnings
                .push(format!("{title} must be downloaded from its website"));
            continue;
        };
        let file_name = safe_jar_name(&version.file_name)?.to_string();
        let target_dest = dir.join(&file_name);
        let staging_file = dir.join(format!(".staging-{}-{}.jar", uuid::Uuid::new_v4(), file_name));

        // 1. Download to isolated staging file FIRST
        if let Err(e) = download(&client, &url, &staging_file).await {
            let _ = std::fs::remove_file(&staging_file);
            let msg = format!("Failed to download {title}: {e}");
            if is_root {
                return Err(msg);
            }
            report.warnings.push(msg);
            continue;
        }

        // 2. Validate staged download
        let meta = match std::fs::metadata(&staging_file) {
            Ok(m) => m,
            Err(e) => {
                let _ = std::fs::remove_file(&staging_file);
                return Err(format!("Failed to verify downloaded {title}: {e}"));
            }
        };
        if meta.len() == 0 {
            let _ = std::fs::remove_file(&staging_file);
            let msg = format!("Downloaded file for {title} is empty");
            if is_root {
                return Err(msg);
            }
            report.warnings.push(msg);
            continue;
        }

        // 3. Collect old files to replace
        let mut tracked = read_tracking(server_dir);
        let mut old_files = Vec::new();
        for old in tracked.iter().filter(|t| t.project_id == id && t.file_name != file_name) {
            let active = dir.join(&old.file_name);
            let disabled = dir.join(format!("{}.disabled", old.file_name));
            if active.exists() {
                old_files.push(active);
            }
            if disabled.exists() {
                old_files.push(disabled);
            }
        }

        // 4. Atomically publish replacement file and commit tracking
        tracked.retain(|t| t.project_id != id && t.file_name != file_name);
        tracked.push(TrackedPlugin {
            file_name,
            source,
            project_id: id.clone(),
            version_id: version.id.clone(),
            version_number: version.version_number.clone(),
            title: title.clone(),
            icon_url,
        });

        publish_installed_plugin(
            &staging_file,
            &target_dest,
            &title,
            || write_tracking_typed(server_dir, &tracked).map_err(|e| PluginError::Other(e.to_string())),
            &old_files,
        )?;

        report.installed.push(title);

        for dep in &version.dependencies {
            queue.push((dep.project_id.clone(), None, false));
        }
    }
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
    let missing: Vec<TrackedPlugin> = read_tracking(server_dir)
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
) -> Vec<PluginUpdate> {
    let mut updates = Vec::new();
    for t in read_tracking(server_dir) {
        let Ok(all) = versions(core, game_version, t.source, &t.project_id, true).await else {
            continue;
        };
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
    updates
}

/// Atomically publishes a staged plugin file to `target_dest`, preserving any existing file
/// under a backup path during tracking update, and rolling back cleanly if tracking fails.
/// Retains and reports both primary and rollback failures if rollback encounters errors.
pub fn publish_installed_plugin<F>(
    staging_file: &Path,
    target_dest: &Path,
    title: &str,
    commit_tracking: F,
    old_files: &[PathBuf],
) -> Result<(), PluginError>
where
    F: FnOnce() -> Result<(), PluginError>,
{
    let dir = target_dest.parent().unwrap_or_else(|| Path::new("."));
    let file_name = target_dest
        .file_name()
        .map(|f| f.to_string_lossy().to_string())
        .unwrap_or_else(|| "plugin.jar".to_string());

    // 1. If target_dest already exists, preserve it in a temporary backup
    let previous_backup = if target_dest.exists() {
        let backup_path = dir.join(format!(".{}.backup.{}", file_name, uuid::Uuid::new_v4()));
        if let Err(e) = std::fs::rename(target_dest, &backup_path) {
            let _ = std::fs::remove_file(staging_file);
            return Err(PluginError::Other(format!(
                "Failed to preserve previous version of {title}: {e}"
            )));
        }
        Some(backup_path)
    } else {
        None
    };

    // 2. Publish staging file to target_dest
    if let Err(e) = std::fs::rename(staging_file, target_dest) {
        let _ = std::fs::remove_file(staging_file);
        if let Some(ref backup_path) = previous_backup {
            if let Err(restore_err) = std::fs::rename(backup_path, target_dest) {
                return Err(PluginError::PublicationRollbackFailed {
                    primary: format!("Failed to move replacement into place for {title}: {e}"),
                    rollback: format!("Failed to restore previous version: {restore_err}"),
                    backup: backup_path.display().to_string(),
                });
            }
        }
        return Err(PluginError::PublicationFailed {
            primary: format!("Failed to move replacement into place for {title}: {e}"),
        });
    }

    // 3. Commit tracking
    if let Err(tracking_err) = commit_tracking() {
        let primary = format!("Failed to record plugin tracking for {title}: {tracking_err}");
        let remove_replacement_res = std::fs::remove_file(target_dest);
        let restore_backup_res = previous_backup.as_ref().map(|bp| std::fs::rename(bp, target_dest));

        let mut rollback_errors = Vec::new();
        if let Err(rem_err) = remove_replacement_res {
            rollback_errors.push(format!("failed to remove replacement file: {rem_err}"));
        }
        if let Some(Err(rest_err)) = restore_backup_res {
            rollback_errors.push(format!("failed to restore backup: {rest_err}"));
        }

        if !rollback_errors.is_empty() {
            return Err(PluginError::PublicationRollbackFailed {
                primary,
                rollback: rollback_errors.join("; "),
                backup: previous_backup
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|| "none".to_string()),
            });
        }

        return Err(PluginError::PublicationFailed { primary });
    }

    // 4. Success: clean up temporary backup
    if let Some(ref backup_path) = previous_backup {
        let _ = std::fs::remove_file(backup_path);
    }

    // 5. Success: remove older versions with different filenames
    for old_file in old_files {
        let _ = std::fs::remove_file(old_file);
    }

    Ok(())
}
