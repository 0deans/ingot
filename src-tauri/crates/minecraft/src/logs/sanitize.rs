#[derive(Debug, Clone, Default)]
pub struct SanitizeContext {
    pub usernames: Vec<String>,
    pub uuids: Vec<String>,
    pub os_username: Option<String>,
    pub home_dirs: Vec<String>,
}

impl SanitizeContext {
    pub fn from_system(usernames: Vec<String>, uuids: Vec<String>) -> Self {
        let mut home_dirs = Vec::new();
        for var in ["USERPROFILE", "HOME"] {
            if let Ok(val) = std::env::var(var) {
                let trimmed = val.trim();
                if !trimmed.is_empty() {
                    home_dirs.push(trimmed.to_string());
                }
            }
        }
        let os_username = std::env::var("USERNAME")
            .or_else(|_| std::env::var("USER"))
            .ok()
            .map(|u| u.trim().to_string())
            .filter(|u| u.len() >= 2);

        Self {
            usernames,
            uuids,
            os_username,
            home_dirs,
        }
    }
}

pub fn sanitize_log_content(raw: &str, ctx: &SanitizeContext) -> String {
    let mut text = raw.to_string();

    text = redact_flag_values(
        &text,
        &[
            "--accessToken",
            "--clientToken",
            "--session",
            "--xuid",
            "--password",
        ],
        "[REDACTED_TOKEN]",
    );
    text = redact_flag_values(&text, &["--uuid"], "[REDACTED_UUID]");
    text = redact_flag_values(&text, &["--username", "--name"], "[REDACTED_USER]");

    text = redact_prefix_token(&text, "(Session ID is ", ')', "[REDACTED_SESSION]");
    text = redact_prefix_token(&text, "Session ID is ", ' ', "[REDACTED_SESSION]");
    text = redact_prefix_token(&text, "Setting user: ", '\n', "[REDACTED_USER]");
    text = redact_prefix_token(&text, "Bearer ", ' ', "[REDACTED_TOKEN]");

    text = redact_json_field(
        &text,
        &[
            "accessToken",
            "access_token",
            "refreshToken",
            "refresh_token",
            "clientToken",
            "client_token",
            "password",
        ],
        "[REDACTED_TOKEN]",
    );

    text = redact_jwt_tokens(&text);

    for home in &ctx.home_dirs {
        if home.len() >= 3 {
            text = text.replace(home, "~");
            let slash_normalized = home.replace('\\', "/");
            if slash_normalized != *home {
                text = text.replace(&slash_normalized, "~");
            }
            let double_backslash = home.replace('\\', "\\\\");
            if double_backslash != *home {
                text = text.replace(&double_backslash, "~");
            }
        }
    }

    text = redact_standard_home_paths(&text);

    if let Some(ref os_user) = ctx.os_username {
        if os_user.len() >= 3 && !is_common_word(os_user) {
            text = replace_word_case_insensitive(&text, os_user, "[USER]");
        }
    }

    for uuid in &ctx.uuids {
        let trimmed = uuid.trim();
        if trimmed.len() >= 8 {
            text = replace_case_insensitive(&text, trimmed, "[REDACTED_UUID]");
            let compact = trimmed.replace('-', "");
            if compact.len() >= 16 {
                text = replace_case_insensitive(&text, &compact, "[REDACTED_UUID]");
            }
        }
    }

    for username in &ctx.usernames {
        let trimmed = username.trim();
        if trimmed.len() >= 3 && !is_common_word(trimmed) {
            text = replace_word_case_insensitive(&text, trimmed, "[REDACTED_USER]");
        }
    }

    text = redact_ip_addresses(&text);

    text
}

fn is_common_word(word: &str) -> bool {
    matches!(
        word.to_ascii_lowercase().as_str(),
        "user"
            | "admin"
            | "root"
            | "player"
            | "minecraft"
            | "java"
            | "server"
            | "client"
            | "main"
            | "info"
            | "warn"
            | "error"
            | "debug"
            | "mods"
            | "true"
            | "false"
            | "null"
            | "test"
    )
}

fn redact_flag_values(input: &str, flags: &[&str], replacement: &str) -> String {
    let mut current = input.to_string();
    for flag in flags {
        current = redact_single_flag(&current, flag, replacement);
    }
    current
}

