use super::super::{
    journal::{write_journal_typed, Journal},
    models::{ItemAction, Target, VersionChangeError, VersionPlan},
};
use super::paths::{checked_path, copy_path_contained, move_path, remove_path, rooted_path};
use std::{fs, path::Path};

pub fn swap(
    target: &Target,
    backup: &Path,
    staging: &Path,
    plan: &VersionPlan,
    backup_worlds: bool,
    journal: &mut Journal,
    progress: &impl Fn(String),
) -> Result<(), VersionChangeError> {
    let root = &target.root;
    // The game rewrites its options and mods/plugins migrate their configs on first start
    for rel in &target.configs {
        let from = rooted_path(root, rel)?;
        if from.exists() {
            let final_dest = checked_path(root, &backup.join("configs").join(rel))?;
            if !final_dest.exists() {
                let staging_dest = backup.join(".staging_configs").join(format!(
                    "{}_{}",
                    rel.replace(['/', '\\'], "_"),
                    uuid::Uuid::new_v4()
                ));
                let staging_dest = checked_path(root, &staging_dest)?;
                prepare_snapshot(&from, &final_dest, &staging_dest)?;
            }
            if !journal.configs.contains(rel) {
                journal.configs.push(rel.clone());
                write_journal_typed(root, journal)?;
            }
        } else if !journal.absent_configs.contains(rel) {
            journal.absent_configs.push(rel.clone());
            write_journal_typed(root, journal)?;
        }
    }
    if backup_worlds {
        for (i, world) in target.worlds.iter().enumerate() {
            let from = rooted_path(root, world)?;
            if from.exists() {
                progress(format!(
                    "Backing up worlds ({} of {})...",
                    i + 1,
                    target.worlds.len()
                ));
                let final_dest = checked_path(root, &backup.join("worlds").join(world))?;
                if !final_dest.exists() {
                    let staging_dest = backup.join(".staging_worlds").join(format!(
                        "{}_{}",
                        world.replace(['/', '\\'], "_"),
                        uuid::Uuid::new_v4()
                    ));
                    let staging_dest = checked_path(root, &staging_dest)?;
                    prepare_snapshot(&from, &final_dest, &staging_dest)?;
                }
                if !journal.worlds.contains(world) {
                    journal.worlds.push(world.clone());
                    write_journal_typed(root, journal)?;
                }
            }
        }
    }

    progress("Applying...".into());
    let move_to_backup = |rel: String, journal: &mut Journal| -> Result<(), VersionChangeError> {
        let stored = format!("files/{rel}");
        if !journal.moved.iter().any(|(r, _)| r == &rel) {
            journal.moved.push((rel.clone(), stored.clone()));
            write_journal_typed(root, journal)?;
        }
        let src = rooted_path(root, &rel)?;
        if src.exists() {
            let destination = checked_path(root, &backup.join(&stored))?;
            move_path(&src, &destination)?;
        }
        Ok(())
    };
    for rel in &target.core_files {
        if rooted_path(root, rel)?.exists() {
            move_to_backup(rel.clone(), journal)?;
        }
    }
    for item in &plan.items {
        let folder = &item.folder;
        match item.action {
            ItemAction::Keep | ItemAction::Skip => {}
            ItemAction::Disable => {
                let Some(name) = &item.file_name else {
                    continue;
                };
                let rel = format!("{folder}/{name}");
                let disabled = rooted_path(root, format!("{rel}.disabled"))?;
                if disabled.exists() {
                    move_to_backup(format!("{rel}.disabled"), journal)?;
                }
                if !journal.disabled.contains(&rel) {
                    journal.disabled.push(rel.clone());
                    write_journal_typed(root, journal)?;
                }
                let src = rooted_path(root, &rel)?;
                if src.exists() {
                    fs::rename(&src, &disabled)
                        .map_err(|e| super::super::error::file_io("disable file", &src, e))?;
                }
            }
            ItemAction::Update | ItemAction::Add => {
                let file = item.target.as_ref().ok_or_else(|| {
                    VersionChangeError::PlanError(format!("{} has nothing to install", item.title))
                })?;
                if let Some(old) = &item.file_name {
                    let old_rel = format!("{folder}/{old}");
                    if rooted_path(root, &old_rel)?.exists() {
                        move_to_backup(old_rel, journal)?;
                    }
                }
                let rel = format!("{folder}/{}", file.file_name);
                if rooted_path(root, &rel)?.exists() {
                    // Already there (e.g. a dependency installed by hand); keep theirs aside
                    move_to_backup(rel.clone(), journal)?;
                }
                if !journal.added.contains(&rel) {
                    journal.added.push(rel.clone());
                    write_journal_typed(root, journal)?;
                }
                let staged_src = checked_path(root, &staging.join(&rel))?;
                let destination = rooted_path(root, &rel)?;
                move_path(&staged_src, &destination)?;
            }
        }
    }
    Ok(())
}

fn prepare_snapshot(
    source: &Path,
    destination: &Path,
    staging: &Path,
) -> Result<(), VersionChangeError> {
    let result = (|| {
        copy_path_contained(source, staging, source, destination)
            .map_err(|error| super::super::error::file_io("copy backup", source, error))?;
        move_path(staging, destination)
    })();
    match result {
        Ok(()) => Ok(()),
        Err(primary) => match remove_path(staging) {
            Ok(()) => Err(primary),
            Err(source) => Err(VersionChangeError::AdditionalFailure {
                primary: Box::new(primary),
                secondary: Box::new(super::super::error::file_io(
                    "clean backup staging",
                    staging,
                    source,
                )),
            }),
        },
    }
}
