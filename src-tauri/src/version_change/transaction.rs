use super::journal::{
    detect_incomplete_transaction, read_journal_state, write_journal_typed, Journal, JournalState,
};
use super::models::{
    backup_dir, loader_label, now_secs, safe_file_name_typed, work_dir, ContentKind, ItemAction,
    PlanItem, Restored, Target, TransactionPhase, VersionChangeError, VersionPlan,
};
use super::staging::stage_typed;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

fn normalize_path(path: &Path) -> std::path::PathBuf {
    let mut normalized = std::path::PathBuf::new();
    for comp in path.components() {
        match comp {
            std::path::Component::ParentDir => {
                normalized.pop();
            }
            std::path::Component::CurDir => {}
            c => normalized.push(c),
        }
    }
    normalized
}

#[cfg(windows)]
fn components_match(p_comp: &std::path::Component, r_comp: &std::path::Component) -> bool {
    use std::path::{Component, Prefix};
    match (p_comp, r_comp) {
        (Component::Prefix(p1), Component::Prefix(p2)) => match (p1.kind(), p2.kind()) {
            (Prefix::VerbatimDisk(d1), Prefix::VerbatimDisk(d2))
            | (Prefix::Disk(d1), Prefix::Disk(d2))
            | (Prefix::VerbatimDisk(d1), Prefix::Disk(d2))
            | (Prefix::Disk(d1), Prefix::VerbatimDisk(d2)) => d1.eq_ignore_ascii_case(&d2),
            _ => p1
                .as_os_str()
                .to_string_lossy()
                .eq_ignore_ascii_case(&p2.as_os_str().to_string_lossy()),
        },
        (Component::RootDir, Component::RootDir) => true,
        (Component::Normal(s1), Component::Normal(s2)) => s1
            .to_string_lossy()
            .eq_ignore_ascii_case(&s2.to_string_lossy()),
        _ => false,
    }
}

pub(crate) fn strip_contained_root(path: &Path, root: &Path) -> Option<std::path::PathBuf> {
    let norm_path = normalize_path(path);
    let norm_root = normalize_path(root);

    #[cfg(windows)]
    {
        let mut path_comps = norm_path.components();
        let mut root_comps = norm_root.components();

        loop {
            match (path_comps.next(), root_comps.next()) {
                (Some(p_comp), Some(r_comp)) => {
                    if !components_match(&p_comp, &r_comp) {
                        return None;
                    }
                }
                (Some(p_comp), None) => {
                    let mut remainder = std::path::PathBuf::from(p_comp.as_os_str());
                    for comp in path_comps {
                        remainder.push(comp.as_os_str());
                    }
                    return Some(remainder);
                }
                (None, None) => {
                    return Some(std::path::PathBuf::new());
                }
                (None, Some(_)) => {
                    return None;
                }
            }
        }
    }
    #[cfg(not(windows))]
    {
        norm_path
            .strip_prefix(&norm_root)
            .ok()
            .map(|p| p.to_path_buf())
    }
}

pub(crate) fn is_contained(source_entry: &Path, link_target: &Path, root: &Path) -> bool {
    let resolved = if link_target.is_absolute() {
        link_target.to_path_buf()
    } else if let Some(parent) = source_entry.parent() {
        parent.join(link_target)
    } else {
        source_entry.join(link_target)
    };

    strip_contained_root(&resolved, root).is_some()
}

fn copy_symlink(from: &Path, to: &Path, src_root: &Path, dst_root: &Path) -> std::io::Result<()> {
    let link_target = fs::read_link(from)?;
    if !is_contained(from, &link_target, src_root) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!(
                "Directory contains uncontained link {:?} -> {:?} escaping root {:?}",
                from, link_target, src_root
            ),
        ));
    }

    let remapped_target = if link_target.is_absolute() {
        let rel = strip_contained_root(&link_target, src_root).ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!(
                    "Directory contains uncontained link {:?} -> {:?} escaping root {:?}",
                    from, link_target, src_root
                ),
            )
        })?;
        dst_root.join(rel)
    } else {
        link_target.clone()
    };

    let _ = remove_path(to);

    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(&remapped_target, to)?;
    }
    #[cfg(windows)]
    {
        let is_dir = if link_target.is_absolute() {
            link_target.is_dir()
        } else if let Some(parent) = from.parent() {
            parent.join(&link_target).is_dir()
        } else {
            false
        };
        if is_dir {
            std::os::windows::fs::symlink_dir(&remapped_target, to)?;
        } else {
            std::os::windows::fs::symlink_file(&remapped_target, to)?;
        }
    }
    #[cfg(not(any(unix, windows)))]
    {
        return Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "Symbolic links are not supported on this platform",
        ));
    }
    Ok(())
}

