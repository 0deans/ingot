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
            let _ = fs::remove_dir_all(&staging);
            return Err(e);
        }
    }

    // 2. Snapshot, then swap; any failure puts back what was done so far
    progress("Backing up...".into());
    remove_path(&backup).map_err(|error| {
        VersionChangeError::BackupError(format!("Failed to replace previous backup: {error}"))
    })?;
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
