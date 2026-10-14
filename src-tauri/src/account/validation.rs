//! Semantic validation of account profiles and profile collections.
use super::AccountProfile;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AccountKind {
    Ely,
    Microsoft,
    Offline,
}

impl AccountKind {
    pub(crate) const ALL: [Self; 3] = [Self::Microsoft, Self::Ely, Self::Offline];
    #[cfg(test)]
    pub(crate) const PREFIXES: [&'static str; 3] = ["microsoft:", "ely:", "offline:"];

    pub(crate) const fn id_prefix(&self) -> &'static str {
        match self {
            Self::Microsoft => "microsoft:",
            Self::Ely => "ely:",
            Self::Offline => "offline:",
        }
    }

    pub(crate) const fn account_type(&self) -> &'static str {
        match self {
            Self::Microsoft => "microsoft",
            Self::Ely => "ely",
            Self::Offline => "offline",
        }
    }

    pub(crate) fn parse(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|&kind| kind.account_type() == s)
    }

    pub(crate) fn parse_id(id: &str) -> Option<(Self, &str)> {
        for kind in Self::ALL {
            if let Some(suffix) = id.strip_prefix(kind.id_prefix()) {
                if !suffix.trim().is_empty() {
                    return Some((kind, suffix));
                }
            }
        }
        None
    }

    pub(crate) fn format_id(&self, suffix: &str) -> String {
        format!("{}{}", self.id_prefix(), suffix)
    }
}

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub(crate) enum AccountValidationError {
    #[error("Unsupported account kind: {0}")]
    UnsupportedKind(String),

    #[error("Missing required identity field '{field}' for account '{id}'")]
    MissingIdentityField { field: &'static str, id: String },

    #[error(
        "Inconsistent provider identity for account '{id}' (accountType: {account_type}, uuid: {uuid})"
    )]
    InconsistentIdentity {
        id: String,
        account_type: String,
        uuid: String,
    },

    #[error("Duplicate account ID found: {0}")]
    DuplicateAccountId(String),

    #[error("Multiple active accounts found (found {0} active accounts)")]
    MultipleActiveAccounts(usize),
}

pub(crate) fn validate_profile(profile: &AccountProfile) -> Result<(), AccountValidationError> {
    if profile.id.trim().is_empty() {
        return Err(AccountValidationError::MissingIdentityField {
            field: "id",
            id: profile.id.clone(),
        });
    }
    if profile.username.trim().is_empty() {
        return Err(AccountValidationError::MissingIdentityField {
            field: "username",
            id: profile.id.clone(),
        });
    }
    if profile.uuid.trim().is_empty() {
        return Err(AccountValidationError::MissingIdentityField {
            field: "uuid",
            id: profile.id.clone(),
        });
    }

    let kind = AccountKind::parse(&profile.account_type)
        .ok_or_else(|| AccountValidationError::UnsupportedKind(profile.account_type.clone()))?;

    let expected_id = match kind {
        AccountKind::Ely | AccountKind::Microsoft => kind.format_id(&profile.uuid),
        AccountKind::Offline => kind.format_id(&profile.username.trim().to_lowercase()),
    };

    if profile.id != expected_id {
        return Err(AccountValidationError::InconsistentIdentity {
            id: profile.id.clone(),
            account_type: profile.account_type.clone(),
            uuid: profile.uuid.clone(),
        });
    }

    Ok(())
}

pub(crate) fn validate_accounts(accounts: &[AccountProfile]) -> Result<(), AccountValidationError> {
    let mut seen_ids = std::collections::HashSet::with_capacity(accounts.len());
    let mut active_count = 0;

    for profile in accounts {
        validate_profile(profile)?;

        if !seen_ids.insert(&profile.id) {
            return Err(AccountValidationError::DuplicateAccountId(
                profile.id.clone(),
            ));
        }

        if profile.is_active {
            active_count += 1;
        }
    }

    if active_count > 1 {
        return Err(AccountValidationError::MultipleActiveAccounts(active_count));
    }

    Ok(())
}

pub(crate) fn is_valid_account_id(id: &str) -> bool {
    AccountKind::parse_id(id).is_some()
}