/// Copies a file or a folder, preserving contained symlinks and rejecting uncontained links.
/// Contained relative symlinks are preserved. Contained absolute symlinks are remapped between
/// `src_root` and `dst_root`.
pub fn copy_path(from: &Path, to: &Path) -> std::io::Result<()> {
    copy_path_contained(from, to, from, to)
}

pub fn copy_path_contained(
    from: &Path,
    to: &Path,
    src_root: &Path,
    dst_root: &Path,
) -> std::io::Result<()> {
    let meta = fs::symlink_metadata(from)?;
    let kind = meta.file_type();
    if kind.is_symlink() {
        if let Some(parent) = to.parent() {
            fs::create_dir_all(parent)?;
        }
        return copy_symlink(from, to, src_root, dst_root);
    }
    if meta.is_file() {
        if let Some(parent) = to.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(from, to)?;
        return Ok(());
    }
    fs::create_dir_all(to)?;
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let entry_kind = entry.file_type()?;
        let dest = to.join(entry.file_name());
        if entry_kind.is_symlink() {
            if let Some(parent) = dest.parent() {
                fs::create_dir_all(parent)?;
            }
            copy_symlink(&entry.path(), &dest, src_root, dst_root)?;
        } else if entry_kind.is_dir() {
            copy_path_contained(&entry.path(), &dest, src_root, dst_root)?;
        } else if entry_kind.is_file() {
            if let Some(parent) = dest.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(entry.path(), &dest)?;
        }
    }
    Ok(())
}

pub fn remove_path(path: &Path) -> std::io::Result<()> {
    match fs::symlink_metadata(path) {
        Ok(meta) if meta.is_dir() => fs::remove_dir_all(path),
        Ok(_) => fs::remove_file(path),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e),
    }
}