fn redact_single_flag(input: &str, flag: &str, replacement: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut rest = input;

    while let Some(idx) = rest.find(flag) {
        out.push_str(&rest[..idx]);
        out.push_str(flag);
        let after_flag = &rest[idx + flag.len()..];

        if let Some(after_eq) = after_flag.strip_prefix('=') {
            out.push('=');
            let value_len = token_value_len(after_eq);
            if value_len > 0 {
                out.push_str(replacement);
                rest = &after_eq[value_len..];
            } else {
                rest = after_eq;
            }
        } else if after_flag.starts_with(' ') || after_flag.starts_with('\t') {
            let ws_len = after_flag
                .chars()
                .take_while(|c| *c == ' ' || *c == '\t')
                .map(char::len_utf8)
                .sum::<usize>();
            out.push_str(&after_flag[..ws_len]);
            let after_ws = &after_flag[ws_len..];
            let value_len = token_value_len(after_ws);
            if value_len > 0 {
                out.push_str(replacement);
                rest = &after_ws[value_len..];
            } else {
                rest = after_ws;
            }
        } else {
            rest = after_flag;
        }
    }
    out.push_str(rest);
    out
}

fn token_value_len(s: &str) -> usize {
    if let Some(stripped) = s.strip_prefix('"') {
        if let Some(end_quote) = stripped.find('"') {
            return end_quote + 2;
        }
    }
    if let Some(stripped) = s.strip_prefix('\'') {
        if let Some(end_quote) = stripped.find('\'') {
            return end_quote + 2;
        }
    }
    s.chars()
        .take_while(|c| {
            !c.is_whitespace() && *c != ',' && *c != ']' && *c != ')' && *c != ';' && *c != '"'
        })
        .map(char::len_utf8)
        .sum()
}

fn redact_prefix_token(input: &str, prefix: &str, terminator: char, replacement: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut rest = input;
    while let Some(idx) = rest.find(prefix) {
        out.push_str(&rest[..idx]);
        out.push_str(prefix);
        let after = &rest[idx + prefix.len()..];
        let len = after
            .chars()
            .take_while(|&c| c != terminator && c != '\n' && c != '\r')
            .map(char::len_utf8)
            .sum::<usize>();
        if len > 0 {
            out.push_str(replacement);
            rest = &after[len..];
        } else {
            rest = after;
        }
    }
    out.push_str(rest);
    out
}

fn redact_json_field(input: &str, keys: &[&str], replacement: &str) -> String {
    let mut current = input.to_string();
    for key in keys {
        for pattern in [format!("\"{key}\":\""), format!("\"{key}\": \"")] {
            let mut out = String::with_capacity(current.len());
            let mut rest = current.as_str();
            while let Some(idx) = rest.find(&pattern) {
                out.push_str(&rest[..idx]);
                out.push_str(&pattern);
                let after = &rest[idx + pattern.len()..];
                if let Some(end_quote) = after.find('"') {
                    out.push_str(replacement);
                    rest = &after[end_quote..];
                } else {
                    rest = after;
                }
            }
            out.push_str(rest);
            current = out;
        }
    }
    current
}

fn redact_jwt_tokens(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut rest = input;
    while let Some(idx) = rest.find("eyJ") {
        let before = &rest[..idx];
        let valid_start = before
            .chars()
            .next_back()
            .is_none_or(|c| !c.is_ascii_alphanumeric());
        out.push_str(before);
        let candidate = &rest[idx..];
        let jwt_len = candidate
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_' || *c == '.')
            .map(char::len_utf8)
            .sum::<usize>();
        let token = &candidate[..jwt_len];
        if valid_start && jwt_len >= 24 && token.matches('.').count() >= 1 {
            out.push_str("[REDACTED_TOKEN]");
            rest = &candidate[jwt_len..];
        } else {
            out.push_str("eyJ");
            rest = &candidate[3..];
        }
    }
    out.push_str(rest);
    out
}

fn redact_standard_home_paths(input: &str) -> String {
    let mut current = input.to_string();
    for prefix in [
        "C:\\Users\\",
        "c:\\Users\\",
        "C:/Users/",
        "c:/Users/",
        "/home/",
        "/Users/",
    ] {
        let mut out = String::with_capacity(current.len());
        let mut rest = current.as_str();
        while let Some(idx) = rest.find(prefix) {
            out.push_str(&rest[..idx]);
            out.push_str(prefix);
            let after = &rest[idx + prefix.len()..];
            let user_len = after
                .chars()
                .take_while(|&c| {
                    c != '\\'
                        && c != '/'
                        && !c.is_whitespace()
                        && c != '"'
                        && c != '\''
                        && c != ';'
                        && c != ':'
                })
                .map(char::len_utf8)
                .sum::<usize>();
            let username_segment = &after[..user_len];
            let next_char = after[user_len..].chars().next();
            if user_len > 0 && matches!(next_char, Some('\\' | '/')) && username_segment != "[USER]"
            {
                out.push_str("[USER]");
                rest = &after[user_len..];
            } else {
                rest = after;
            }
        }
        out.push_str(rest);
        current = out;
    }
    current
}

