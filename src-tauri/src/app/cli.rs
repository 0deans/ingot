//! Command-line argument parsing and action dispatch for OS shortcuts and external calls.

use tauri::{AppHandle, Runtime};

/// Parses `--launch-instance <ID>`, `-l <ID>`, `--launch-instance=<ID>`, or `-l=<ID>`.
pub fn parse_cli_launch_instance(args: &[String]) -> Option<String> {
    let mut iter = args.iter().peekable();
    while let Some(arg) = iter.next() {
        if arg == "--launch-instance" || arg == "-l" {
            if let Some(target) = iter.next() {
                let trimmed = target.trim();
                if !trimmed.is_empty() {
                    return Some(trimmed.to_string());
                }
            }
        } else if let Some(target) = arg.strip_prefix("--launch-instance=") {
            let trimmed = target.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        } else if let Some(target) = arg.strip_prefix("-l=") {
            let trimmed = target.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
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
            if let Err(err) =
                crate::app::games::launch_instance_with_events(handle, instance_id.clone(), None)
                    .await
            {
                eprintln!("[CLI] Failed to launch instance {instance_id}: {err}");
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
