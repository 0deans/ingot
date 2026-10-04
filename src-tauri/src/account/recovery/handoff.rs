//! Transfer an identified Microsoft session from its grant slot to the journal.
use super::{AccountRecovery, Intent, ProfileChange, RecoveryError};
use crate::account::{
    credentials::CredentialStore, operations::AccountOperation, pending_login::GrantReceipt,
    repository::AccountRepository, AccountProfile, AccountSecrets,
};

impl AccountRecovery {
    /// Serialize grant read/modify/write with recovery acknowledgements. The
    /// closure must not call recovery or acquire the sign-in publication lock.
    /// The lock is released before any provider request or account publication.
    pub(crate) fn with_pending_login<T, E: From<RecoveryError>>(
        &self,
        action: impl FnOnce() -> Result<T, E>,
    ) -> Result<T, E> {
        let _guard = self
            .pending_login_lock
            .lock()
            .map_err(|_| RecoveryError::LockPoisoned)?;
        action()
    }

    pub(crate) fn publish_sign_in(
        &self,
        store: &dyn CredentialStore,
        repository: &AccountRepository,
        operation: &AccountOperation,
        secrets: &AccountSecrets,
        profile: AccountProfile,
        receipt: GrantReceipt,
    ) -> Result<(), RecoveryError> {
        if profile.id != operation.id() {
            return Err(RecoveryError::Invalid);
        }
        self.commit(
            store,
            repository,
            Intent {
                version: 2,
                change: ProfileChange::Activate { profile },
                secrets: Some(secrets.clone()),
                grant: Some(receipt),
            },
        )
    }
}
