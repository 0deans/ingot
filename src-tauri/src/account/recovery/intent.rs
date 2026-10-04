use crate::account::{
    policy, repository::AccountRepository, AccountError, AccountProfile, AccountSecrets,
};
use serde::{Deserialize, Serialize};

// Deliberately no Debug: recovery records contain secrets.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Intent {
    pub version: u8,
    pub change: ProfileChange,
    pub secrets: Option<AccountSecrets>,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "kind", deny_unknown_fields)]
pub(crate) enum ProfileChange {
    Activate {
        profile: AccountProfile,
    },
    MicrosoftMetadata {
        id: String,
        username: String,
        skin_url: Option<String>,
    },
    Keep {
        id: String,
    },
    Remove {
        id: String,
    },
}

impl ProfileChange {
    pub(super) fn id(&self) -> &str {
        match self {
            Self::Activate { profile } => &profile.id,
            Self::MicrosoftMetadata { id, .. } | Self::Keep { id } | Self::Remove { id } => id,
        }
    }

    pub(super) fn removes(&self) -> bool {
        matches!(self, Self::Remove { .. })
    }

    pub(super) fn apply(&self, repository: &AccountRepository) -> Result<(), AccountError> {
        match self {
            Self::Activate { profile } => repository.activate(profile.clone()).map(|_| ()),
            Self::MicrosoftMetadata {
                id,
                username,
                skin_url,
            } => repository.update_profile(id, |profile| {
                profile.username = username.clone();
                profile.skin_url = skin_url.clone();
            }),
            Self::Keep { id } => repository.update_profile(id, |_| {}),
            Self::Remove { id } => repository.update(|profiles| {
                // Replay after an already committed deletion is idempotent.
                if profiles.iter().any(|profile| profile.id == *id) {
                    policy::remove(profiles, id)?;
                }
                Ok(())
            }),
        }
    }
}
