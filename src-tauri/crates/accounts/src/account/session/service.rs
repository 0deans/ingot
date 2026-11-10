use super::SessionError;
use crate::account::{
    credentials::CredentialStore, login::LoginAttempts, operations::AccountOperations,
    pending_login::PendingLoginGrants, provider::SessionProvider, recovery::AccountRecovery,
    repository::AccountRepository, AccountProfile,
};

pub struct SessionService<'a> {
    pub repository: &'a AccountRepository,
    pub credentials: &'a dyn CredentialStore,
    pub operations: &'a AccountOperations,
    pub recovery: &'a AccountRecovery,
    pub provider: &'a dyn SessionProvider,
    pub login: &'a LoginAttempts,
    pub pending_login: &'a PendingLoginGrants,
    pub now: fn() -> u64,
}

impl SessionService<'_> {
    pub(super) fn profiles(&self) -> Result<Vec<AccountProfile>, SessionError> {
        Ok(self
            .recovery
            .with_recovered(self.credentials, self.repository, || self.repository.load())?)
    }

    pub(super) fn profile(&self, id: &str) -> Result<AccountProfile, SessionError> {
        self.profiles()?
            .into_iter()
            .find(|profile| profile.id == id)
            .ok_or(SessionError::AccountNotFound)
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
