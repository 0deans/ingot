use crate::account::pending_login::GrantReceipt;
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grant: Option<GrantReceipt>,
}

impl Intent {
    pub(super) fn valid_for(&self, repository: &AccountRepository) -> bool {
        let version_valid = match (self.version, self.grant) {
            (1, None) => true,
            (2, Some(receipt)) => {
                receipt.belongs_to(repository.path())
                    && matches!(&self.change, ProfileChange::Activate { profile }
                        if profile.account_type == "microsoft"
                            && profile.id == format!("microsoft:{}", profile.uuid))
            }
            _ => false,
        };
        version_valid
            && self.change.removes() == self.secrets.is_none()
            && ["microsoft:", "ely:", "offline:"]
                .iter()
                .any(|prefix| self.change.id().starts_with(prefix))
    }
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
