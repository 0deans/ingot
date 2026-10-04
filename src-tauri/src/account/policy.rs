//! Profile list decisions. Storage and credential effects belong to application adapters.
use super::AccountProfile;

#[derive(Debug, thiserror::Error)]
pub(crate) enum AccountPolicyError {
    #[error("Account not found")]
    NotFound,
}

pub(crate) fn microsoft_token_is_fresh(expires_at: Option<u64>, now_seconds: u64) -> bool {
    const EXPIRY_MARGIN_SECONDS: u64 = 5 * 60;
    expires_at.is_some_and(|expiry| expiry > now_seconds.saturating_add(EXPIRY_MARGIN_SECONDS))
}

pub(crate) fn select(accounts: &mut [AccountProfile], id: &str) -> Result<(), AccountPolicyError> {
    if !accounts.iter().any(|account| account.id == id) {
        return Err(AccountPolicyError::NotFound);
    }
    for account in accounts {
        account.is_active = account.id == id;
    }
    Ok(())
}

pub(crate) fn remove(
    accounts: &mut Vec<AccountProfile>,
    id: &str,
) -> Result<AccountProfile, AccountPolicyError> {
    let position = accounts
        .iter()
        .position(|account| account.id == id)
        .ok_or(AccountPolicyError::NotFound)?;
    let removed = accounts.remove(position);
    if removed.is_active {
        if let Some(first) = accounts.first_mut() {
            first.is_active = true;
        }
    }
    Ok(removed)
}

pub(crate) fn reorder(accounts: &[AccountProfile], ids: &[String]) -> Vec<AccountProfile> {
    let mut reordered = Vec::new();
    for id in ids {
        if let Some(account) = accounts.iter().find(|account| &account.id == id) {
            reordered.push(account.clone());
        }
    }
    for account in accounts {
        if !reordered
            .iter()
            .any(|existing: &AccountProfile| existing.id == account.id)
        {
            reordered.push(account.clone());
        }
    }
    reordered
}

#[cfg(test)]
mod tests;
