use super::dependencies::{find_target, resolve_dependencies};
use super::models::{
    loader_label, ContentKind, Folder, ItemAction, ItemStatus, PlanItem, PlanRequest, Target,
    VersionChangeError, VersionPlan, FABRIC_ONLY,
};
use super::scan::{list_worlds, scan};
use super::sources::{self, ProjectInfo, Release, Source, SourceClient};
use std::collections::{HashMap, HashSet};

pub fn site(source: Source) -> &'static str {
    match source {
        Source::Modrinth => "Modrinth",
        Source::Curseforge => "CurseForge",
        Source::Hangar => "Hangar",
    }
}

/// Works out what moving to the target means for every file. Changes nothing.
pub async fn check_typed(
    client: &reqwest::Client,
    target: &Target,
    request: PlanRequest,
) -> Result<VersionPlan, VersionChangeError> {
    check_with_sources(&SourceClient::new(client), target, request).await
}

pub(super) async fn check_with_sources(
    sources: &SourceClient<'_>,
    target: &Target,
    request: PlanRequest,
) -> Result<VersionPlan, VersionChangeError> {
    let game = request.to_game_version.as_str();
    let loader_changes = request.from_loader != request.to_loader;
    let target_clone = target.clone();
    let local = tokio::task::spawn_blocking(move || scan(&target_clone))
        .await
        .map_err(VersionChangeError::Task)??;

    // Identify: Modrinth by hash (per folder: the loaders differ), then CurseForge
    let mut on_modrinth: HashMap<String, (Release, bool)> = HashMap::new();
    for (index, folder) in target.folders.iter().enumerate() {
        let hashes: Vec<String> = local
            .iter()
            .filter(|f| f.folder == index)
            .map(|f| f.sha1.clone())
            .collect();
        on_modrinth.extend(
            sources
                .modrinth_identify(&hashes, game, &folder.scope.modrinth)
                .await?,
        );
    }
    let mut on_curseforge: HashMap<u32, (Release, bool)> = HashMap::new();
    for (index, folder) in target.folders.iter().enumerate() {
        let prints: Vec<u32> = local
            .iter()
            .filter(|f| f.folder == index && !target.hangar.contains_key(&f.file_name))
            .filter_map(|f| f.fingerprint)
            .collect();
        on_curseforge.extend(sources.cf_identify(&prints, game, &folder.scope).await?);
    }

    // One entry per file, plus the release it would end up on
    let mut items: Vec<PlanItem> = Vec::new();
    let mut resulting: Vec<Option<Release>> = Vec::new();
    let mut hangar_info: HashMap<String, ProjectInfo> = HashMap::new();
    for file in &local {
        let folder = &target.folders[file.folder];
        let kind = folder.kind;
        let mut item = PlanItem {
            kind,
            folder: folder.dir.to_string(),
            file_name: Some(file.file_name.clone()),
            title: file.file_name.clone(),
            icon_url: None,
            page_url: None,
            source: None,
            project_id: None,
            current_version: None,
            target: None,
            status: ItemStatus::Works,
            action: ItemAction::Keep,
            actions: vec![ItemAction::Keep, ItemAction::Disable],
            note: None,
        };

        // What the file is now, and whether it already fits
        let identified: Option<(Release, bool)> = if let Some(found) = on_modrinth.get(&file.sha1) {
            Some(found.clone())
        } else if let Some(found) = file.fingerprint.and_then(|p| on_curseforge.get(&p)) {
            Some(found.clone())
        } else if let Some(tracked) = target.hangar.get(&file.file_name) {
            hangar_info.insert(
                tracked.project_id.clone(),
                ProjectInfo {
                    title: tracked.title.clone(),
                    icon_url: tracked.icon_url.clone(),
                    page_url: Some(format!(
                        "https://hangar.papermc.io/search?query={}",
                        tracked.project_id
                    )),
                },
            );
            let versions = sources.hangar_versions(&tracked.project_id, game).await?;
            let now = Release {
                source: Source::Hangar,
                project: tracked.project_id.clone(),
                version_id: tracked.version_id.clone(),
                version_number: tracked.version_id.clone(),
                channel: sources::Channel::Release,
                file: None,
                deps: Vec::new(),
            };
            Some(
                match versions.iter().find(|v| v.version_id == tracked.version_id) {
                    Some(listed) => (listed.clone(), true),
                    None => (now, false),
                },
            )
        } else {
            None
        };

        let result = match identified {
            None => {
                if kind == ContentKind::Mod || kind == ContentKind::Plugin {
                    let path = target.root.join(folder.dir).join(&file.file_name);
                    if let Some((name, version, _, _)) =
                        crate::server::plugins::read_jar_metadata(&path)
                    {
                        item.title = name;
                        item.current_version = version;
                    }
                }
                item.status = ItemStatus::Unknown;
                item.note =
                    Some("Not on Modrinth, CurseForge or Hangar, so Ingot can't check it".into());
                // A mod that doesn't match usually stops the game from starting; plugins
                // and packs mostly keep working
                if kind == ContentKind::Mod {
                    item.action = ItemAction::Disable;
                }
                None
            }
            Some((now, fits)) => {
                item.source = Some(now.source);
                item.project_id = Some(now.project.clone());
                item.current_version = Some(now.version_number.clone());
                if fits {
                    Some(now)
                } else {
                    let mut found =
                        find_target(sources, now.source, &now.project, &folder.scope, game).await?;
                    // A newer file of the same version already installed
                    if found
                        .as_ref()
                        .is_some_and(|f| f.version_id == now.version_id)
                    {
                        found = None;
                    }
                    // Not on Modrinth for the target: CurseForge may have it (same file there)
                    if found.as_ref().is_none_or(|f| f.file.is_none())
                        && now.source == Source::Modrinth
                    {
                        if let Some((cf_now, _)) =
                            file.fingerprint.and_then(|p| on_curseforge.get(&p))
                        {
                            let cf_found = find_target(
                                sources,
                                Source::Curseforge,
                                &cf_now.project,
                                &folder.scope,
                                game,
                            )
                            .await?;
                            if cf_found.as_ref().is_some_and(|f| {
                                f.file.is_some() && f.version_id != cf_now.version_id
                            }) {
                                found = cf_found;
                            }
                        }
                    }
                    match found {
                        Some(release) if release.file.is_some() => {
                            item.source = Some(release.source);
                            item.project_id = Some(release.project.clone());
                            item.status = ItemStatus::Update;
                            item.target = release.file.clone();
                            item.note = release.unstable_note();
                            item.action = ItemAction::Update;
                            item.actions =
                                vec![ItemAction::Update, ItemAction::Disable, ItemAction::Keep];
                            Some(release)
                        }
                        found => {
                            item.status = ItemStatus::Missing;
                            let target_name = if loader_changes {
                                format!("{} {game}", loader_label(&request.to_loader))
                            } else {
                                format!("Minecraft {game}")
                            };
                            item.note = Some(if found.is_some() {
                                format!(
                                    "Its {target_name} version can only be downloaded from {}",
                                    site(now.source)
                                )
                            } else if loader_changes && FABRIC_ONLY.contains(&now.project.as_str())
                            {
                                "Only needed on Fabric and Quilt".into()
                            } else {
                                match kind {
                                    ContentKind::Mod => format!("No version for {target_name} yet"),
                                    ContentKind::Plugin => {
                                        format!(
                                            "Not marked for {game} yet; most plugins still work"
                                        )
                                    }
                                    _ => format!("Not updated for {game}; it usually still works"),
                                }
                            });
                            if kind == ContentKind::Mod {
                                item.action = ItemAction::Disable;
                                item.actions = vec![ItemAction::Disable, ItemAction::Keep];
                                None
                            } else {
                                Some(now)
                            }
                        }
                    }
                }
            }
        };
        items.push(item);
        resulting.push(result);
    }

    // The same project twice in a folder: keep the first, turn the others off
    let mut seen: HashMap<(String, Source, String), String> = HashMap::new();
    for item in &mut items {
        let (Some(source), Some(project)) = (item.source, item.project_id.clone()) else {
            continue;
        };
        match seen.get(&(item.folder.clone(), source, project.clone())) {
            Some(first) => {
                item.status = ItemStatus::Conflict;
                item.action = ItemAction::Disable;
                item.actions = vec![ItemAction::Disable, ItemAction::Keep];
                item.note = Some(format!("Same as {first}"));
            }
            None => {
                seen.insert(
                    (item.folder.clone(), source, project),
                    item.file_name.clone().unwrap_or_default(),
                );
            }
        }
    }

    let folders: HashMap<String, &Folder> = target
        .folders
        .iter()
        .map(|f| (f.dir.to_string(), f))
        .collect();
    let mentioned =
        resolve_dependencies(sources, game, &folders, &mut items, &mut resulting).await?;

    // Titles, icons and pages (notes mention projects until now)
    let mut ids: HashMap<Source, HashSet<String>> = HashMap::new();
    for item in &items {
        if let (Some(source), Some(project)) = (item.source, &item.project_id) {
            ids.entry(source).or_default().insert(project.clone());
        }
    }
    for (source, project) in mentioned {
        ids.entry(source).or_default().insert(project);
    }
    let list = |source: Source| {
        ids.get(&source)
            .map(|s| s.iter().cloned().collect::<Vec<_>>())
            .unwrap_or_default()
    };
    let mut info: HashMap<(Source, String), ProjectInfo> = HashMap::new();
    info.extend(
        sources
            .modrinth_projects(&list(Source::Modrinth))
            .await?
            .into_iter()
            .map(|(k, v)| ((Source::Modrinth, k), v)),
    );
    info.extend(
        sources
            .cf_projects(&list(Source::Curseforge))
            .await?
            .into_iter()
            .map(|(k, v)| ((Source::Curseforge, k), v)),
    );
    info.extend(
        hangar_info
            .into_iter()
            .map(|(k, v)| ((Source::Hangar, k), v)),
    );
    let titles: HashMap<String, String> = info
        .iter()
        .map(|((_, id), p)| (id.clone(), p.title.clone()))
        .collect();
    for item in &mut items {
        if let Some(p) = item
            .source
            .zip(item.project_id.clone())
            .and_then(|key| info.get(&key))
        {
            if !p.title.is_empty() {
                item.title = p.title.clone();
            }
            item.icon_url = p.icon_url.clone();
            item.page_url = p.page_url.clone();
        }
        if let Some(note) = &mut item.note {
            let parts: Vec<&str> = note.split('\u{1}').collect();
            *note = parts
                .iter()
                .enumerate()
                .map(|(i, part)| {
                    if i % 2 == 1 {
                        titles
                            .get(*part)
                            .cloned()
                            .unwrap_or_else(|| part.to_string())
                    } else {
                        part.to_string()
                    }
                })
                .collect();
        }
    }

    let target_clone = target.clone();
    let worlds = tokio::task::spawn_blocking(move || list_worlds(&target_clone))
        .await
        .map_err(VersionChangeError::Task)??;

    Ok(VersionPlan {
        target_kind: request.target_kind,
        target_id: request.target_id,
        from_game_version: request.from_game_version,
        from_loader: request.from_loader,
        from_loader_version: request.from_loader_version,
        to_game_version: request.to_game_version,
        to_loader: request.to_loader,
        to_loader_version: request.to_loader_version,
        downgrade: request.downgrade,
        items,
        worlds,
    })
}

pub async fn check(
    client: &reqwest::Client,
    target: &Target,
    request: PlanRequest,
) -> Result<VersionPlan, VersionChangeError> {
    check_typed(client, target, request).await
}
