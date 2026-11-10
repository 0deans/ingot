use super::models::{InstanceLogLine, LogLevel};

pub fn strip_ansi_and_cr(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            if chars.peek() == Some(&'[') {
                chars.next();
                for esc in chars.by_ref() {
                    if esc.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
        } else if c != '\r' {
            out.push(c);
        }
    }
    out
}

pub fn parse_log_line(
    seq: u64,
    raw_input: &str,
    default_level: LogLevel,
    previous_level: Option<LogLevel>,
) -> InstanceLogLine {
    let cleaned = strip_ansi_and_cr(raw_input);

    if let Some(rest) = cleaned.strip_prefix("[Ingot]") {
        let trimmed = rest.trim();
        let level = if trimmed.starts_with("Failed") || trimmed.starts_with("Error") {
            LogLevel::Error
        } else {
            LogLevel::Launcher
        };
        return InstanceLogLine {
            seq,
            timestamp: None,
            thread: None,
            source: Some("Ingot".to_string()),
            level,
            message: trimmed.to_string(),
            raw: cleaned,
        };
    }

    if let Some(parsed) = parse_bracketed_line(seq, &cleaned) {
        return parsed;
    }

    if is_stack_continuation(&cleaned) {
        let inherited = match previous_level {
            Some(LogLevel::Fatal) => LogLevel::Fatal,
            Some(LogLevel::Error) => LogLevel::Error,
            Some(LogLevel::Warn) => LogLevel::Warn,
            _ => LogLevel::Error,
        };
        return InstanceLogLine {
            seq,
            timestamp: None,
            thread: None,
            source: None,
            level: inherited,
            message: cleaned.clone(),
            raw: cleaned,
        };
    }

    let inferred = infer_unstructured_level(&cleaned).unwrap_or(default_level);
    InstanceLogLine {
        seq,
        timestamp: None,
        thread: None,
        source: None,
        level: inferred,
        message: cleaned.clone(),
        raw: cleaned,
    }
}

pub fn parse_log_document(content: &str) -> Vec<InstanceLogLine> {
    let mut lines = Vec::new();
    let mut prev_level: Option<LogLevel> = None;

    for (idx, raw) in content.lines().enumerate() {
        let parsed = parse_log_line((idx + 1) as u64, raw, LogLevel::Info, prev_level);
        prev_level = Some(parsed.level);
        lines.push(parsed);
    }

    lines
}

fn parse_bracketed_line(seq: u64, line: &str) -> Option<InstanceLogLine> {
    let rest = line.strip_prefix('[')?;
    let (first_bracket, after_first) = rest.split_once(']')?;

    if looks_like_timestamp(first_bracket) {
        let timestamp = Some(first_bracket.trim().to_string());
        let after_first = after_first.trim_start();
        let second_rest = after_first.strip_prefix('[')?;
        let (second_bracket, after_second) = second_rest.split_once(']')?;

        let (thread, level, inline_source) = parse_thread_and_level(second_bracket)?;

        let mut cursor = after_second.trim_start();
        let mut source = inline_source;

        if cursor.starts_with('[') {
            if let Some((third_bracket, after_third)) = cursor[1..].split_once(']') {
                let trimmed_source = third_bracket.trim().trim_end_matches('/');
                if !trimmed_source.is_empty() {
                    source = Some(trimmed_source.to_string());
                }
                cursor = after_third.trim_start();
            }
        }

        if let Some(stripped) = cursor.strip_prefix(':') {
            cursor = stripped.strip_prefix(' ').unwrap_or(stripped);
        }

        Some(InstanceLogLine {
            seq,
            timestamp,
            thread,
            source,
            level,
            message: cursor.to_string(),
            raw: line.to_string(),
        })
    } else {
        let (thread, level, inline_source) = parse_thread_and_level(first_bracket)?;

        let mut cursor = after_first.trim_start();
        let mut source = inline_source;

        if cursor.starts_with('[') {
            if let Some((next_bracket, after_next)) = cursor[1..].split_once(']') {
                let trimmed_source = next_bracket.trim().trim_end_matches('/');
                if !trimmed_source.is_empty() {
                    source = Some(trimmed_source.to_string());
                }
                cursor = after_next.trim_start();
            }
        }

        if let Some(stripped) = cursor.strip_prefix(':') {
            cursor = stripped.strip_prefix(' ').unwrap_or(stripped);
        }

        Some(InstanceLogLine {
            seq,
            timestamp: None,
            thread,
            source,
            level,
            message: cursor.to_string(),
            raw: line.to_string(),
        })
    }
}

