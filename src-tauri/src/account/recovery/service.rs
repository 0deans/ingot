//! Roll forward local commits. No providers, awaits, or secrets in filesystem discovery.

use super::intent::Intent;
use super::ProfileChange;
use super::RecoveryError;

use crate::account::{
    credentials::CredentialStore, operations::AccountOperation, repository::AccountRepository,
    AccountError, AccountSecrets,
};
use std::{io::Write, path::PathBuf, sync::Mutex};

const MARKER: &[u8] = b"ingot-account-recovery-v1";
const COMPLETE: &str = "ingot-account-recovery-complete-v1";

pub(crate) struct AccountRecovery {
    marker: PathBuf,
    key: String,
    lock: Mutex<()>,
}

impl AccountRecovery {
    pub(crate) fn new(marker: PathBuf) -> Self {
        // Stable discovery key, isolated for application data directories.
        let scope = uuid::Uuid::new_v3(
            &uuid::Uuid::NAMESPACE_URL,
            marker.to_string_lossy().as_bytes(),
        );
        Self {
            marker,
            key: format!("ingot:account-recovery:v1:{scope}"),
            lock: Mutex::new(()),
        }
    }

    /// All cooperating profile reads/mutations pass here. Replay completes before a
    /// newer selection/removal can be overwritten by an old activation intent.
    pub(crate) fn with_recovered<T>(
        &self,
        store: &dyn CredentialStore,
        repository: &AccountRepository,
        action: impl FnOnce() -> Result<T, AccountError>,
    ) -> Result<T, RecoveryError> {
        let _guard = self.lock.lock().map_err(|_| RecoveryError::LockPoisoned)?;
        self.recover(store, repository)?;
        action().map_err(RecoveryError::Account)
    }

    pub(crate) fn publish(
        &self,
        store: &dyn CredentialStore,
        repository: &AccountRepository,
        operation: &AccountOperation,
        secrets: &AccountSecrets,
        change: ProfileChange,
    ) -> Result<(), RecoveryError> {
        if change.id() != operation.id() || change.removes() {
            return Err(RecoveryError::Invalid);
        }
        self.commit(
            store,
            repository,
            Intent {
                version: 1,
                change,
                secrets: Some(secrets.clone()),
            },
        )
    }

    pub(crate) fn remove(
        &self,
        store: &dyn CredentialStore,
        repository: &AccountRepository,
        operation: &AccountOperation,
    ) -> Result<(), RecoveryError> {
        self.commit(
            store,
            repository,
            Intent {
                version: 1,
                change: ProfileChange::Remove {
                    id: operation.id().to_owned(),
                },
                secrets: None,
            },
        )
    }

    fn commit(
        &self,
        store: &dyn CredentialStore,
        repository: &AccountRepository,
        intent: Intent,
    ) -> Result<(), RecoveryError> {
        let _guard = self.lock.lock().map_err(|_| RecoveryError::LockPoisoned)?;
        // Another account's provider may already have rotated tokens while earlier
        // work became pending. Queue its secrets before requiring that work to finish.
        let mut pending = self.read_pending(store)?;
        // Removal can be rejected before touching either resource. Login/refresh
        // callers preflight before provider I/O; after a remote token rotation we
        // must preserve the new secrets even if the file has since become corrupt.
        if intent.change.removes() {
            self.replay(store, repository, pending)?;
            pending = Vec::new();
            let profiles = repository.load()?;
            if !profiles.iter().any(|p| p.id == intent.change.id()) {
                return Err(AccountError::from(
                    crate::account::policy::AccountPolicyError::NotFound,
                )
                .into());
            }
        }
        pending.push(intent);
        let value = serde_json::to_string(&pending).map_err(|_| RecoveryError::Invalid)?;
        if !self.marker.exists() {
            self.create_marker()?;
        }
        // Marker publication precedes journal publication. A crash in between leaves
        // an empty marker, safe to clear; no canonical credentials were changed yet.
        store.write(&self.key, &value)?;
        self.replay(store, repository, pending)
    }