/// Moves a file or folder, creating the destination's parent
pub fn move_path(from: &Path, to: &Path) -> Result<(), VersionChangeError> {
    if let Some(parent) = to.parent() {
        fs::create_dir_all(parent).map_err(|e| {
            VersionChangeError::FileOp(format!("Failed to create {}: {e}", parent.display()))
        })?;
    }
    fs::rename(from, to)
        .map_err(|e| VersionChangeError::FileOp(format!("Failed to move {}: {e}", from.display())))
}

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
        let from = root.join(rel);
        if from.exists() {
            let final_dest = backup.join("configs").join(rel);
            if !final_dest.exists() {
                let staging_dest = backup.join(".staging_configs").join(format!(
                    "{}_{}",
                    rel.replace(['/', '\\'], "_"),
                    uuid::Uuid::new_v4()
                ));
                if let Err(e) = copy_path_contained(&from, &staging_dest, &from, &final_dest) {
                    let _ = remove_path(&staging_dest);
                    return Err(VersionChangeError::BackupError(format!(
                        "Failed to back up {rel}: {e}"
                    )));
                }
                if let Some(parent) = final_dest.parent() {
                    if let Err(e) = fs::create_dir_all(parent) {
                        let _ = remove_path(&staging_dest);
                        return Err(VersionChangeError::BackupError(format!(
                            "Failed to create directory for {rel}: {e}"
                        )));
                    }
                }
                if let Err(e) = move_path(&staging_dest, &final_dest) {
                    let _ = remove_path(&staging_dest);
                    return Err(e);
                }
            }
            if !journal.configs.contains(rel) {
                journal.configs.push(rel.clone());
                write_journal_typed(root, journal)?;
            }
        }
    }
    if backup_worlds {
        for (i, world) in target.worlds.iter().enumerate() {
            let from = root.join(world);
            if from.exists() {
                progress(format!(
                    "Backing up worlds ({} of {})...",
                    i + 1,
                    target.worlds.len()
                ));
                let final_dest = backup.join("worlds").join(world);
                if !final_dest.exists() {
                    let staging_dest = backup.join(".staging_worlds").join(format!(
                        "{}_{}",
                        world.replace(['/', '\\'], "_"),
                        uuid::Uuid::new_v4()
                    ));
                    if let Err(e) = copy_path_contained(&from, &staging_dest, &from, &final_dest) {
                        let _ = remove_path(&staging_dest);
                        return Err(VersionChangeError::BackupError(format!(
                            "Failed to back up the world {world}: {e}"
                        )));
                    }
                    if let Some(parent) = final_dest.parent() {
                        if let Err(e) = fs::create_dir_all(parent) {
                            let _ = remove_path(&staging_dest);
                            return Err(VersionChangeError::BackupError(format!(
                                "Failed to create directory for {world}: {e}"
                            )));
                        }
                    }
                    if let Err(e) = move_path(&staging_dest, &final_dest) {
                        let _ = remove_path(&staging_dest);
                        return Err(e);
                    }
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
        let src = root.join(&rel);
        if src.exists() {
            move_path(&src, &backup.join(&stored))?;
        }
        Ok(())
    };
    for rel in &target.core_files {
        if root.join(rel).exists() {
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
                let disabled = root.join(format!("{rel}.disabled"));
                if disabled.exists() {
                    move_to_backup(format!("{rel}.disabled"), journal)?;
                }
                if !journal.disabled.contains(&rel) {
                    journal.disabled.push(rel.clone());
                    write_journal_typed(root, journal)?;
                }
                let src = root.join(&rel);
                if src.exists() {
                    fs::rename(&src, &disabled).map_err(|e| {
                        VersionChangeError::FileOp(format!("Failed to turn off {name}: {e}"))
                    })?;
                }
            }
            ItemAction::Update | ItemAction::Add => {
                let file = item.target.as_ref().ok_or_else(|| {
                    VersionChangeError::PlanError(format!("{} has nothing to install", item.title))
                })?;
                if let Some(old) = &item.file_name {
                    let old_rel = format!("{folder}/{old}");
                    if root.join(&old_rel).exists() {
                        move_to_backup(old_rel, journal)?;
                    }
                }
                let rel = format!("{folder}/{}", file.file_name);
                if root.join(&rel).exists() {
                    // Already there (e.g. a dependency installed by hand); keep theirs aside
                    move_to_backup(rel.clone(), journal)?;
                }
                if !journal.added.contains(&rel) {
                    journal.added.push(rel.clone());
                    write_journal_typed(root, journal)?;
                }
                let staged_src = staging.join(&rel);
                if staged_src.exists() {
                    move_path(&staged_src, &root.join(&rel))?;
                }
            }
        }
    }
    Ok(())
}

/// Puts back everything the journal lists, newest change first.
/// Idempotent and non-destructive to backup sources so undo and recovery can be retried.
pub fn restore(root: &Path, backup: &Path, journal: &Journal) -> Result<(), VersionChangeError> {
    let mut errors = Vec::new();
    for rel in journal.added.iter().rev() {
        let target = root.join(rel);
        if target.exists() {
            if let Err(e) = remove_path(&target) {
                errors.push(format!("{rel}: {e}"));
            }
        }
    }
    for rel in journal.disabled.iter().rev() {
        let disabled = root.join(format!("{rel}.disabled"));
        let target = root.join(rel);
        if disabled.exists() {
            let _ = remove_path(&target);
            if let Err(e) = fs::rename(&disabled, &target) {
                errors.push(format!("{rel}: {e}"));
            }
        } else if !target.exists() {
            errors.push(format!(
                "Disabled file {rel}.disabled is missing and {rel} is absent from root"
            ));
        }
    }
    for (rel, stored) in journal.moved.iter().rev() {
        let src = backup.join(stored);
        let dest = root.join(rel);
        if src.exists() {
            let _ = remove_path(&dest);
            if let Err(e) = copy_path(&src, &dest) {
                errors.push(format!("{rel}: {e}"));
            }
        } else if !dest.exists() {
            errors.push(format!(
                "Backup source {stored} is missing and {rel} is absent from root"
            ));
        }
    }
    for (dir, list) in [("configs", &journal.configs), ("worlds", &journal.worlds)] {
        for rel in list {
            let src = backup.join(dir).join(rel);
            let dest = root.join(rel);
            if src.exists() {
                let _ = remove_path(&dest);
                if let Err(e) = copy_path(&src, &dest) {
                    errors.push(format!("{rel}: {e}"));
                }
            } else if !dest.exists() {
                errors.push(format!(
                    "Backup for {dir}/{rel} is missing and {rel} is absent from root"
                ));
            }
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(VersionChangeError::RestoreError(errors.join("; ")))
    }
}

/// Carries out the reviewed plan with an atomic metadata commit, all or nothing.
pub async fn apply_typed_with_commit<F, E>(
    client: &reqwest::Client,
    target: &Target,
    plan: &VersionPlan,
    backup_worlds: bool,
    progress: impl Fn(String) + Sync,
    commit_metadata: F,
) -> Result<(), VersionChangeError>
where
    F: FnOnce() -> Result<(), E>,
    E: std::fmt::Display,
{
    let root = &target.root;
    match detect_incomplete_transaction(root) {
        Ok(Some(incomplete)) => {
            return Err(VersionChangeError::Custom(format!(
                "Cannot start version change: a previous transaction is in state {:?} and requires recovery",
                incomplete.phase
            )));
        }
        Err(err) => {
            return Err(VersionChangeError::Custom(format!(
                "Cannot start version change: {err}"
            )));
        }
        Ok(None) => {}
    }

    let kinds: HashMap<&str, ContentKind> =
        target.folders.iter().map(|f| (f.dir, f.kind)).collect();
    for item in &plan.items {
        if kinds.get(item.folder.as_str()) != Some(&item.kind) {
            return Err(VersionChangeError::PlanError(format!(
                "{} is in an unexpected folder",
                item.title
            )));
        }
        if !item.actions.contains(&item.action) {
            return Err(VersionChangeError::PlanError(format!(
                "{} can't be set to {:?}",
                item.title, item.action
            )));
        }
        if let Some(name) = &item.file_name {
            safe_file_name_typed(name, item.kind)?;
        }
    }

    // 1. Download and verify first: a failure here changes nothing
    let staging = work_dir(root).join("staging");
    let to_download: Vec<&PlanItem> = plan
        .items
        .iter()
        .filter(|i| matches!(i.action, ItemAction::Update | ItemAction::Add))
        .collect();
    if !to_download.is_empty() {
        progress("Downloading...".into());
        if let Err(e) = stage_typed(client, &staging, &to_download, &progress).await {
            let _ = fs::remove_dir_all(&staging);
            return Err(e);
        }
    }

    // 2. Snapshot, then swap; any failure puts back what was done so far
    progress("Backing up...".into());
    let backup = backup_dir(root);
    let _ = fs::remove_dir_all(&backup);
    fs::create_dir_all(&backup).map_err(|e| {
        VersionChangeError::BackupError(format!("Failed to create the backup: {e}"))
    })?;
    let mut journal = Journal {
        created_at: now_secs(),
        from_game_version: plan.from_game_version.clone(),
        from_loader: plan.from_loader.clone(),
        from_loader_version: plan.from_loader_version.clone(),
        to_game_version: plan.to_game_version.clone(),
        to_loader: plan.to_loader.clone(),
        phase: TransactionPhase::Swapping,
        ..Default::default()
    };
    write_journal_typed(root, &journal)?;

    let swap_res = swap(
        target,
        &backup,
        &staging,
        plan,
        backup_worlds,
        &mut journal,
        &progress,
    );
    let _ = fs::remove_dir_all(&staging);

    if let Err(swap_err) = swap_res {
        let rollback_res = restore(root, &backup, &journal);
        match rollback_res {
            Ok(()) => {
                let _ = fs::remove_dir_all(&backup);
                return Err(VersionChangeError::RollbackSucceeded(swap_err.to_string()));
            }
            Err(restore_err) => {
                journal.phase = TransactionPhase::RollbackFailed;
                let _ = write_journal_typed(root, &journal);
                return Err(VersionChangeError::RollbackFailed {
                    primary: swap_err.to_string(),
                    restore: restore_err.to_string(),
                    backup_path: backup,
                });
            }
        }
    }

    // Save intermediate journal with recorded files
    write_journal_typed(root, &journal)?;

    // 3. Commit metadata: unify configuration commit with transaction journal state
    if let Err(meta_err) = commit_metadata() {
        let rollback_res = restore(root, &backup, &journal);
        match rollback_res {
            Ok(()) => {
                let _ = fs::remove_dir_all(&backup);
                return Err(VersionChangeError::RollbackSucceeded(format!(
                    "Failed to save configuration: {meta_err}"
                )));
            }
            Err(restore_err) => {
                journal.phase = TransactionPhase::RollbackFailed;
                let _ = write_journal_typed(root, &journal);
                return Err(VersionChangeError::RollbackFailed {
                    primary: format!("Failed to save configuration: {meta_err}"),
                    restore: restore_err.to_string(),
                    backup_path: backup,
                });
            }
        }
    }

    // 4. Finalize journal state
    journal.phase = TransactionPhase::Committed;
    write_journal_typed(root, &journal)
}

/// Carries out the reviewed plan, all or nothing. The caller then saves the new version.
pub async fn apply_typed(
    client: &reqwest::Client,
    target: &Target,
    plan: &VersionPlan,
    backup_worlds: bool,
    progress: impl Fn(String) + Sync,
) -> Result<(), VersionChangeError> {
    apply_typed_with_commit(client, target, plan, backup_worlds, progress, || {
        Ok::<(), VersionChangeError>(())
    })
    .await
}

pub async fn apply_with_commit<F, E>(
    client: &reqwest::Client,
    target: &Target,
    plan: &VersionPlan,
    backup_worlds: bool,
    progress: impl Fn(String) + Sync,
    commit_metadata: F,
) -> Result<(), String>
where
    F: FnOnce() -> Result<(), E>,
    E: std::fmt::Display,
{
    apply_typed_with_commit(
        client,
        target,
        plan,
        backup_worlds,
        progress,
        commit_metadata,
    )
    .await
    .map_err(|e| e.to_string())
}

pub async fn apply(
    client: &reqwest::Client,
    target: &Target,
    plan: &VersionPlan,
    backup_worlds: bool,
    progress: impl Fn(String) + Sync,
) -> Result<(), String> {
    apply_typed(client, target, plan, backup_worlds, progress)
        .await
        .map_err(|e| e.to_string())
}

/// Restores everything as it was before the last change, atomically updating metadata.
pub fn undo_typed_with_commit<F, E>(
    root: &Path,
    game_version: &str,
    loader: &str,
    commit_metadata: F,
) -> Result<Restored, VersionChangeError>
where
    F: FnOnce(&Restored) -> Result<(), E>,
    E: std::fmt::Display,
{
    let mut journal = match read_journal_state(root) {
        JournalState::Present(j) => *j,
        JournalState::Missing => {
            return Err(VersionChangeError::UndoError(
                "There's no version change to undo".into(),
            ));
        }
        JournalState::Corrupt(err) => {
            return Err(VersionChangeError::UndoError(format!(
                "Cannot undo: recovery journal is corrupt: {err}"
            )));
        }
    };
    if journal.phase == TransactionPhase::RollbackFailed {
        return Err(VersionChangeError::UndoError(
            "The previous transaction suffered a failed rollback and cannot be cleanly undone. Check the recovery backup.".into(),
        ));
    }
    if game_version != journal.to_game_version || loader != journal.to_loader {
        return Err(VersionChangeError::UndoError(format!(
            "It's on {} {game_version} now, not {} {}, so this backup no longer matches",
            loader_label(loader),
            loader_label(&journal.to_loader),
            journal.to_game_version
        )));
    }
    let backup = backup_dir(root);
    if let Err(e) = restore(root, &backup, &journal) {
        journal.phase = TransactionPhase::RollbackFailed;
        let _ = write_journal_typed(root, &journal);
        return Err(e);
    }
    let restored = Restored {
        game_version: journal.from_game_version,
        loader: journal.from_loader,
        loader_version: journal.from_loader_version,
    };
    if let Err(meta_err) = commit_metadata(&restored) {
        return Err(VersionChangeError::Custom(format!(
            "Restored files successfully, but failed to save configuration: {meta_err}. Backup retained."
        )));
    }
    let _ = fs::remove_dir_all(&backup);
    Ok(restored)
}

/// Restores everything as it was before the last change. `game_version` and `loader` are
/// the current ones: the backup only applies to the version it changed to.
pub fn undo_typed(
    root: &Path,
    game_version: &str,
    loader: &str,
) -> Result<Restored, VersionChangeError> {
    undo_typed_with_commit(root, game_version, loader, |_| {
        Ok::<(), VersionChangeError>(())
    })
}

pub fn undo_with_commit<F, E>(
    root: &Path,
    game_version: &str,
    loader: &str,
    commit_metadata: F,
) -> Result<Restored, String>
where
    F: FnOnce(&Restored) -> Result<(), E>,
    E: std::fmt::Display,
{
    undo_typed_with_commit(root, game_version, loader, commit_metadata).map_err(|e| e.to_string())
}

pub fn undo(root: &Path, game_version: &str, loader: &str) -> Result<Restored, String> {
    undo_typed(root, game_version, loader).map_err(|e| e.to_string())
}

/// Recovers an interrupted version change transaction left in Swapping or RollbackFailed state,
/// restoring files and updating configuration metadata.
pub fn recover_interrupted_transaction_with_commit<F, E>(
    root: &Path,
    commit_metadata: F,
) -> Result<Restored, VersionChangeError>
where
    F: FnOnce(&Restored) -> Result<(), E>,
    E: std::fmt::Display,
{
    let mut journal = match read_journal_state(root) {
        JournalState::Present(j) => *j,
        JournalState::Missing => {
            return Err(VersionChangeError::RestoreError(
                "No recovery journal found".into(),
            ));
        }
        JournalState::Corrupt(err) => {
            return Err(VersionChangeError::RestoreError(format!(
                "Cannot recover: recovery journal is corrupt: {err}"
            )));
        }
    };
    if journal.phase == TransactionPhase::Committed {
        return Err(VersionChangeError::RestoreError(
            "Transaction was already committed. Use undo instead.".into(),
        ));
    }
    let backup = backup_dir(root);
    if !backup.exists() {
        return Err(VersionChangeError::RestoreError(
            "No backup directory found for recovery".into(),
        ));
    }
    if let Err(restore_err) = restore(root, &backup, &journal) {
        journal.phase = TransactionPhase::RollbackFailed;
        let _ = write_journal_typed(root, &journal);
        return Err(VersionChangeError::RollbackFailed {
            primary: "Interrupted transaction recovery failed".into(),
            restore: restore_err.to_string(),
            backup_path: backup,
        });
    }

    let restored = Restored {
        game_version: journal.from_game_version,
        loader: journal.from_loader,
        loader_version: journal.from_loader_version,
    };
    if let Err(meta_err) = commit_metadata(&restored) {
        return Err(VersionChangeError::Custom(format!(
            "Restored files successfully, but failed to save configuration: {meta_err}. Backup retained."
        )));
    }
    let _ = fs::remove_dir_all(&backup);
    Ok(restored)
}

/// Recovers an interrupted version change transaction left in Swapping or RollbackFailed state.
pub fn recover_interrupted_transaction(root: &Path) -> Result<(), VersionChangeError> {
    recover_interrupted_transaction_with_commit(root, |_| Ok::<(), VersionChangeError>(()))
        .map(|_| ())
}
