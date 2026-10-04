//! Minecraft token XUID extraction, preserving legacy claim handling.
/// The Xbox user ID the game reports as `auth_xuid`, read from the Minecraft token (a JWT)
pub fn xuid_from_token(access_token: &str) -> Option<String> {
    use base64::Engine;

    let payload = access_token.split('.').nth(1)?;
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(payload.trim_end_matches('='))
        .ok()?;
    let claims: serde_json::Value = serde_json::from_slice(&bytes).ok()?;
    match &claims["xuid"] {
        serde_json::Value::String(s) => Some(s.clone()),
        serde_json::Value::Number(n) => Some(n.to_string()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine;
    use serde_json::json;

    fn jwt(claims: serde_json::Value) -> String {
        let b64 = |v: &[u8]| base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(v);
        format!(
            "{}.{}.sig",
            b64(br#"{"alg":"none"}"#),
            b64(claims.to_string().as_bytes())
        )
    }

    #[test]
    fn reads_xuid_from_minecraft_token() {
        assert_eq!(
            xuid_from_token(&jwt(json!({ "xuid": "2535400000000000" }))).as_deref(),
            Some("2535400000000000")
        );
        assert_eq!(
            xuid_from_token(&jwt(json!({ "xuid": 2535400000000000u64 }))).as_deref(),
            Some("2535400000000000")
        );
    }

    #[test]
    fn missing_or_malformed_xuid_is_none() {
        assert_eq!(xuid_from_token(&jwt(json!({ "sub": "x" }))), None);
        assert_eq!(xuid_from_token("not-a-jwt"), None);
    }
}
