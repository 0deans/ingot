//! Native provider skin operations and account metadata updates.
use super::profiles::{save_skin_url, ProfileError};
use super::session::microsoft_token_for_locked;
use super::storage::{current_timestamp, with_recovered_typed};
use crate::account::{
    credentials::{self, CredentialError},
    operations::OperationError,
    recovery::RecoveryError,
    session::SessionError,
    AccountProfile, AccountSecrets,
};
use crate::auth::{
    ely::{ElyAuthService, ElyError, ElySkinsCatalogResponse},
    microsoft::{self, MicrosoftSkinError},
};
use base64::Engine;

#[derive(Debug, thiserror::Error)]
pub(crate) enum SkinError {
    #[error(transparent)]
    Ownership(#[from] OperationError),
    #[error(transparent)]
    Recovery(#[from] RecoveryError),
    #[error(transparent)]
    Credentials(#[from] CredentialError),
    #[error(transparent)]
    Session(#[from] SessionError),
    #[error(transparent)]
    Microsoft(#[from] MicrosoftSkinError),
    #[error(transparent)]
    Ely(#[from] ElyError),
    #[error("Invalid base64 skin image: {0}")]
    InvalidImage(#[from] base64::DecodeError),
    #[error("Account not found")]
    AccountNotFound,
    #[error("This is not an Ely.by account")]
    NotEly,
    #[error("PASSWORD_REQUIRED")]
    PasswordRequired,
    #[error("Skin changed at provider, but local account metadata could not be saved: {0}")]
    MetadataAfterProvider(#[source] ProfileError),
    #[error("Skin changed at provider, but its password could not be saved: {0}")]
    CredentialsAfterProvider(#[source] CredentialError),
}

pub(crate) async fn apply_microsoft_skin<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    account_id: &str,
    skin_url: &str,
    is_slim: bool,
) -> Result<(), SkinError> {
    let operation = crate::app::state(app)
        .account_operations
        .acquire(account_id)
        .await?;
    let token = microsoft_token_for_locked(app, &operation).await?;
    let new_url = microsoft::set_skin_from_url(&token, skin_url, is_slim).await?;
    save_skin_url(app, account_id, new_url).map_err(SkinError::MetadataAfterProvider)
}

pub(crate) async fn upload_microsoft_skin<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    account_id: &str,
    image_base64: &str,
    is_slim: bool,
) -> Result<(), SkinError> {
    let png = decode_image(image_base64)?;
    let operation = crate::app::state(app)
        .account_operations
        .acquire(account_id)
        .await?;
    let token = microsoft_token_for_locked(app, &operation).await?;
    let new_url = microsoft::upload_skin(&token, png, is_slim).await?;
    save_skin_url(app, account_id, new_url).map_err(SkinError::MetadataAfterProvider)
}

pub(crate) async fn get_ely_skins_catalog(
    page: u32,
    query: Option<String>,
    sort: Option<String>,
    model: Option<String>,
    uploader: Option<String>,
) -> Result<ElySkinsCatalogResponse, SkinError> {
    Ok(ElyAuthService::new()
        .fetch_catalog(page, query, sort, model, uploader)
        .await?)
}

pub(crate) fn has_ely_web_credentials<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    account_id: &str,
) -> Result<bool, SkinError> {
    with_recovered_typed(app, |_| Ok(()))?;
    match credentials::load(crate::app::state(app).credentials.as_ref(), account_id) {
        Ok(secrets) => Ok(secrets
            .password
            .as_deref()
            .is_some_and(|password| !password.trim().is_empty())),
        Err(CredentialError::Missing) => Ok(false),
        Err(error) => Err(SkinError::Credentials(error)),
    }
}

pub(crate) async fn apply_ely_skin<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    account_id: &str,
    skin_id: u64,
    password: Option<String>,
) -> Result<(), SkinError> {
    change_ely_skin(app, account_id, password, ElySkinAction::Wear(skin_id)).await
}

pub(crate) async fn upload_ely_skin<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    account_id: &str,
    image_base64: &str,
    password: Option<String>,
) -> Result<(), SkinError> {
    let png = decode_image(image_base64)?;
    change_ely_skin(app, account_id, password, ElySkinAction::Upload(png)).await
}

enum ElySkinAction {
    Wear(u64),
    Upload(Vec<u8>),
}

fn decode_image(input: &str) -> Result<Vec<u8>, SkinError> {
    let encoded = input.split_once(',').map_or(input, |(_, data)| data);
    Ok(base64::engine::general_purpose::STANDARD.decode(encoded.trim())?)
}

fn ely_username(accounts: &[AccountProfile], account_id: &str) -> Result<String, SkinError> {
    let account = accounts
        .iter()
        .find(|account| account.id == account_id)
        .ok_or(SkinError::AccountNotFound)?;
    if account.account_type != "ely" {
        return Err(SkinError::NotEly);
    }
    Ok(account.username.clone())
}

// No Debug: this preparation contains user-supplied credentials.
struct ElyPassword {
    value: String,
    updated_secrets: Option<AccountSecrets>,
}

fn prepare_password(
    secrets: Option<AccountSecrets>,
    supplied: Option<String>,
) -> Result<ElyPassword, SkinError> {
    let supplied = supplied.filter(|password| !password.trim().is_empty());
    if let Some(password) = supplied {
        let value = password.trim().to_owned();
        let updated_secrets = secrets.map(|mut secrets| {
            secrets.password = Some(value.clone());
            secrets
        });
        return Ok(ElyPassword {
            value,
            updated_secrets,
        });
    }
    let value = secrets
        .and_then(|secrets| secrets.password)
        .filter(|password| !password.trim().is_empty())
        .ok_or(SkinError::PasswordRequired)?;
    Ok(ElyPassword {
        value,
        updated_secrets: None,
    })
}

async fn change_ely_skin<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    account_id: &str,
    password: Option<String>,
    action: ElySkinAction,
) -> Result<(), SkinError> {
    let state = crate::app::state(app);
    let operation = state.account_operations.acquire(account_id).await?;
    let accounts = with_recovered_typed(app, |repository| repository.load())?;
    let username = ely_username(&accounts, account_id)?;
    let secrets = match credentials::load(state.credentials.as_ref(), account_id) {
        Ok(secrets) => Some(secrets),
        Err(CredentialError::Missing) => None,
        Err(error) => return Err(SkinError::Credentials(error)),
    };
    let password = prepare_password(secrets, password)?;
    let service = ElyAuthService::new();
    match action {
        ElySkinAction::Wear(skin_id) => {
            service
                .wear_skin(&username, &password.value, skin_id)
                .await?
        }
        ElySkinAction::Upload(png) => service.upload_skin(&username, &password.value, png).await?,
    }
    // A rejected provider request must not replace previously verified credentials.
    if let Some(secrets) = password.updated_secrets {
        credentials::save(state.credentials.as_ref(), &operation, &secrets)
            .map_err(SkinError::CredentialsAfterProvider)?;
    }
    let url = format!(
        "https://skinsystem.ely.by/skins/{username}.png?t={}",
        current_timestamp()
    );
    save_skin_url(app, account_id, Some(url)).map_err(SkinError::MetadataAfterProvider)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn password_preparation_keeps_existing_credentials_until_provider_accepts() {
        let secrets = AccountSecrets {
            access_token: "access".into(),
            client_token: "client".into(),
            refresh_token: Some("refresh".into()),
            password: Some("old".into()),
            expires_at: Some(123),
        };
        let prepared = prepare_password(Some(secrets.clone()), Some(" new ".into())).unwrap();
        assert_eq!(prepared.value, "new");
        assert_eq!(
            prepared.updated_secrets.unwrap().password.as_deref(),
            Some("new")
        );
        assert_eq!(secrets.password.as_deref(), Some("old"));
        let prepared = prepare_password(Some(secrets), None).unwrap();
        assert_eq!(prepared.value, "old");
        assert!(prepared.updated_secrets.is_none());
        assert!(matches!(
            prepare_password(None, None),
            Err(SkinError::PasswordRequired)
        ));
    }
    #[test]
    fn base64_decode_failure_does_not_include_image_payload() {
        let error = decode_image("private invalid image").unwrap_err();
        assert!(matches!(error, SkinError::InvalidImage(_)));
        assert!(!format!("{error:?}").contains("private invalid image"));
    }
}