    fn recover(
        &self,
        store: &dyn CredentialStore,
        repository: &AccountRepository,
    ) -> Result<(), RecoveryError> {
        let pending = self.read_pending(store)?;
        self.replay(store, repository, pending)
    }

    fn read_pending(&self, store: &dyn CredentialStore) -> Result<Vec<Intent>, RecoveryError> {
        match std::fs::read(&self.marker) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(error) => return Err(RecoveryError::Discovery(error)),
            Ok(value) if value != MARKER => return Err(RecoveryError::Invalid),
            Ok(_) => {}
        }
        let Some(value) = store.read(&self.key)? else {
            return Ok(Vec::new());
        };
        if value == COMPLETE {
            return Ok(Vec::new());
        }
        // Parsing errors deliberately omit serde's source: malformed values may
        // contain passwords/tokens in an unexpected field or enum discriminator.
        let pending: Vec<Intent> =
            serde_json::from_str(&value).map_err(|_| RecoveryError::Invalid)?;
        if pending.is_empty()
            || pending.iter().any(|intent| {
                intent.version != 1
                    || intent.change.removes() != intent.secrets.is_none()
                    || !["microsoft:", "ely:", "offline:"]
                        .iter()
                        .any(|prefix| intent.change.id().starts_with(prefix))
            })
        {
            return Err(RecoveryError::Invalid);
        }
        Ok(pending)
    }

    fn replay(
        &self,
        store: &dyn CredentialStore,
        repository: &AccountRepository,
        mut pending: Vec<Intent>,
    ) -> Result<(), RecoveryError> {
        while !pending.is_empty() {
            self.finish(store, repository, &pending[0])?;
            pending.remove(0);
            // Acknowledge each completed intent by publishing the remaining queue.
            // The native vault publishes a new pointer before cleaning old chunks.
            let value = if pending.is_empty() {
                COMPLETE.to_owned()
            } else {
                serde_json::to_string(&pending).map_err(|_| RecoveryError::Invalid)?
            };
            store
                .write(&self.key, &value)
                .map_err(RecoveryError::PendingCredentials)?;
        }
        if self.marker.exists() {
            self.cleanup(store)?;
        }
        Ok(())
    }

    fn finish(
        &self,
        store: &dyn CredentialStore,
        repository: &AccountRepository,
        intent: &Intent,
    ) -> Result<(), RecoveryError> {
        if let Some(secrets) = &intent.secrets {
            let value = serde_json::to_string(secrets).map_err(|_| RecoveryError::Invalid)?;
            store
                .write(intent.change.id(), &value)
                .map_err(RecoveryError::PendingCredentials)?;
        }
        intent
            .change
            .apply(repository)
            .map_err(RecoveryError::Profile)?;
        if intent.change.removes() {
            store
                .delete(intent.change.id())
                .map_err(RecoveryError::PendingCredentials)?;
        }
        Ok(())
    }

    fn cleanup(&self, store: &dyn CredentialStore) -> Result<(), RecoveryError> {
        store
            .delete(&self.key)
            .map_err(RecoveryError::PendingCredentials)?;
        self.clear_marker()
    }

    fn create_marker(&self) -> Result<(), RecoveryError> {
        let parent = self.marker.parent().ok_or(RecoveryError::Invalid)?;
        std::fs::create_dir_all(parent).map_err(RecoveryError::Discovery)?;
        let mut staged =
            tempfile::NamedTempFile::new_in(parent).map_err(RecoveryError::Discovery)?;
        staged.write_all(MARKER).map_err(RecoveryError::Discovery)?;
        staged
            .as_file()
            .sync_all()
            .map_err(RecoveryError::Discovery)?;
        staged
            .persist(&self.marker)
            .map_err(|error| RecoveryError::Discovery(error.error))?;
        Ok(())
    }

    fn clear_marker(&self) -> Result<(), RecoveryError> {
        std::fs::remove_file(&self.marker).map_err(RecoveryError::Discovery)
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
