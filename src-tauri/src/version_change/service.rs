//! Version mutations own the target before provider I/O and patch fresh metadata.
use super::{
    models::{Restored, TargetKind, VersionChangeError, VersionPlan},
    targets, transaction,
};
use crate::minecraft::{
    instance::{InstanceConfig, ModLoaderType},
    launcher::ProcessManager,
    metadata::{MetadataError, MetadataRepository},
};
use crate::server::{ServerConfig, ServerCoreType, ServerProcessManager};
use std::path::Path;

#[derive(Debug, thiserror::Error)]
pub(crate) enum ChangeServiceError {
    #[error(transparent)]
    Metadata(#[from] MetadataError),
    #[error(transparent)]
    Transaction(#[from] VersionChangeError),
    #[error(transparent)]
    Tracking(#[from] crate::server::plugins::tracking::TrackingError),
    #[error("Target is running or another operation owns it")]
    Busy,
    #[error("Target not found")]
    NotFound,
    #[error("Target changed since the plan was checked")]
    StalePlan,
    #[error("Invalid version target: {0}")]
    InvalidTarget(String),
}

#[derive(Clone, Copy)]
pub(crate) enum Restoration {
    Undo,
    Recover,
}

fn find_instance(
    repository: &MetadataRepository<InstanceConfig>,
    id: &str,
) -> Result<InstanceConfig, ChangeServiceError> {
    repository
        .load()?
        .into_iter()
        .find(|record| record.id == id)
        .ok_or(ChangeServiceError::NotFound)
}

fn find_server(
    repository: &MetadataRepository<ServerConfig>,
    id: &str,
) -> Result<ServerConfig, ChangeServiceError> {
    repository
        .load()?
        .into_iter()
        .find(|record| record.id == id)
        .ok_or(ChangeServiceError::NotFound)
}

fn patch_instance(
    repository: &MetadataRepository<InstanceConfig>,
    id: &str,
    version: &str,
    loader: ModLoaderType,
    loader_version: Option<String>,
) -> Result<InstanceConfig, MetadataError> {
    repository.update(|records| {
        let record = records
            .iter_mut()
            .find(|record| record.id == id)
            .ok_or_else(|| MetadataError::Conflict("Instance not found".into()))?;
        record.game_version = version.to_string();
        record.loader = loader;
        record.loader_version = loader_version;
        Ok(record.clone())
    })
}

fn patch_server(
    repository: &MetadataRepository<ServerConfig>,
    id: &str,
    version: &str,
    core: ServerCoreType,
    build: Option<String>,
) -> Result<ServerConfig, MetadataError> {
    repository.update(|records| {
        let record = records
            .iter_mut()
            .find(|record| record.id == id)
            .ok_or_else(|| MetadataError::Conflict("Server not found".into()))?;
        record.game_version = version.to_string();
        record.core = core;
        record.build_number = build;
        Ok(record.clone())
    })
}

pub(crate) async fn apply_instance(
    repository: &MetadataRepository<InstanceConfig>,
    processes: &ProcessManager,
    client: &reqwest::Client,
    root: &Path,
    plan: &VersionPlan,
    backup_worlds: bool,
) -> Result<InstanceConfig, ChangeServiceError> {
    let _ownership = processes
        .reserve_maintenance(&plan.target_id)
        .map_err(|_| ChangeServiceError::Busy)?;
    let current = find_instance(repository, &plan.target_id)?;
    if plan.target_kind != TargetKind::Instance
        || plan.from_game_version != current.game_version
        || plan.from_loader != targets::loader_id(&current.loader)
        || plan.from_loader_version != current.loader_version
    {
        return Err(ChangeServiceError::StalePlan);
    }
    let loader =
        targets::parse_loader(&plan.to_loader).map_err(ChangeServiceError::InvalidTarget)?;
    let target = targets::instance(root, &plan.from_loader, &plan.to_loader);
    transaction::apply_typed_with_commit(
        client,
        &target,
        plan,
        backup_worlds,
        |_| {},
        || {
            patch_instance(
                repository,
                &plan.target_id,
                &plan.to_game_version,
                loader,
                plan.to_loader_version.clone(),
            )
            .map(|_| ())
        },
    )
    .await?;
    find_instance(repository, &plan.target_id)
}

pub(crate) fn restore_instance(
    repository: &MetadataRepository<InstanceConfig>,
    processes: &ProcessManager,
    root: &Path,
    id: &str,
    mode: Restoration,
) -> Result<InstanceConfig, ChangeServiceError> {
    let _ownership = processes
        .reserve_maintenance(id)
        .map_err(|_| ChangeServiceError::Busy)?;
    let current = find_instance(repository, id)?;
    let commit = |restored: &Restored| -> Result<(), ChangeServiceError> {
        let loader =
            targets::parse_loader(&restored.loader).map_err(ChangeServiceError::InvalidTarget)?;
        patch_instance(
            repository,
            id,
            &restored.game_version,
            loader,
            restored.loader_version.clone(),
        )?;
        Ok(())
    };
    match mode {
        Restoration::Undo => {
            transaction::undo_typed_with_commit(
                root,
                &current.game_version,
                &targets::loader_id(&current.loader),
                commit,
            )?;
        }
        Restoration::Recover => {
            transaction::recover_interrupted_transaction_with_commit(root, commit)?;
        }
    }
    find_instance(repository, id)
}

pub(crate) async fn apply_server(
    repository: &MetadataRepository<ServerConfig>,
    processes: &ServerProcessManager,
    client: &reqwest::Client,
    root: &Path,
    plan: &VersionPlan,
    backup_worlds: bool,
) -> Result<ServerConfig, ChangeServiceError> {
    let _ownership = processes
        .reserve_maintenance(&plan.target_id)
        .await
        .map_err(|_| ChangeServiceError::Busy)?;
    let current = find_server(repository, &plan.target_id)?;
    let core = targets::parse_loader(&plan.to_loader).map_err(ChangeServiceError::InvalidTarget)?;
    if plan.target_kind != TargetKind::Server
        || plan.from_game_version != current.game_version
        || plan.from_loader != targets::loader_id(&current.core)
        || plan.from_loader_version != current.build_number
        || !targets::can_switch(&current.core, &core)
    {
        return Err(ChangeServiceError::StalePlan);
    }
    let target = targets::server(root, &core)?;
    transaction::apply_typed_with_commit(
        client,
        &target,
        plan,
        backup_worlds,
        |_| {},
        || -> Result<(), ChangeServiceError> {
            crate::server::plugins::track_version_change(root, &plan.items)?;
            patch_server(
                repository,
                &plan.target_id,
                &plan.to_game_version,
                core,
                None,
            )?;
            Ok(())
        },
    )
    .await?;
    find_server(repository, &plan.target_id)
}

pub(crate) async fn restore_server(
    repository: &MetadataRepository<ServerConfig>,
    processes: &ServerProcessManager,
    root: &Path,
    id: &str,
    mode: Restoration,
) -> Result<ServerConfig, ChangeServiceError> {
    let _ownership = processes
        .reserve_maintenance(id)
        .await
        .map_err(|_| ChangeServiceError::Busy)?;
    let current = find_server(repository, id)?;
    let commit = |restored: &Restored| -> Result<(), ChangeServiceError> {
        let core =
            targets::parse_loader(&restored.loader).map_err(ChangeServiceError::InvalidTarget)?;
        patch_server(
            repository,
            id,
            &restored.game_version,
            core,
            restored.loader_version.clone(),
        )?;
        Ok(())
    };
    match mode {
        Restoration::Undo => {
            transaction::undo_typed_with_commit(
                root,
                &current.game_version,
                &targets::loader_id(&current.core),
                commit,
            )?;
        }
        Restoration::Recover => {
            transaction::recover_interrupted_transaction_with_commit(root, commit)?;
        }
    }
    find_server(repository, id)
}
