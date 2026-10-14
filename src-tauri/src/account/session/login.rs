use super::{SessionError, SessionService};
use crate::account::{
    login::{AttemptId, LoginError},
    pending_login::GrantReceipt,
    provider::ProviderError,
    recovery::ProfileChange,
    AccountProfile, AccountSecrets,
};
use crate::auth::microsoft::{MicrosoftDeviceCode, MicrosoftGrant, MinecraftSession};

impl SessionService<'_> {
    pub(crate) async fn ely_login(
        &self,
        username: &str,
        password: String,
    ) -> Result<AccountProfile, SessionError> {
        let username = username.trim();
        if username.is_empty() || password.is_empty() {
            return Err(SessionError::InvalidLogin);
        }
        self.profiles()?;
        let client_token = uuid::Uuid::new_v4().to_string();
        let response = self
            .provider
            .ely_login(username, &password, &client_token)
            .await?;
        let profile = response
            .selected_profile
            .ok_or(SessionError::MissingMinecraftProfile)?;
        let secrets = AccountSecrets {
            access_token: response.access_token,
            client_token: response.client_token,
            password: Some(password),
            ..Default::default()
        };
        let profile = AccountProfile {
            id: format!("ely:{}", profile.id),
            account_type: "ely".to_owned(),
            skin_url: Some(format!(
                "https://skinsystem.ely.by/skins/{}.png",
                profile.name
            )),
            username: profile.name,
            uuid: profile.id,
            is_active: true,
            created_at: (self.now)(),
        };
        crate::account::validation::validate_profile(&profile)?;
        let operation = self.operations.acquire(&profile.id).await?;
        self.recovery.publish(
            self.credentials,
            self.repository,
            &operation,
            &secrets,
            ProfileChange::Activate {
                profile: profile.clone(),
            },
        )?;
        Ok(profile)
    }

    pub(crate) async fn microsoft_start(&self) -> Result<MicrosoftDeviceCode, SessionError> {
        let attempt = self.login.begin_with(|| self.discard_pending_sign_in())?;
        let code = self.provider.microsoft_start().await?;
        Ok(self.login.issued(attempt, code)?)
    }

    pub(crate) fn cancel_login(&self) -> Result<(), SessionError> {
        self.login
            .invalidate_with(|| self.discard_pending_sign_in())
    }

    fn discard_pending_sign_in(&self) -> Result<(), SessionError> {
        self.recovery.with_pending_login(|| {
            self.pending_login
                .discard(self.credentials)
                .map_err(SessionError::from)
        })
    }

    pub(crate) async fn microsoft_finish(
        &self,
        code: &MicrosoftDeviceCode,
    ) -> Result<AccountProfile, SessionError> {
        self.profiles()?;
        let attempt = self.login.claim(code)?;
        let cancelled = || self.login.is_cancelled(attempt.id);
        let result = self
            .provider
            .microsoft_finish(&attempt.code, &cancelled)
            .await;
        if cancelled() {
            return Err(SessionError::Cancelled);
        }
        let grant = result.map_err(|error| match error {
            ProviderError::Cancelled => SessionError::Cancelled,
            other => SessionError::Provider(other),
        })?;
        let receipt = self.checkpoint_sign_in(attempt.id, code, &grant)?;
        let result = self.provider.microsoft_exchange(grant, &cancelled).await;
        if cancelled() {
            return Err(SessionError::Cancelled);
        }
        let session = result?;
        self.publish_sign_in(attempt.id, receipt, session).await
    }

    fn checkpoint_sign_in(
        &self,
        attempt: AttemptId,
        code: &MicrosoftDeviceCode,
        grant: &MicrosoftGrant,
    ) -> Result<GrantReceipt, SessionError> {
        let _publication = self.login.publication(attempt).map_err(login_error)?;
        // Keep only the refresh token in the vault. OAuth access tokens remain
        // in this future; no account is created before its identity is known.
        self.recovery.with_pending_login(|| {
            Ok(self.pending_login.checkpoint(
                self.credentials,
                code,
                grant.refresh_token(),
                (self.now)(),
            )?)
        })
    }

    pub(super) async fn publish_sign_in(
        &self,
        attempt: AttemptId,
        receipt: GrantReceipt,
        session: MinecraftSession,
    ) -> Result<AccountProfile, SessionError> {
        let secrets = microsoft_secrets(&session);
        let profile = AccountProfile {
            id: format!("microsoft:{}", session.uuid),
            account_type: "microsoft".to_owned(),
            username: session.username,
            uuid: session.uuid,
            skin_url: session.skin_url,
            is_active: true,
            created_at: (self.now)(),
        };
        crate::account::validation::validate_profile(&profile)?;
        let operation = self.operations.acquire(&profile.id).await?;
        // Serialize the local publication boundary with cancellation and new starts.
        let _publication = self.login.publication(attempt).map_err(login_error)?;
        self.recovery.publish_sign_in(
            self.credentials,
            self.repository,
            &operation,
            &secrets,
            profile.clone(),
            receipt,
        )?;
        self.recovery.with_pending_login(|| {
            self.pending_login
                .discard(self.credentials)
                .map_err(SessionError::PendingGrantCleanup)
        })?;
        Ok(profile)
    }
}

pub(super) fn login_error(error: LoginError) -> SessionError {
    match error {
        LoginError::Stale => SessionError::Cancelled,
        other => SessionError::Login(other),
    }
}

pub(super) fn microsoft_secrets(session: &MinecraftSession) -> AccountSecrets {
    AccountSecrets {
        access_token: session.access_token.clone(),
        refresh_token: Some(session.refresh_token.clone()),
        expires_at: Some(session.expires_at),
        ..Default::default()
    }
}
