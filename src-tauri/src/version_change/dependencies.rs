use super::models::{mention, ContentKind, Folder, ItemAction, ItemStatus, PlanItem};
use super::sources::{self, Release, Scope, Source, SourceClient, SourceError};
use std::collections::{HashMap, HashSet};

pub fn is_active(action: ItemAction) -> bool {
    matches!(
        action,
        ItemAction::Keep | ItemAction::Update | ItemAction::Add
    )
}

/// The version of a project for the target on its source
pub async fn find_target(
    sources: &SourceClient<'_>,
    source: Source,
    project: &str,
    scope: &Scope,
    game_version: &str,
) -> Result<Option<Release>, SourceError> {
    match source {
        Source::Modrinth => {
            sources
                .modrinth_target(project, game_version, &scope.modrinth)
                .await
        }
        Source::Curseforge => sources.cf_target(project, game_version, scope).await,
        Source::Hangar => {
            let versions = sources.hangar_versions(project, game_version).await?;
            let pick = versions
                .iter()
                .find(|v| v.channel == sources::Channel::Release)
                .or_else(|| versions.first());
            Ok(pick.cloned())
        }
    }
}

/// Adds missing required dependencies and flags incompatible mods, until nothing changes.
/// Returns the projects mentioned in notes that aren't items.
pub async fn resolve_dependencies(
    sources: &SourceClient<'_>,
    game: &str,
    folders: &HashMap<String, &Folder>,
    items: &mut Vec<PlanItem>,
    resulting: &mut Vec<Option<Release>>,
) -> Result<Vec<(Source, String)>, SourceError> {
    let mut unavailable: HashSet<(Source, String)> = HashSet::new();
    let mut project_of_version: HashMap<String, Option<String>> = HashMap::new();
    for _ in 0..10 {
        let mut changed = false;
        for index in 0..items.len() {
            if !matches!(items[index].kind, ContentKind::Mod | ContentKind::Plugin)
                || !is_active(items[index].action)
            {
                continue;
            }
            let Some(release) = resulting[index].clone() else {
                continue;
            };
            for dep in &release.deps {
                let project = match (&dep.project, &dep.version_id) {
                    (Some(p), _) => Some(p.clone()),
                    (None, Some(version)) if release.source == Source::Modrinth => {
                        if !project_of_version.contains_key(version) {
                            let p = sources.modrinth_project_of(version).await?;
                            project_of_version.insert(version.clone(), p);
                        }
                        project_of_version[version].clone()
                    }
                    _ => None,
                };
                let Some(project) = project.filter(|p| !p.is_empty()) else {
                    continue;
                };
                let key = (release.source, project.clone());
                let is_project = |i: &PlanItem| {
                    i.source == Some(release.source)
                        && i.project_id.as_deref() == Some(project.as_str())
                };
                let active = items.iter().any(|i| is_active(i.action) && is_project(i));

                if !dep.required {
                    if active && items[index].status != ItemStatus::Conflict {
                        let item = &mut items[index];
                        item.status = ItemStatus::Conflict;
                        item.action = ItemAction::Disable;
                        item.actions = vec![ItemAction::Disable, ItemAction::Keep];
                        item.note =
                            Some(format!("Doesn't work together with {}", mention(&project)));
                        changed = true;
                    }
                    continue;
                }
                if active {
                    continue;
                }
                let turned_off = items.iter().any(is_project);
                let Some(folder) = folders.get(&items[index].folder) else {
                    continue;
                };
                let found = if turned_off || unavailable.contains(&key) {
                    None
                } else {
                    find_target(sources, release.source, &project, &folder.scope, game).await?
                };
                match found.filter(|r| r.file.is_some()) {
                    Some(dep_release) => {
                        let needed_by = items[index]
                            .project_id
                            .as_deref()
                            .map(mention)
                            .unwrap_or_else(|| items[index].title.clone());
                        let note = match dep_release.unstable_note() {
                            Some(channel) => format!("Needed by {needed_by} · {channel}"),
                            None => format!("Needed by {needed_by}"),
                        };
                        items.push(PlanItem {
                            kind: items[index].kind,
                            folder: items[index].folder.clone(),
                            file_name: None,
                            title: project.clone(),
                            icon_url: None,
                            page_url: None,
                            source: Some(release.source),
                            project_id: Some(project.clone()),
                            current_version: None,
                            target: dep_release.file.clone(),
                            status: ItemStatus::NewDependency,
                            action: ItemAction::Add,
                            actions: vec![ItemAction::Add, ItemAction::Skip],
                            note: Some(note),
                        });
                        resulting.push(Some(dep_release));
                    }
                    None => {
                        unavailable.insert(key.clone());
                        let item = &mut items[index];
                        item.status = ItemStatus::Conflict;
                        item.action = ItemAction::Disable;
                        item.actions = vec![ItemAction::Disable, ItemAction::Keep];
                        item.note = Some(if turned_off {
                            format!("Needs {}, which is being turned off", mention(&project))
                        } else {
                            format!(
                                "Needs {}, which isn't available for {game}",
                                mention(&project)
                            )
                        });
                    }
                }
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    Ok(unavailable.into_iter().collect())
}
