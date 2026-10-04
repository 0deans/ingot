//! Deliberate resume/discard, with recovery preceding all grant eligibility checks.
use super::{SessionError, SessionService};
use crate::account::{
    pending_login::{PendingLoginStatus, PendingMicrosoftLogin},
    provider::ProviderError,
};

impl SessionService<'_> {
    pub(crate) fn pending_microsoft_login(
        &self,
    ) -> Result<Option<PendingMicrosoftLogin>, SessionError> {
        self.login.with_guard(|| {
            self.profiles()?;
            let mut pending = self.recovery.with_pending_login(|| {
                self.pending_login
                    .inspect(self.credentials, (self.now)())
                    .map_err(SessionError::from)
            })?;
            if self.login.busy() {
                if let Some(pending) = &mut pending {
                    pending.status = PendingLoginStatus::Busy;
                }
            }
            Ok(pending)
        })
    }

    pub(crate) fn discard_pending_microsoft_login(&self, ticket: &str) -> Result<(), SessionError> {
        self.login.discard_with(
            || {
                self.profiles()?;
                self.recovery.with_pending_login(|| {
                    self.pending_login
                        .check_ticket(self.credentials, ticket)
                        .map_err(SessionError::from)
                })
            },
            |_| {
                self.recovery.with_pending_login(|| {
                    self.pending_login
                        .discard(self.credentials)
                        .map_err(SessionError::from)
                })
            },
        )
    }

    pub(crate) async fn resume_microsoft_login(
        &self,
        ticket: &str,
    ) -> Result<crate::account::AccountProfile, SessionError> {
        let (attempt, pending) = self.login.resume_with(|| {
            self.profiles()?;
            self.recovery.with_pending_login(|| {
                self.pending_login
                    .resume_grant(self.credentials, ticket, (self.now)())
                    .map_err(SessionError::from)
            })
        })?;
        let cancelled = || self.login.is_cancelled(attempt.id);
        let result = self
            .provider
            .microsoft_refresh(&pending.refresh_token)
            .await;
        if cancelled() {
            return Err(SessionError::Cancelled);
        }
        let grant = result?;
        {
            let _publication = self
                .login
                .publication(attempt.id)
                .map_err(super::login::login_error)?;
            self.recovery.with_pending_login(|| {
                self.pending_login
                    .rotate(self.credentials, pending.receipt, grant.refresh_token())
                    .map_err(SessionError::ResumeCheckpoint)
            })?;
        }
        let result = self.provider.microsoft_exchange(grant, &cancelled).await;
        if cancelled() {
            return Err(SessionError::Cancelled);
        }
        let session = result.map_err(|error| match error {
            ProviderError::Cancelled => SessionError::Cancelled,
            other => SessionError::Provider(other),
        })?;
        self.publish_sign_in(attempt.id, pending.receipt, session)
            .await
    }
}