fn replace_case_insensitive(input: &str, needle: &str, replacement: &str) -> String {
    if needle.is_empty() {
        return input.to_string();
    }
    let lower_input = input.to_ascii_lowercase();
    let lower_needle = needle.to_ascii_lowercase();
    if lower_input.len() != input.len() {
        return input.replace(needle, replacement);
    }
    let mut out = String::with_capacity(input.len());
    let mut last = 0;
    while let Some(pos) = lower_input[last..].find(&lower_needle) {
        let abs = last + pos;
        out.push_str(&input[last..abs]);
        out.push_str(replacement);
        last = abs + needle.len();
    }
    out.push_str(&input[last..]);
    out
}

fn replace_word_case_insensitive(input: &str, word: &str, replacement: &str) -> String {
    if word.is_empty() {
        return input.to_string();
    }
    let lower_input = input.to_ascii_lowercase();
    let lower_word = word.to_ascii_lowercase();
    if lower_input.len() != input.len() {
        return input.replace(word, replacement);
    }
    let bytes = input.as_bytes();
    let mut out = String::with_capacity(input.len());
    let mut last = 0;
    while let Some(pos) = lower_input[last..].find(&lower_word) {
        let start = last + pos;
        let end = start + word.len();
        let prev_ok = start == 0 || !is_ident_byte(bytes[start - 1]);
        let next_ok = end == bytes.len() || !is_ident_byte(bytes[end]);
        if prev_ok && next_ok {
            out.push_str(&input[last..start]);
            out.push_str(replacement);
            last = end;
        } else {
            out.push_str(&input[last..start + 1]);
            last = start + 1;
        }
    }
    out.push_str(&input[last..]);
    out
}

fn is_ident_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

fn redact_ip_addresses(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let bytes = input.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        if bytes[i].is_ascii_digit() {
            let prev_ok = i == 0
                || (!bytes[i - 1].is_ascii_alphanumeric()
                    && bytes[i - 1] != b'.'
                    && bytes[i - 1] != b'-');
            if prev_ok {
                if let Some((ip_len, octets)) = parse_ipv4_at(&input[i..]) {
                    let end = i + ip_len;
                    let next_ok = end == bytes.len()
                        || (!bytes[end].is_ascii_alphanumeric() && bytes[end] != b'.');
                    let is_reserved_or_version = octets[0] == 0
                        || octets[0] == 127
                        || (octets[0] <= 2 && octets[1] <= 30 && octets[2] <= 30);
                    if next_ok && !is_reserved_or_version {
                        out.push_str("[REDACTED_IP]");
                        i = end;
                        continue;
                    }
                }
            }
        }
        let ch = input[i..].chars().next().unwrap_or('\0');
        out.push(ch);
        i += ch.len_utf8();
    }

    out
}

