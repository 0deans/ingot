use serde::{Deserialize, Serialize};
#[taurpc::ipc_type]
#[derive(Debug)]
#[serde(rename_all = "camelCase")]
pub struct AccountProfile {
    pub id: String,
    pub account_type: String, // "ely", "microsoft", "offline"
    pub username: String,
    pub uuid: String,
    pub skin_url: Option<String>,
    pub is_active: bool,
    pub created_at: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountSecrets {
    pub access_token: String,
    pub client_token: String,
    #[serde(default)]
    pub password: Option<String>,
    /// Microsoft accounts: renews the Minecraft token without signing in again
    #[serde(default)]
    pub refresh_token: Option<String>,
    /// Microsoft accounts: when the Minecraft token expires (unix seconds)
    #[serde(default)]
    pub expires_at: Option<u64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_json_retains_provider_ids_camel_case_and_all_metadata() {
        let stored = serde_json::json!([
            {"id":"ely:123", "accountType":"ely", "username":"Player", "uuid":"123", "skinUrl":"https://example.test/skin.png", "isActive":true, "createdAt":42},
            {"id":"microsoft:456", "accountType":"microsoft", "username":"Other", "uuid":"456", "skinUrl":null, "isActive":false, "createdAt":43},
            {"id":"offline:local", "accountType":"offline", "username":"Local", "uuid":"789", "skinUrl":null, "isActive":false, "createdAt":44}
        ]);
        let profiles: Vec<AccountProfile> = serde_json::from_value(stored.clone()).unwrap();
        assert_eq!(serde_json::to_value(profiles).unwrap(), stored);
    }

    #[test]
    fn legacy_credentials_keep_optional_refresh_and_password_defaults() {
        let stored =
            serde_json::json!({"accessToken":"fixture-access", "clientToken":"fixture-client"});
        let secrets: AccountSecrets = serde_json::from_value(stored).unwrap();
        assert_eq!(secrets.access_token, "fixture-access");
        assert_eq!(secrets.client_token, "fixture-client");
        assert!(secrets.password.is_none());
        assert!(secrets.refresh_token.is_none());
        assert!(secrets.expires_at.is_none());
    }
}
