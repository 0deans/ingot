//! Command-line argument parsing and action dispatch for OS shortcuts and external calls.

#[cfg(not(desktop))]
use tauri::Manager;
use tauri::{AppHandle, Runtime};

fn sanitize_instance_id(target: &str) -> Option<String> {
    let trimmed = target
        .trim()
        .trim_matches(|c| c == '"' || c == '\'' || c == '\\')
        .trim();
    if !trimmed.is_empty() {
        Some(trimmed.to_string())
    } else {
        None
    }
}

/// Parses `--launch-instance <ID>`, `-l <ID>`, `--launch-instance=<ID>`, or `-l=<ID>`.
pub fn parse_cli_launch_instance(args: &[String]) -> Option<String> {
    let mut iter = args.iter().peekable();
    while let Some(arg) = iter.next() {
        if arg == "--launch-instance" || arg == "-l" {
            if let Some(target) = iter.next() {
                if let Some(id) = sanitize_instance_id(target) {
                    return Some(id);
                }
            }
        } else if let Some(target) = arg.strip_prefix("--launch-instance=") {
            if let Some(id) = sanitize_instance_id(target) {
                return Some(id);
            }
        } else if let Some(target) = arg.strip_prefix("-l=") {
            if let Some(id) = sanitize_instance_id(target) {
                return Some(id);
            }
        }
    }
    None
}

/// Dispatches actions specified by command line arguments.
pub fn handle_cli_args<R: Runtime>(app: &AppHandle<R>, args: &[String]) {
    if let Some(instance_id) = parse_cli_launch_instance(args) {
        let handle = app.clone();
        tauri::async_runtime::spawn(async move {
            if let Err(err) = crate::app::games::launch_instance_with_events(
                handle.clone(),
                instance_id.clone(),
                None,
            )
            .await
            {
                eprintln!("[CLI] Failed to launch instance {instance_id}: {err}");
                #[cfg(desktop)]
                crate::tray::restore_main_window(&handle);
                #[cfg(not(desktop))]
                if let Some(window) = handle.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_separate_flag_and_id() {
        let args = vec![
            "ingot".to_string(),
            "--launch-instance".to_string(),
            "inst-123".to_string(),
        ];
        assert_eq!(
            parse_cli_launch_instance(&args),
            Some("inst-123".to_string())
        );

        let short_args = vec![
            "ingot".to_string(),
            "-l".to_string(),
            "inst-456".to_string(),
        ];
        assert_eq!(
            parse_cli_launch_instance(&short_args),
            Some("inst-456".to_string())
        );
    }

    #[test]
    fn parses_equals_syntax() {
        let args = vec![
            "ingot".to_string(),
            "--launch-instance=inst-789".to_string(),
        ];
        assert_eq!(
            parse_cli_launch_instance(&args),
            Some("inst-789".to_string())
        );

        let short_args = vec!["ingot".to_string(), "-l=inst-abc".to_string()];
        assert_eq!(
            parse_cli_launch_instance(&short_args),
            Some("inst-abc".to_string())
        );
    }

    #[test]
    fn handles_empty_or_missing_id() {
        let missing = vec!["ingot".to_string(), "--launch-instance".to_string()];
        assert_eq!(parse_cli_launch_instance(&missing), None);

        let empty = vec![
            "ingot".to_string(),
            "--launch-instance".to_string(),
            "   ".to_string(),
        ];
        assert_eq!(parse_cli_launch_instance(&empty), None);

        let empty_eq = vec!["ingot".to_string(), "--launch-instance=".to_string()];
        assert_eq!(parse_cli_launch_instance(&empty_eq), None);

        let empty_quotes = vec![
            "ingot".to_string(),
            "--launch-instance".to_string(),
            "\"\"".to_string(),
        ];
        assert_eq!(parse_cli_launch_instance(&empty_quotes), None);

        let empty_quoted_spaces =
            vec!["ingot".to_string(), "--launch-instance=\"   \"".to_string()];
        assert_eq!(parse_cli_launch_instance(&empty_quoted_spaces), None);
    }

    #[test]
    fn parses_quoted_or_escaped_instance_ids() {
        let double_quoted = vec![
            "ingot".to_string(),
            "--launch-instance".to_string(),
            "\"inst-quotes-1\"".to_string(),
        ];
        assert_eq!(
            parse_cli_launch_instance(&double_quoted),
            Some("inst-quotes-1".to_string())
        );

        let single_quoted = vec![
            "ingot".to_string(),
            "-l".to_string(),
            "'inst-quotes-2'".to_string(),
        ];
        assert_eq!(
            parse_cli_launch_instance(&single_quoted),
            Some("inst-quotes-2".to_string())
        );

        let escaped_quotes = vec![
            "ingot".to_string(),
            "--launch-instance".to_string(),
            "\\\"inst-quotes-3\\\"".to_string(),
        ];
        assert_eq!(
            parse_cli_launch_instance(&escaped_quotes),
            Some("inst-quotes-3".to_string())
        );

        let equals_quoted = vec![
            "ingot".to_string(),
            "--launch-instance=\"inst-quotes-4\"".to_string(),
        ];
        assert_eq!(
            parse_cli_launch_instance(&equals_quoted),
            Some("inst-quotes-4".to_string())
        );
    }

    #[test]
    fn ignores_unrelated_flags() {
        let args = vec![
            "ingot".to_string(),
            "--some-other-flag".to_string(),
            "value".to_string(),
        ];
        assert_eq!(parse_cli_launch_instance(&args), None);
    }
}
