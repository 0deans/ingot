//! Vault-only checkpoint before a Microsoft sign-in has a Minecraft account ID.
//! All application calls use AccountRecovery's pending-login gate. Sign-in
//! mutations additionally hold the publication guard to reject stale callbacks.
use super::credentials::{CredentialError, CredentialStore};
use crate::auth::microsoft::MicrosoftDeviceCode;
use serde::{Deserialize, Serialize};
use std::path::Path;

const DISCARDED: &str = "ingot-microsoft-pending-discarded-v1";

mod receipt;
pub(crate) use receipt::GrantReceipt;
mod resume;
pub use resume::{PendingLoginStatus, PendingMicrosoftLogin};

#[derive(Default, PartialEq, Eq, Serialize, Deserialize)]
enum GrantState {
    // Old records cannot prove whether publication already happened.
    #[default]
    Legacy,
    AwaitingExchange,
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum PendingGrantError {
    #[error(transparent)]
    Credentials(#[from] CredentialError),
    #[error("Pending Microsoft sign-in record is invalid; it was preserved")]
    Invalid,
    #[error("A previous Microsoft sign-in grant is still pending")]
    AlreadyPending,
    #[error("Pending Microsoft sign-in was not found")]
    NotFound,
    #[error("Pending Microsoft sign-in expired; please start again")]
    Expired,
    #[error("This older Microsoft sign-in cannot be resumed; please start again")]
    Legacy,
}

// Deliberately no Debug: this record contains a refresh token and device code.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PendingGrant {
    version: u8,
    #[serde(default)]
    state: GrantState,
    ticket: uuid::Uuid,
    code: MicrosoftDeviceCode,
    refresh_token: String,
    created_at: u64,
}

pub(crate) struct PendingLoginGrants {
    scope: uuid::Uuid,
    key: String,
}

impl PendingLoginGrants {
    pub(crate) fn new(scope: &Path) -> Self {
        Self::for_scope(receipt::scope_id(scope))
    }

    fn for_scope(scope: uuid::Uuid) -> Self {
        Self {
            scope,
            key: format!("ingot:microsoft-pending:v1:{scope}"),
        }
    }

    pub(crate) fn checkpoint(
        &self,
        store: &dyn CredentialStore,
        code: &MicrosoftDeviceCode,
        refresh_token: &str,
        created_at: u64,
    ) -> Result<GrantReceipt, PendingGrantError> {
        if self.read(store)?.is_some() {
            return Err(PendingGrantError::AlreadyPending);
        }
        let record = PendingGrant {
            version: 2,
            state: GrantState::AwaitingExchange,
            ticket: uuid::Uuid::new_v4(),
            code: code.clone(),
            refresh_token: refresh_token.to_owned(),
            created_at,
        };
        validate(&record)?;
        let value = serde_json::to_string(&record).map_err(|_| PendingGrantError::Invalid)?;
        store.write(&self.key, &value)?;
        Ok(GrantReceipt::new(self.scope, record.ticket))
    }

    /// The account journal must be durable before calling this. Its full session
    /// then owns recovery; this slot must become inert before account publication.
    /// Replays never dispose of a newer ticket in the same application scope.
    pub(crate) fn acknowledge_transfer(
        store: &dyn CredentialStore,
        receipt: GrantReceipt,
    ) -> Result<(), PendingGrantError> {
        let grants = Self::for_scope(receipt.scope());
        let Some(record) = grants.read(store)? else {
            return Ok(());
        };
        if record.ticket != receipt.ticket() {
            return Ok(());
        }
        if record.state != GrantState::AwaitingExchange {
            return Err(PendingGrantError::Invalid);
        }
        store.write(&grants.key, DISCARDED)?;
        Ok(())
    }

    /// Acknowledge disposal before deleting a possibly chunked native vault record.
    /// A failed cleanup cannot make a partially deleted record eligible for reuse.
    pub(crate) fn discard(&self, store: &dyn CredentialStore) -> Result<(), PendingGrantError> {
        if self.read(store)?.is_some() {
            store.write(&self.key, DISCARDED)?;
        }
        store.delete(&self.key)?;
        Ok(())
    }

    fn read(&self, store: &dyn CredentialStore) -> Result<Option<PendingGrant>, PendingGrantError> {
        let Some(value) = store.read(&self.key)? else {
            return Ok(None);
        };
        if value == DISCARDED {
            return Ok(None);
        }
        // Parsing diagnostics can contain tokens in malformed fields; omit them.
        let record: PendingGrant =
            serde_json::from_str(&value).map_err(|_| PendingGrantError::Invalid)?;
        validate(&record)?;
        Ok(Some(record))
    }
}

fn validate(record: &PendingGrant) -> Result<(), PendingGrantError> {
    let supported = matches!(
        (record.version, &record.state),
        (1, GrantState::Legacy) | (2, GrantState::AwaitingExchange)
    );
    if !supported
        || record.ticket.is_nil()
        || record.refresh_token.trim().is_empty()
        || record.code.device_code.trim().is_empty()
        || record.code.user_code.trim().is_empty()
        || record.code.verification_uri.trim().is_empty()
        || record.code.interval == 0
        || record.code.expires_in == 0
    {
        return Err(PendingGrantError::Invalid);
    }
    Ok(())
}

#[cfg(test)]
#[path = "pending_login/tests.rs"]
mod tests;
