use super::command::{build_aikar_gc_flags, build_java_version_flags};
use super::console::{
    clean_console_line, extract_player_keyword, find_java_console_bin, parse_bedrock_connected,
    parse_bedrock_disconnected,
};
use super::manager::ServerProcessManager;
use super::models::StartingGuard;
use crate::server::config::ServerStatus;
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

#[test]
fn test_clean_console_line_ansi_and_prompts() {
    let raw = "\u{1b}[32m[Server thread/INFO]: Done (2.5s)!\u{1b}[0m\r";
    let cleaned = clean_console_line(raw);
    assert_eq!(cleaned, "[Server thread/INFO]: Done (2.5s)!");

    let prompt_raw = "> > help";
    assert_eq!(clean_console_line(prompt_raw), "help");

    let mixed = "\u{1b}[0;31m> Error occurred!\u{1b}[0m\r\n";
    assert_eq!(clean_console_line(mixed), "Error occurred!");
}

#[test]
fn test_find_java_console_bin() {
    let tmp = tempfile::tempdir().unwrap();
    let bin_dir = tmp.path().join("bin");
    std::fs::create_dir_all(&bin_dir).unwrap();

    #[cfg(target_os = "windows")]
    let bin_name = "java.exe";
    #[cfg(not(target_os = "windows"))]
    let bin_name = "java";

    let java_file = bin_dir.join(bin_name);
    std::fs::write(&java_file, b"").unwrap();

    let found = find_java_console_bin(tmp.path());
    assert!(found.is_some());
    assert_eq!(found.unwrap(), java_file);
}

#[test]
fn test_build_aikar_and_java_flags() {
    let flags = build_aikar_gc_flags(2048, 4096);
    assert!(flags.contains(&"-Xms2048M".to_string()));
    assert!(flags.contains(&"-Xmx4096M".to_string()));
    assert!(flags.contains(&"-XX:+UseG1GC".to_string()));

    let j17 = build_java_version_flags(17);
    assert_eq!(j17, vec!["--enable-native-access=ALL-UNNAMED".to_string()]);

    let j24 = build_java_version_flags(24);
    assert!(j24.contains(&"--sun-misc-unsafe-memory-access=allow".to_string()));
    assert!(j24.contains(&"--enable-native-access=ALL-UNNAMED".to_string()));
}

#[tokio::test]
async fn test_process_manager_status_and_starting() {
    let pm = ServerProcessManager::new();
    assert_eq!(pm.get_server_status("srv-1").await, ServerStatus::Stopped);

    let guard = pm.try_claim_starting("srv-1");
    assert!(guard.is_ok());
    assert_eq!(pm.get_server_status("srv-1").await, ServerStatus::Starting);
    assert!(pm.is_starting("srv-1"));

    // Second claim must fail
    let second_claim = pm.try_claim_starting("srv-1");
    assert!(second_claim.is_err());

    // Dropping guard clears starting status
    drop(guard);
    assert!(!pm.is_starting("srv-1"));
    assert_eq!(pm.get_server_status("srv-1").await, ServerStatus::Stopped);
}

#[test]
fn test_starting_guard_drop_behavior() {
    let set = Arc::new(std::sync::Mutex::new(HashSet::new()));
    set.lock().unwrap().insert("test-srv".to_string());

    let guard = StartingGuard {
        set: set.clone(),
        server_id: "test-srv".to_string(),
    };
    assert!(set.lock().unwrap().contains("test-srv"));

    drop(guard);
    assert!(!set.lock().unwrap().contains("test-srv"));
}

#[test]
fn test_player_log_event_parsing() {
    let join_line = "[12:00:00 INFO]: Alex joined the game";
    let player = extract_player_keyword(join_line, " joined the game");
    assert_eq!(player, Some("Alex".to_string()));

    let leave_line = "[12:05:00 INFO]: Alex left the game";
    let player_leave = extract_player_keyword(leave_line, " left the game");
    assert_eq!(player_leave, Some("Alex".to_string()));

    let bedrock_join = "Player connected: Steve, xuid: 25354123456789";
    let bedrock_parsed = parse_bedrock_connected(bedrock_join);
    assert_eq!(
        bedrock_parsed,
        Some(("Steve".to_string(), Some("25354123456789".to_string())))
    );

    let bedrock_leave = "Player disconnected: Steve, xuid: 25354123456789";
    let bedrock_leave_parsed = parse_bedrock_disconnected(bedrock_leave);
    assert_eq!(bedrock_leave_parsed, Some("Steve".to_string()));
}

#[tokio::test]
async fn test_query_fails_when_not_running() {
    let pm = ServerProcessManager::new();
    let res = pm
        .query("unknown", "list", |_| true, Duration::from_millis(100))
        .await;
    assert!(res.is_err());
    assert_eq!(res.unwrap_err(), "Server is not running");
}

#[tokio::test]
async fn test_pending_query_guard_drop_decrements_immediately() {
    let pending = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    {
        let guard = super::models::PendingQueryGuard::new(pending.clone());
        assert_eq!(pending.load(std::sync::atomic::Ordering::SeqCst), 1);
        drop(guard);
    }
    assert_eq!(pending.load(std::sync::atomic::Ordering::SeqCst), 0);
}

#[tokio::test]
async fn test_pending_query_guard_complete_delayed() {
    let pending = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let guard = super::models::PendingQueryGuard::new(pending.clone());
    assert_eq!(pending.load(std::sync::atomic::Ordering::SeqCst), 1);

    guard.complete_delayed(Duration::from_millis(50));
    // Immediately after complete_delayed, still 1 due to delay
    assert_eq!(pending.load(std::sync::atomic::Ordering::SeqCst), 1);

    tokio::time::sleep(Duration::from_millis(80)).await;
    // After delay, background task has decremented it
    assert_eq!(pending.load(std::sync::atomic::Ordering::SeqCst), 0);
}
