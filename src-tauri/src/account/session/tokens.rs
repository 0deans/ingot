use super::{login::microsoft_secrets, SessionError, SessionService};
use crate::account::{
    credentials, operations::AccountOperation, policy, provider::TokenValidity,
    recovery::ProfileChange, AccountProfile, AccountSecrets,
};

impl SessionService<'_> {
    pub(crate) async fn active_token(&self) -> Result<String, SessionError> {
        let id = self
            .profiles()?
            .into_iter()
            .find(|profile| profile.is_active)
            .ok_or(SessionError::NoActiveAccount)?
            .id;
        self.token_for(&id).await
    }

    pub(crate) async fn token_for(&self, id: &str) -> Result<String, SessionError> {
        let operation = self.operations.acquire(id).await?;
        // Re-read after waiting: removal or re-login may have completed.
        let profile = self.profile(id)?;
        if profile.account_type == "offline" {
            return Ok("offline".to_owned());
        }
        let secrets = credentials::load(self.credentials, id)?;
        if profile.account_type == "microsoft" {
            return self
                .microsoft_access_token(&profile, secrets, &operation)
                .await;
        }
        if self
            .provider
            .ely_validate(&secrets.access_token, &secrets.client_token)
            .await?
            == TokenValidity::Valid
        {
            return Ok(secrets.access_token);
        }
        let response = self
            .provider
            .ely_refresh(&secrets.access_token, &secrets.client_token)
            .await?;
        let secrets = AccountSecrets {
            access_token: response.access_token,
            client_token: response.client_token,
            password: secrets.password,
            ..Default::default()
        };
        self.recovery.publish(
            self.credentials,
            self.repository,
            &operation,
            &secrets,
            ProfileChange::Keep { id: id.to_string() },
        )?;
        Ok(secrets.access_token)
    }

    pub(crate) async fn microsoft_token_for_locked(
        &self,
        operation: &AccountOperation,
    ) -> Result<String, SessionError> {
        let profile = self.profile(operation.id())?;
        if profile.account_type != "microsoft" {
            return Err(SessionError::NotMicrosoft);
        }
        let secrets = credentials::load(self.credentials, operation.id())?;
        self.microsoft_access_token(&profile, secrets, operation)
            .await
    }

    async fn microsoft_access_token(
        &self,
        profile: &AccountProfile,
        secrets: AccountSecrets,
        operation: &AccountOperation,
    ) -> Result<String, SessionError> {
        if policy::microsoft_token_is_fresh(secrets.expires_at, (self.now)()) {
            return Ok(secrets.access_token);
        }
        let refresh = secrets
            .refresh_token
            .as_deref()
            .ok_or(SessionError::MissingRefreshToken)?;
        let grant = self.provider.microsoft_refresh(refresh).await?;
        self.checkpoint_microsoft_rotation(&profile.id, secrets, grant.refresh_token(), operation)?;
        let session = self.provider.microsoft_exchange(grant, &|| false).await?;
        let next = microsoft_secrets(&session);
        self.recovery.publish(
            self.credentials,
            self.repository,
            operation,
            &next,
            ProfileChange::MicrosoftMetadata {
                id: profile.id.clone(),
                username: session.username,
                skin_url: session.skin_url,
            },
        )?;
        Ok(next.access_token)
    }

    fn checkpoint_microsoft_rotation(
        &self,
        id: &str,
        secrets: AccountSecrets,
        refresh_token: &str,
        operation: &AccountOperation,
    ) -> Result<(), SessionError> {
        let checkpoint = AccountSecrets {
            refresh_token: Some(refresh_token.to_owned()),
            // No new Minecraft token exists yet. Never mark the old one as fresh.
            expires_at: Some(0),
            ..secrets
        };
        // No await between receiving the grant and checkpointing its rotation.
        // Dropping the later exchange future cannot discard the new refresh token.
        self.recovery
            .publish(
                self.credentials,
                self.repository,
                operation,
                &checkpoint,
                ProfileChange::Keep { id: id.to_owned() },
            )
            .map_err(SessionError::RotationCheckpoint)
    }
}