pub(crate) fn is_valid_microsoft_id(id: &str) -> bool {
    matches!(AccountKind::parse_id(id), Some((AccountKind::Microsoft, _)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn account_id_validation_requires_supported_prefix_and_non_empty_suffix() {
        assert_eq!(AccountKind::PREFIXES, ["microsoft:", "ely:", "offline:"]);

        assert_eq!(
            AccountKind::parse_id("microsoft:123"),
            Some((AccountKind::Microsoft, "123"))
        );
        assert_eq!(
            AccountKind::parse_id("ely:456"),
            Some((AccountKind::Ely, "456"))
        );
        assert_eq!(
            AccountKind::parse_id("offline:player"),
            Some((AccountKind::Offline, "player"))
        );

        assert_eq!(
            AccountKind::Microsoft.format_id("uuid-1"),
            "microsoft:uuid-1"
        );
        assert_eq!(AccountKind::Ely.format_id("uuid-2"), "ely:uuid-2");
        assert_eq!(AccountKind::Offline.format_id("steve"), "offline:steve");

        assert!(is_valid_account_id("microsoft:123"));
        assert!(is_valid_account_id("ely:456"));
        assert!(is_valid_account_id("offline:player"));

        assert!(!is_valid_account_id("microsoft:"));
        assert!(!is_valid_account_id("microsoft:   "));
        assert!(!is_valid_account_id("ely:"));
        assert!(!is_valid_account_id("ely:\t"));
        assert!(!is_valid_account_id("offline:"));
        assert!(!is_valid_account_id("offline: \n "));
        assert!(!is_valid_account_id("unknown:123"));
        assert!(!is_valid_account_id(""));
        assert!(!is_valid_account_id("   "));

        assert!(is_valid_microsoft_id("microsoft:123"));
        assert!(!is_valid_microsoft_id("microsoft:"));
        assert!(!is_valid_microsoft_id("microsoft:   "));
        assert!(!is_valid_microsoft_id("ely:123"));
    }

    fn valid_profile(account_type: &str, username: &str, uuid: &str) -> AccountProfile {
        let id = match account_type {
            "ely" => format!("ely:{uuid}"),
            "microsoft" => format!("microsoft:{uuid}"),
            "offline" => format!("offline:{}", username.trim().to_lowercase()),
            _ => format!("unknown:{uuid}"),
        };
        AccountProfile {
            id,
            account_type: account_type.to_owned(),
            username: username.to_owned(),
            uuid: uuid.to_owned(),
            skin_url: None,
            is_active: false,
            created_at: 100,
        }
    }

    #[test]
    fn fixture_profiles_pass_validation_and_round_trip() {
        let json = include_str!("../../tests/fixtures/accounts-current.json");
        let profiles: Vec<AccountProfile> = serde_json::from_str(json).unwrap();
        assert_eq!(profiles.len(), 3);
        validate_accounts(&profiles).unwrap();
        let serialized = serde_json::to_string(&profiles).unwrap();
        let round_tripped: Vec<AccountProfile> = serde_json::from_str(&serialized).unwrap();
        validate_accounts(&round_tripped).unwrap();
    }

    #[test]
    fn unsupported_account_kinds_are_rejected() {
        for kind in ["mojang", "custom", "", "MICROSOFT", "OfflinePlayer"] {
            let mut profile = valid_profile("offline", "Steve", "steve-uuid");
            profile.account_type = kind.to_owned();
            assert_eq!(
                validate_profile(&profile),
                Err(AccountValidationError::UnsupportedKind(kind.to_owned()))
            );
        }
    }

    #[test]
    fn missing_or_blank_identity_fields_are_rejected() {
        let mut profile = valid_profile("offline", "Steve", "steve-uuid");
        profile.id = "   ".to_owned();
        assert_eq!(
            validate_profile(&profile),
            Err(AccountValidationError::MissingIdentityField {
                field: "id",
                id: "   ".to_owned(),
            })
        );

        let mut profile = valid_profile("offline", "Steve", "steve-uuid");
        profile.username = "".to_owned();
        assert_eq!(
            validate_profile(&profile),
            Err(AccountValidationError::MissingIdentityField {
                field: "username",
                id: profile.id.clone(),
            })
        );

        let mut profile = valid_profile("offline", "Steve", "steve-uuid");
        profile.uuid = "  \t ".to_owned();
        assert_eq!(
            validate_profile(&profile),
            Err(AccountValidationError::MissingIdentityField {
                field: "uuid",
                id: profile.id.clone(),
            })
        );
    }

    #[test]
    fn inconsistent_identities_are_rejected() {
        // Ely mismatch
        let mut ely = valid_profile("ely", "Player", "123");
        ely.id = "ely:999".to_owned();
        assert!(matches!(
            validate_profile(&ely),
            Err(AccountValidationError::InconsistentIdentity { .. })
        ));

        // Microsoft mismatch
        let mut ms = valid_profile("microsoft", "Player", "ms-uuid-1");
        ms.id = "microsoft:other-uuid".to_owned();
        assert!(matches!(
            validate_profile(&ms),
            Err(AccountValidationError::InconsistentIdentity { .. })
        ));

        // Microsoft with ely prefix
        let mut ms_cross = valid_profile("microsoft", "Player", "ms-uuid-1");
        ms_cross.id = "ely:ms-uuid-1".to_owned();
        assert!(matches!(
            validate_profile(&ms_cross),
            Err(AccountValidationError::InconsistentIdentity { .. })
        ));

        // Offline mismatch with username
        let mut offline = valid_profile("offline", "Steve", "off-uuid");
        offline.id = "offline:alex".to_owned();
        assert!(matches!(
            validate_profile(&offline),
            Err(AccountValidationError::InconsistentIdentity { .. })
        ));

        // Offline not lowercase
        let mut offline_case = valid_profile("offline", "Steve", "off-uuid");
        offline_case.id = "offline:Steve".to_owned();
        assert!(matches!(
            validate_profile(&offline_case),
            Err(AccountValidationError::InconsistentIdentity { .. })
        ));
    }

    #[test]
    fn duplicate_account_ids_are_rejected() {
        let a = valid_profile("offline", "Steve", "uuid-1");
        let mut b = valid_profile("offline", "Steve", "uuid-2");
        b.id = a.id.clone();
        let accounts = vec![a, b];
        assert_eq!(
            validate_accounts(&accounts),
            Err(AccountValidationError::DuplicateAccountId(
                "offline:steve".to_owned()
            ))
        );
    }

    #[test]
    fn valid_active_selection_allows_zero_or_one_and_rejects_multiple() {
        // Zero active accounts is legitimate (e.g. initial empty state or guest play)
        let mut a = valid_profile("offline", "Steve", "uuid-1");
        let mut b = valid_profile("microsoft", "Alex", "uuid-2");
        a.is_active = false;
        b.is_active = false;
        assert!(validate_accounts(&[a.clone(), b.clone()]).is_ok());

        // Exactly one active account is valid
        a.is_active = true;
        assert!(validate_accounts(&[a.clone(), b.clone()]).is_ok());

        // Multiple active accounts is rejected
        b.is_active = true;
        assert_eq!(
            validate_accounts(&[a, b]),
            Err(AccountValidationError::MultipleActiveAccounts(2))
        );
    }
}
