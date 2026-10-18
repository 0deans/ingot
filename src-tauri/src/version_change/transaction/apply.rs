use super::super::{
    journal::{detect_incomplete_transaction, write_journal_typed, Journal},
    models::{
        backup_dir, now_secs, safe_file_name_typed, work_dir, ContentKind, ItemAction, PlanItem,
        Target, TransactionPhase, VersionChangeError, VersionPlan,
    },
    staging::stage_typed,
};
use super::{
    paths::{checked_path, remove_path},
    restoration_files::restore,
    swap::swap,
};
use std::{collections::HashMap, fs};

/// Applies a reviewed plan, retaining recovery material when rollback or finalization fails.
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
    E: std::error::Error + Send + Sync + 'static,
{
    let root = &target.root;
    if let Some(incomplete) = detect_incomplete_transaction(root)? {
        return Err(VersionChangeError::IncompleteTransaction(incomplete.phase));
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
    let staging = checked_path(root, &work_dir(root).join("staging"))?;
    // Validate both before the first download or removal can follow a linked ancestor.
    let backup = checked_path(root, &backup_dir(root))?;
    checked_path(root, &backup.join(super::super::models::JOURNAL))?;
    checked_path(root, &staging.join(".ancestor-check"))?;
    let to_download: Vec<&PlanItem> = plan
        .items
        .iter()
        .filter(|i| matches!(i.action, ItemAction::Update | ItemAction::Add))
        .collect();
    if !to_download.is_empty() {
        progress("Downloading...".into());
        if let Err(e) = stage_typed(client, &staging, &to_download, &progress).await {
            return Err(with_staging_cleanup(e, &staging));
        }
    }

    // 2. Snapshot, then swap; any failure puts back what was done so far
    progress("Backing up...".into());
    remove_path(&backup)
        .map_err(|error| super::super::error::file_io("replace previous backup", &backup, error))?;
    fs::create_dir_all(&backup)
        .map_err(|e| super::super::error::file_io("create backup", &backup, e))?;
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
    let swap_res = match (swap_res, remove_path(&staging)) {
        (result, Ok(())) => result,
        (Ok(()), Err(source)) => Err(super::super::error::file_io(
            "clean staging",
            &staging,
            source,
        )),
        (Err(primary), Err(source)) => Err(VersionChangeError::AdditionalFailure {
            primary: Box::new(primary),
            secondary: Box::new(super::super::error::file_io(
                "clean staging",
                &staging,
                source,
            )),
        }),
    };

    if let Err(primary) = swap_res {
        return Err(rollback(root, &backup, &mut journal, primary));
    }

    // Save intermediate journal with recorded files
    write_journal_typed(root, &journal)?;

    // 3. Commit metadata: unify configuration commit with transaction journal state
    if let Err(source) = commit_metadata() {
        let primary = VersionChangeError::MetadataCommit(Box::new(source));
        return Err(rollback(root, &backup, &mut journal, primary));
    }

    // 4. Finalize journal state
    journal.phase = TransactionPhase::Committed;
    write_journal_typed(root, &journal).map_err(|source| VersionChangeError::CommitJournal {
        backup_path: backup,
        source: Box::new(source),
    })
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
) -> Result<(), VersionChangeError>
where
    F: FnOnce() -> Result<(), E>,
    E: std::error::Error + Send + Sync + 'static,
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
}

pub async fn apply(
    client: &reqwest::Client,
    target: &Target,
    plan: &VersionPlan,
    backup_worlds: bool,
    progress: impl Fn(String) + Sync,
) -> Result<(), VersionChangeError> {
    apply_typed(client, target, plan, backup_worlds, progress).await
}

fn with_staging_cleanup(
    primary: VersionChangeError,
    staging: &std::path::Path,
) -> VersionChangeError {
    match remove_path(staging) {
        Ok(()) => primary,
        Err(source) => VersionChangeError::AdditionalFailure {
            primary: Box::new(primary),
            secondary: Box::new(super::super::error::file_io(
                "clean staging",
                staging,
                source,
            )),
        },
    }
}

fn rollback(
    root: &std::path::Path,
    backup: &std::path::Path,
    journal: &mut Journal,
    primary: VersionChangeError,
) -> VersionChangeError {
    rollback_with(
        root,
        backup,
        journal,
        primary,
        write_journal_typed,
        super::recovery::cleanup_backup,
    )
}

fn rollback_with(
    root: &std::path::Path,
    backup: &std::path::Path,
    journal: &mut Journal,
    primary: VersionChangeError,
    checkpoint: impl Fn(&std::path::Path, &Journal) -> Result<(), VersionChangeError>,
    cleanup: impl FnOnce(&std::path::Path, &std::path::Path) -> Result<(), VersionChangeError>,
) -> VersionChangeError {
    if let Err(restore) = restore(root, backup, journal) {
        journal.phase = TransactionPhase::RollbackFailed;
        let checkpoint = checkpoint(root, journal).err().map(Box::new);
        return VersionChangeError::RollbackFailed {
            primary: Box::new(primary),
            restore: Box::new(restore),
            checkpoint,
            backup_path: backup.to_owned(),
        };
    }
    // Restored permits retrying cleanup without replaying file changes or metadata.
    journal.phase = TransactionPhase::Restored;
    if let Err(secondary) = checkpoint(root, journal) {
        return VersionChangeError::AdditionalFailure {
            primary: Box::new(primary),
            secondary: Box::new(secondary),
        };
    }
    match cleanup(root, backup) {
        Ok(()) => VersionChangeError::RollbackSucceeded(Box::new(primary)),
        Err(secondary) => VersionChangeError::AdditionalFailure {
            primary: Box::new(primary),
            secondary: Box::new(secondary),
        },
    }
}

#[cfg(test)]
mod tests;