fn parse_ipv4_at(s: &str) -> Option<(usize, [u8; 4])> {
    let mut octets = [0u8; 4];
    let mut cursor = 0;
    for (idx, slot) in octets.iter_mut().enumerate() {
        let rest = &s[cursor..];
        let digit_len = rest.bytes().take_while(|b| b.is_ascii_digit()).count();
        if digit_len == 0 || digit_len > 3 {
            return None;
        }
        let part = &rest[..digit_len];
        if part.len() > 1 && part.starts_with('0') {
            return None;
        }
        *slot = part.parse::<u8>().ok()?;
        cursor += digit_len;
        if idx < 3 {
            if !s[cursor..].starts_with('.') {
                return None;
            }
            cursor += 1;
        }
    }
    Some((cursor, octets))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_tokens_uuids_usernames_paths_and_ips_while_preserving_versions() {
        let ctx = SanitizeContext {
            usernames: vec!["SteveSecretName".to_string()],
            uuids: vec!["550e8400-e29b-41d4-a716-446655440000".to_string()],
            os_username: Some("denus".to_string()),
            home_dirs: vec!["C:\\Users\\denus".to_string()],
        };

        let raw = "\
[19:44:14] [main/INFO]: Loading Minecraft 1.21.4 with Fabric Loader 0.16.9.2
[19:44:15] [main/INFO]: Args: --username SteveSecretName --uuid 550e8400-e29b-41d4-a716-446655440000 --accessToken eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.abc123xyz
[19:44:16] [Render thread/INFO]: (Session ID is token:secret123:550e8400e29b41d4a716446655440000)
[19:44:17] [Render thread/INFO]: Mod directory: C:\\Users\\denus\\AppData\\Roaming\\ingot\\instances\\test\\mods
[19:44:18] [Render thread/INFO]: Connecting to 203.0.113.42, 25565 (local loopback 127.0.0.1:25565)";

        let sanitized = sanitize_log_content(raw, &ctx);

        assert!(sanitized.contains("Minecraft 1.21.4"));
        assert!(sanitized.contains("0.16.9.2"));
        assert!(sanitized.contains("127.0.0.1:25565"));

        assert!(!sanitized.contains("SteveSecretName"));
        assert!(!sanitized.contains("550e8400-e29b-41d4-a716-446655440000"));
        assert!(!sanitized.contains("550e8400e29b41d4a716446655440000"));
        assert!(!sanitized.contains("eyJhbGci"));
        assert!(!sanitized.contains("secret123"));
        assert!(!sanitized.contains("denus"));
        assert!(!sanitized.contains("203.0.113.42"));

        assert!(sanitized.contains("[REDACTED_USER]"));
        assert!(sanitized.contains("[REDACTED_UUID]"));
        assert!(sanitized.contains("[REDACTED_TOKEN]"));
        assert!(sanitized.contains("[REDACTED_SESSION]"));
        assert!(sanitized.contains("[REDACTED_IP]"));
        assert!(sanitized.contains("~\\AppData\\Roaming"));
    }

    #[test]
    fn redacts_equals_quoted_flags_bearer_and_json_secret_fields() {
        let ctx = SanitizeContext::default();
        let raw = "\
Args: --accessToken=\"tok-1\" --clientToken='tok 2' --uuid=550e8400-e29b-41d4-a716-446655440000 --name='Alt Name'
Auth: Bearer live-bearer-token next
Log: Session ID is sess-abc-999 connected
Setting user: HiddenPlayer
Payload: {\"access_token\":\"secA\",\"refreshToken\": \"secB\",\"password\":\"secC\"}";

        let sanitized = sanitize_log_content(raw, &ctx);

        assert!(sanitized.contains("--accessToken=[REDACTED_TOKEN]"));
        assert!(sanitized.contains("--clientToken=[REDACTED_TOKEN]"));
        assert!(sanitized.contains("--uuid=[REDACTED_UUID]"));
        assert!(sanitized.contains("--name=[REDACTED_USER]"));
        assert!(sanitized.contains("Bearer [REDACTED_TOKEN] next"));
        assert!(sanitized.contains("Session ID is [REDACTED_SESSION] connected"));
        assert!(sanitized.contains("Setting user: [REDACTED_USER]"));
        assert!(sanitized.contains("\"access_token\":\"[REDACTED_TOKEN]\""));
        assert!(sanitized.contains("\"refreshToken\": \"[REDACTED_TOKEN]\""));
        assert!(sanitized.contains("\"password\":\"[REDACTED_TOKEN]\""));
        assert!(!sanitized.contains("secA"));
        assert!(!sanitized.contains("secB"));
        assert!(!sanitized.contains("secC"));
    }

    #[test]
    fn redacts_home_variants_compact_uuids_and_respects_word_and_common_word_boundaries() {
        let ctx = SanitizeContext {
            usernames: vec!["Ada".to_string(), "Player".to_string()],
            uuids: vec!["550E8400-E29B-41D4-A716-446655440000".to_string()],
            os_username: Some("alice".to_string()),
            home_dirs: vec!["C:\\Users\\alice".to_string()],
        };

        let raw = "\
Slash: C:/Users/alice/mods and Escaped: C:\\\\Users\\\\alice\\\\mods
Linux: /home/bob/game and macOS: /Users/carol/Library
Words: Ada joined with Radar and Player plus alice in alice_bot
Compact: 550e8400e29b41d4a716446655440000";

        let sanitized = sanitize_log_content(raw, &ctx);

        assert!(sanitized.contains("Slash: ~/mods and Escaped: ~\\\\mods"));
        assert!(sanitized.contains("/home/[USER]/game and macOS: /Users/[USER]/Library"));
        assert!(sanitized.contains(
            "Words: [REDACTED_USER] joined with Radar and Player plus [USER] in alice_bot"
        ));
        assert!(sanitized.contains("Compact: [REDACTED_UUID]"));
    }

    #[test]
    fn respects_jwt_and_ipv4_boundaries() {
        let ctx = SanitizeContext::default();
        let raw = "\
NotJwtShort: eyJ0eXAiOiJKV1QifQ NotJwtPrefixed: xeyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.sig
RealJwt: eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.sig
VersionsAndInvalidIps: 1.21.4.2 192.168.001.1 999.1.2.3 1.2.3.4-rc1 198.51.100.7";

        let sanitized = sanitize_log_content(raw, &ctx);

        assert!(sanitized.contains("NotJwtShort: eyJ0eXAiOiJKV1QifQ"));
        assert!(sanitized.contains("NotJwtPrefixed: xeyJhbGciOiJIUzI1NiJ9"));
        assert!(sanitized.contains("RealJwt: [REDACTED_TOKEN]"));
        assert!(sanitized.contains("1.21.4.2 192.168.001.1 999.1.2.3 1.2.3.4-rc1 [REDACTED_IP]"));
    }
}
