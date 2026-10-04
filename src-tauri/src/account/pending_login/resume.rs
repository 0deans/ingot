//! Safe public metadata and vault-only explicit retry of a retained refresh token.
use super::*;

const RETENTION_SECONDS: u64 = 24 * 60 * 60;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
pub enum PendingLoginStatus {
    Ready,
    Busy,
    Expired,
    Legacy,
}

#[taurpc::ipc_type]
#[derive(Debug)]
#[serde(rename_all = "camelCase")]
pub struct PendingMicrosoftLogin {
    pub ticket: String,
    pub created_at: u64,
    pub expires_at: u64,
    pub status: PendingLoginStatus,
}

// No Debug: the refresh token must never enter diagnostics or IPC metadata.
pub(crate) struct ResumeGrant {
    pub(crate) receipt: GrantReceipt,
    pub(crate) refresh_token: String,
}

impl PendingLoginGrants {
    pub(crate) fn inspect(
        &self,
        store: &dyn CredentialStore,
        now: u64,
    ) -> Result<Option<PendingMicrosoftLogin>, PendingGrantError> {
        let Some(record) = self.read(store)? else {
            return Ok(None);
        };
        let expires_at = record
            .created_at
            .checked_add(RETENTION_SECONDS)
            .ok_or(PendingGrantError::Invalid)?;
        let status = if record.state == GrantState::Legacy {
            PendingLoginStatus::Legacy
        } else if now < record.created_at || now >= expires_at {
            PendingLoginStatus::Expired
        } else {
            PendingLoginStatus::Ready
        };
        Ok(Some(PendingMicrosoftLogin {
            ticket: record.ticket.to_string(),
            created_at: record.created_at,
            expires_at,
            status,
        }))
    }

    pub(crate) fn check_ticket(
        &self,
        store: &dyn CredentialStore,
        ticket: &str,
    ) -> Result<(), PendingGrantError> {
        self.matching(store, ticket).map(|_| ())
    }

    pub(crate) fn resume_grant(
        &self,
        store: &dyn CredentialStore,
        ticket: &str,
        now: u64,
    ) -> Result<ResumeGrant, PendingGrantError> {
        let record = self.matching(store, ticket)?;
        if record.state == GrantState::Legacy {
            return Err(PendingGrantError::Legacy);
        }
        let expires_at = record
            .created_at
            .checked_add(RETENTION_SECONDS)
            .ok_or(PendingGrantError::Invalid)?;
        if now < record.created_at || now >= expires_at {
            return Err(PendingGrantError::Expired);
        }
        Ok(ResumeGrant {
            receipt: GrantReceipt::new(self.scope, record.ticket),
            refresh_token: record.refresh_token,
        })
    }

    pub(crate) fn rotate(
        &self,
        store: &dyn CredentialStore,
        receipt: GrantReceipt,
        refresh_token: &str,
    ) -> Result<(), PendingGrantError> {
        if receipt.scope() != self.scope {
            return Err(PendingGrantError::NotFound);
        }
        let mut record = self.matching(store, &receipt.ticket().to_string())?;
        if record.state != GrantState::AwaitingExchange {
            return Err(PendingGrantError::Legacy);
        }
        record.refresh_token = refresh_token.to_owned();
        validate(&record)?;
        let value = serde_json::to_string(&record).map_err(|_| PendingGrantError::Invalid)?;
        store.write(&self.key, &value)?;
        Ok(())
    }

    fn matching(
        &self,
        store: &dyn CredentialStore,
        ticket: &str,
    ) -> Result<PendingGrant, PendingGrantError> {
        let ticket = uuid::Uuid::parse_str(ticket).map_err(|_| PendingGrantError::NotFound)?;
        if ticket.is_nil() {
            return Err(PendingGrantError::NotFound);
        }
        let record = self.read(store)?.ok_or(PendingGrantError::NotFound)?;
        if record.ticket != ticket {
            return Err(PendingGrantError::NotFound);
        }
        Ok(record)
    }
}
