use super::SessionError;
use crate::account::{
    credentials::CredentialStore, login::LoginAttempts, operations::AccountOperations,
    pending_login::PendingLoginGrants, provider::SessionProvider, recovery::AccountRecovery,
    repository::AccountRepository, AccountProfile,
};

pub(crate) struct SessionService<'a> {
    pub(crate) repository: &'a AccountRepository,
    pub(crate) credentials: &'a dyn CredentialStore,
    pub(crate) operations: &'a AccountOperations,
    pub(crate) recovery: &'a AccountRecovery,
    pub(crate) provider: &'a dyn SessionProvider,
    pub(crate) login: &'a LoginAttempts,
    pub(crate) pending_login: &'a PendingLoginGrants,
    pub(crate) now: fn() -> u64,
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
