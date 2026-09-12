use keyring::Entry;

const SERVICE_NAME: &str = "ingot-launcher";

/// Securely stores an account secret (token / password) in the OS Credential Vault
pub fn save_secret(account_id: &str, secret: &str) -> Result<(), String> {
    let entry = Entry::new(SERVICE_NAME, account_id)
        .map_err(|e| format!("Failed to access OS keyring: {e}"))?;
    entry
        .set_password(secret)
        .map_err(|e| format!("Failed to save secret in OS keyring: {e}"))?;
    Ok(())
}

/// Retrieves an account secret from the OS Credential Vault
pub fn get_secret(account_id: &str) -> Result<Option<String>, String> {
    let entry = Entry::new(SERVICE_NAME, account_id)
        .map_err(|e| format!("Failed to access OS keyring: {e}"))?;
    match entry.get_password() {
        Ok(secret) => Ok(Some(secret)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(format!("Failed to read secret from OS keyring: {e}")),
    }
}

/// Deletes an account secret from the OS Credential Vault
pub fn delete_secret(account_id: &str) -> Result<(), String> {
    let entry = Entry::new(SERVICE_NAME, account_id)
        .map_err(|e| format!("Failed to access OS keyring: {e}"))?;
    match entry.delete_credential() {
        Ok(_) => Ok(()),
        Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(format!("Failed to delete secret from OS keyring: {e}")),
    }
}
