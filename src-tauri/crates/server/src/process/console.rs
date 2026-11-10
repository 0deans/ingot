use std::path::{Path, PathBuf};

/// Strips ANSI escape sequences, carriage returns and JLine's "> " prompt
/// from a console line so it renders cleanly in the UI
pub fn clean_console_line(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut chars = raw.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            // CSI sequence: ESC [ params... final byte (0x40-0x7E)
            if chars.peek() == Some(&'[') {
                chars.next();
                for c in chars.by_ref() {
                    if ('\u{40}'..='\u{7e}').contains(&c) {
                        break;
                    }
                }
            }
            continue;
        }
        if c != '\r' {
            out.push(c);
        }
    }
    let mut trimmed = out.trim_start();
    while let Some(rest) = trimmed.strip_prefix('>') {
        trimmed = rest.trim_start();
    }
    trimmed.trim_end().to_string()
}

/// Helper to locate java.exe (or java) for console use
pub fn find_java_console_bin(path: &Path) -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    let bin_name = "java.exe";
    #[cfg(not(target_os = "windows"))]
    let bin_name = "java";

    // 1. If path points directly to an executable file (e.g. /bin/javaw.exe)
    if path.is_file() {
        let sibling_console = path.with_file_name(bin_name);
        if sibling_console.exists() {
            return Some(sibling_console);
        }
        return Some(path.to_path_buf());
    }

    // 2. Direct check in directory (e.g. dir/java.exe or dir/bin/java.exe)
    let direct = path.join(bin_name);
    if direct.exists() {
        return Some(direct);
    }

    let bin_folder = path.join("bin").join(bin_name);
    if bin_folder.exists() {
        return Some(bin_folder);
    }

    // 3. Check nested directory (e.g. jdk-25.0.4.1+1/bin/java.exe)
    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                let nested = p.join("bin").join(bin_name);
                if nested.exists() {
                    return Some(nested);
                }
                let nested_direct = p.join(bin_name);
                if nested_direct.exists() {
                    return Some(nested_direct);
                }
            }
        }
    }

    None
}

/// Turns sandbox setup progress into startup steps: each distinct message once, plus
/// every 10% of a download
pub fn sandbox_progress<F>(step: &F) -> impl Fn(&str, f32) + Send + Sync + 'static
where
    F: Fn(&str) + Clone + Send + Sync + 'static,
{
    let last_reported = std::sync::Mutex::new((String::new(), 0u32));
    let step = step.clone();
    move |msg, prog| {
        let pct = (prog * 100.0) as u32;
        let Ok(mut last) = last_reported.lock() else {
            return;
        };
        if last.0 != msg || pct >= last.1 + 10 {
            *last = (msg.to_string(), pct);
            step(&format!("{msg} ({pct}%)"));
        }
    }
}

/// Extracts a player username appearing before a given keyword (e.g. " joined the game")
pub fn extract_player_keyword(line: &str, keyword: &str) -> Option<String> {
    let idx = line.find(keyword)?;
    let before = line[..idx].trim();
    before.split_whitespace().last().map(|s| s.to_string())
}

/// Parses Bedrock player connect lines: "Player connected: PlayerName, xuid: 123456"
pub fn parse_bedrock_connected(line: &str) -> Option<(String, Option<String>)> {
    let rest = line.split("Player connected: ").nth(1)?;
    let name = rest.split(',').next().unwrap_or(rest).trim().to_string();
    if name.is_empty() {
        return None;
    }
    let xuid = rest.split("xuid:").nth(1).and_then(|xuid_str| {
        let trimmed = xuid_str.split(',').next().unwrap_or(xuid_str).trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    });
    Some((name, xuid))
}

/// Parses Bedrock player disconnect lines: "Player disconnected: PlayerName, xuid: 123456"
pub fn parse_bedrock_disconnected(line: &str) -> Option<String> {
    let rest = line.split("Player disconnected: ").nth(1)?;
    let name = rest.split(',').next().unwrap_or(rest).trim().to_string();
    if name.is_empty() {
        None
    } else {
        Some(name)
    }
}
