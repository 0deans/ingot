use crate::app::accounts as account;
use crate::minecraft::launcher::{
    models::LaunchError,
    prepare::{PreparationEffects, PreparationFuture, ResolvedAuth},
};
use std::path::PathBuf;
use tauri::{AppHandle, Runtime};

pub(super) struct NativePreparation<'a, R: Runtime> {
    app: &'a AppHandle<R>,
}
impl<'a, R: Runtime> NativePreparation<'a, R> {
    pub(super) fn new(app: &'a AppHandle<R>) -> Self {
        Self { app }
    }
}
impl<R: Runtime> PreparationEffects for NativePreparation<'_, R> {
    fn ensure_java<'a>(
        &'a self,
        client: &'a reqwest::Client,
        major_version: u32,
        custom_path: Option<&'a str>,
        progress: Option<crate::minecraft::java::JavaProgressFn>,
    ) -> PreparationFuture<'a, PathBuf> {
        Box::pin(async move {
            crate::minecraft::java::ensure_java_runtime(
                self.app,
                client,
                major_version,
                custom_path,
                progress,
            )
            .await
            .map_err(LaunchError::JavaRuntime)
        })
    }
    fn resolve_auth(&self) -> PreparationFuture<'_, ResolvedAuth> {
        Box::pin(async move { resolve_auth(self.app).await })
    }
}

async fn resolve_auth<R: Runtime>(app: &AppHandle<R>) -> Result<ResolvedAuth, LaunchError> {
    let accounts = account::load_accounts_file(app)
        .map_err(|error| LaunchError::Account(error.to_string()))?;
    let active_account = accounts.into_iter().find(|a| a.is_active);

    let auth = match active_account {
        Some(acc) => {
            let is_ely = acc.account_type == "ely";
            let is_microsoft = acc.account_type == "microsoft";
            let token = if is_microsoft || is_ely {
                account::get_account_token(app, &acc.id)
                    .await
                    .map_err(|error| LaunchError::Account(error.to_string()))?
            } else {
                "0".to_string()
            };
            let user_type = if is_microsoft { "msa" } else { "mojang" };
            let xuid = if is_microsoft {
                crate::auth::microsoft::xuid_from_token(&token)
            } else {
                None
            };
            ResolvedAuth {
                player_name: acc.username,
                uuid_str: acc.uuid,
                access_token: token,
                user_type,
                xuid,
                is_ely,
            }
        }
        None => ResolvedAuth {
            player_name: "Player".to_string(),
            uuid_str: uuid::Uuid::new_v4().to_string(),
            access_token: "0".to_string(),
            user_type: "mojang",
            xuid: None,
            is_ely: false,
        },
    };

    Ok(auth)
}
