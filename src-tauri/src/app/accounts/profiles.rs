//! Native profile commands; pure list decisions live in account::policy.
use super::storage::{current_timestamp, with_recovered_typed as with_recovered};
use crate::account::AccountProfile;

#[derive(Debug, thiserror::Error)]
pub(crate) enum ProfileError {
    #[error("Player nickname cannot be empty")]
    EmptyNickname,
    #[error(transparent)]
    Validation(#[from] crate::account::validation::AccountValidationError),
    #[error(transparent)]
    Ownership(#[from] crate::account::operations::OperationError),
    #[error(transparent)]
    Recovery(#[from] crate::account::recovery::RecoveryError),
}

pub(crate) async fn add_offline_account<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    username: String,
) -> Result<AccountProfile, ProfileError> {
    let clean_name = username.trim();
    if clean_name.is_empty() {
        return Err(ProfileError::EmptyNickname);
    }

    // Minecraft offline UUID: MD5 nameUUIDFromBytes("OfflinePlayer:" + name)
    let offline_uuid = uuid::Uuid::new_v3(
        &uuid::Uuid::NAMESPACE_DNS,
        format!("OfflinePlayer:{}", clean_name).as_bytes(),
    )
    .as_simple()
    .to_string();

    let account_id = format!("offline:{}", clean_name.to_lowercase());
    let new_profile = AccountProfile {
        id: account_id.clone(),
        account_type: "offline".to_string(),
        username: clean_name.to_string(),
        uuid: offline_uuid,
        skin_url: None,
        is_active: true,
        created_at: current_timestamp(),
    };
    crate::account::validation::validate_profile(&new_profile).map_err(ProfileError::from)?;

    let _operation = crate::app::state(&app)
        .account_operations
        .acquire(&account_id)
        .await
        .map_err(ProfileError::from)?;
    with_recovered(&app, |repository| repository.activate(new_profile)).map_err(ProfileError::from)
}

pub(super) fn save_skin_url<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    account_id: &str,
    skin_url: Option<String>,
) -> Result<(), ProfileError> {
    with_recovered(app, |repository| {
        repository.update_profile(account_id, |account| account.skin_url = skin_url)
    })
    .map_err(ProfileError::from)
}
pub(crate) fn get_accounts<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
) -> Result<Vec<AccountProfile>, ProfileError> {
    with_recovered(&app, |repository| repository.load()).map_err(ProfileError::from)
}

pub(crate) fn set_active_account<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    account_id: String,
) -> Result<(), ProfileError> {
    with_recovered(&app, |repository| {
        repository.update(|accounts| {
            crate::account::policy::select(accounts, &account_id)?;
            Ok(())
        })
    })
    .map_err(ProfileError::from)
}
pub(crate) async fn remove_account<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    account_id: String,
) -> Result<(), ProfileError> {
    let state = crate::app::state(&app);
    let _operation = state
        .account_operations
        .acquire(&account_id)
        .await
        .map_err(ProfileError::from)?;
    let accounts =
        with_recovered(&app, |repository| repository.load()).map_err(ProfileError::from)?;
    if accounts
        .iter()
        .any(|account| account.id == account_id && account.account_type == "offline")
    {
        return with_recovered(&app, |repository| {
            repository.update(|accounts| {
                crate::account::policy::remove(accounts, &account_id)?;
                Ok(())
            })
        })
        .map_err(ProfileError::from);
    }
    state
        .account_recovery
        .remove(state.credentials.as_ref(), &state.accounts, &_operation)
        .map_err(ProfileError::from)
}

#[cfg(test)]
mod tests;
pub(crate) fn reorder_accounts<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    account_ids: Vec<String>,
) -> Result<(), ProfileError> {
    with_recovered(&app, |repository| {
        repository.update(|accounts| {
            *accounts = crate::account::policy::reorder(accounts, &account_ids)?;
            Ok(())
        })
    })
    .map_err(ProfileError::from)
}