fn looks_like_timestamp(text: &str) -> bool {
    let trimmed = text.trim();
    (trimmed.len() >= 7 && trimmed.len() <= 24)
        && trimmed.contains(':')
        && trimmed.chars().next().is_some_and(|ch| ch.is_ascii_digit())
}

fn parse_thread_and_level(bracket: &str) -> Option<(Option<String>, LogLevel, Option<String>)> {
    if let Some((left, right)) = bracket.rsplit_once('/') {
        let parts: Vec<&str> = bracket.split('/').collect();
        if parts.len() == 2 {
            let level = parse_level_token(right)?;
            let thread = non_empty(left);
            return Some((thread, level, None));
        }
        if parts.len() >= 3 {
            if let Some(level) = parse_level_token(parts[1]) {
                let thread = non_empty(parts[0]);
                let source = non_empty(&parts[2..].join("/"));
                return Some((thread, level, source));
            }
            if let Some(level) = parse_level_token(right) {
                let thread = non_empty(left);
                return Some((thread, level, None));
            }
        }
    }
    let level = parse_level_token(bracket)?;
    Some((None, level, None))
}

fn non_empty(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn parse_level_token(token: &str) -> Option<LogLevel> {
    match token.trim().to_ascii_uppercase().as_str() {
        "FATAL" | "SEVERE" => Some(LogLevel::Fatal),
        "ERROR" | "ERR" | "STDERR" => Some(LogLevel::Error),
        "WARN" | "WARNING" => Some(LogLevel::Warn),
        "INFO" | "STDOUT" => Some(LogLevel::Info),
        "DEBUG" | "TRACE" | "FINE" | "FINER" | "FINEST" => Some(LogLevel::Debug),
        _ => None,
    }
}

fn is_stack_continuation(line: &str) -> bool {
    let trimmed = line.trim_start();
    if (line.starts_with('\t') || line.starts_with("    ") || line.starts_with("  at "))
        && (trimmed.starts_with("at ") || trimmed.starts_with("... "))
    {
        return true;
    }
    trimmed.starts_with("Caused by:")
        || trimmed.starts_with("Suppressed:")
        || (trimmed.starts_with("... ") && trimmed.ends_with(" more"))
}

fn infer_unstructured_level(line: &str) -> Option<LogLevel> {
    if line.contains("---- Minecraft Crash Report ----")
        || line.contains("A fatal error has been detected by the Java Runtime Environment")
        || line.contains("FATAL]")
        || line.contains("FATAL:")
        || line.contains(" FATAL ")
    {
        return Some(LogLevel::Fatal);
    }
    if line.contains("ERROR]")
        || line.contains("ERROR:")
        || line.contains(" ERROR ")
        || line.contains("Exception in thread")
        || line.contains("Exception:")
        || line.contains("Error:")
    {
        return Some(LogLevel::Error);
    }
    if line.contains("WARN]")
        || line.contains("WARN:")
        || line.contains("WARNING:")
        || line.contains(" WARN ")
        || line.contains(" WARNING ")
    {
        return Some(LogLevel::Warn);
    }
    if line.contains("INFO]")
        || line.contains("INFO:")
        || line.contains("/INFO")
        || line.contains(" INFO ")
    {
        return Some(LogLevel::Info);
    }
    if line.contains("DEBUG]")
        || line.contains("DEBUG:")
        || line.contains("/DEBUG")
        || line.contains(" DEBUG ")
        || line.contains("TRACE]")
        || line.contains(" TRACE ")
    {
        return Some(LogLevel::Debug);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_standard_vanilla_and_fabric_log_lines() {
        let raw = "[19:44:14] [Render thread/INFO]: Setting user: PlayerOne";
        let line = parse_log_line(1, raw, LogLevel::Info, None);
        assert_eq!(line.seq, 1);
        assert_eq!(line.timestamp.as_deref(), Some("19:44:14"));
        assert_eq!(line.thread.as_deref(), Some("Render thread"));
        assert_eq!(line.source, None);
        assert_eq!(line.level, LogLevel::Info);
        assert_eq!(line.message, "Setting user: PlayerOne");
    }

    #[test]
    fn parses_forge_modlauncher_line_with_logger_source() {
        let raw =
            "[19:44:14] [main/WARN] [mixin/]: Reference map 'create.refmap.json' could not be read";
        let line = parse_log_line(2, raw, LogLevel::Info, None);
        assert_eq!(line.timestamp.as_deref(), Some("19:44:14"));
        assert_eq!(line.thread.as_deref(), Some("main"));
        assert_eq!(line.source.as_deref(), Some("mixin"));
        assert_eq!(line.level, LogLevel::Warn);
        assert_eq!(
            line.message,
            "Reference map 'create.refmap.json' could not be read"
        );
    }

    #[test]
    fn stack_trace_lines_inherit_error_or_fatal_severity() {
        let doc = "\
[19:44:14] [main/ERROR]: Uncaught exception in thread \"main\"
java.lang.RuntimeException: Mixin transformation failed
\tat net.fabricmc.loader.impl.launch.knot.Knot.init(Knot.java:160)
Caused by: org.spongepowered.asm.mixin.throwables.MixinApplyError: Mixin failed
\t... 12 more";
        let parsed = parse_log_document(doc);
        assert_eq!(parsed.len(), 5);
        for line in parsed {
            assert_eq!(line.level, LogLevel::Error);
        }
    }

    #[test]
    fn strips_ansi_escape_sequences_and_carriage_returns() {
        let raw = "\x1b[31m[19:44:14] [main/FATAL]: Crash!\x1b[0m\r";
        let line = parse_log_line(1, raw, LogLevel::Info, None);
        assert_eq!(line.level, LogLevel::Fatal);
        assert_eq!(line.message, "Crash!");
        assert!(!line.raw.contains('\x1b'));
        assert!(!line.raw.contains('\r'));
    }

    #[test]
    fn classifies_ingot_launcher_messages() {
        let info = parse_log_line(
            1,
            "[Ingot] Launching Minecraft 1.21.4...",
            LogLevel::Info,
            None,
        );
        assert_eq!(info.level, LogLevel::Launcher);
        assert_eq!(info.source.as_deref(), Some("Ingot"));
        assert_eq!(info.message, "Launching Minecraft 1.21.4...");

        let err = parse_log_line(2, "[Ingot] Failed to launch process", LogLevel::Info, None);
        assert_eq!(err.level, LogLevel::Error);
    }

    #[test]
    fn parses_console_lines_without_timestamp_brackets() {
        let raw = "[Render thread/INFO]: Backend library: LWJGL version 3.3.3";
        let line = parse_log_line(1, raw, LogLevel::Info, None);
        assert_eq!(line.timestamp, None);
        assert_eq!(line.thread.as_deref(), Some("Render thread"));
        assert_eq!(line.level, LogLevel::Info);
        assert_eq!(line.message, "Backend library: LWJGL version 3.3.3");

        let forge_raw =
            "[main/INFO] [cpw.mods.modlauncher.Launcher/MODLAUNCHER]: ModLauncher starting";
        let forge_line = parse_log_line(2, forge_raw, LogLevel::Info, None);
        assert_eq!(forge_line.timestamp, None);
        assert_eq!(forge_line.thread.as_deref(), Some("main"));
        assert_eq!(
            forge_line.source.as_deref(),
            Some("cpw.mods.modlauncher.Launcher/MODLAUNCHER")
        );
        assert_eq!(forge_line.level, LogLevel::Info);
        assert_eq!(forge_line.message, "ModLauncher starting");

        let simple_raw = "[INFO] Simple log message";
        let simple_line = parse_log_line(3, simple_raw, LogLevel::Info, None);
        assert_eq!(simple_line.level, LogLevel::Info);
        assert_eq!(simple_line.message, "Simple log message");
    }

    #[test]
    fn parses_single_digit_hour_timestamps_and_unbracketed_info() {
        let single_digit = "[9:15:00] [Render thread/INFO]: Game starting";
        let parsed = parse_log_line(1, single_digit, LogLevel::Info, None);
        assert_eq!(parsed.timestamp.as_deref(), Some("9:15:00"));
        assert_eq!(parsed.level, LogLevel::Info);

        let unbracketed =
            "2026-11-03 20:15:30.123 [main] INFO net.fabricmc.loader.impl.launch.knot.Knot - Loading Minecraft";
        let unbracketed_line = parse_log_line(2, unbracketed, LogLevel::Info, None);
        assert_eq!(unbracketed_line.level, LogLevel::Info);
    }
}
