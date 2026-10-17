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
        .query_typed("unknown", "list", |_| true, Duration::from_millis(100))
        .await;
    assert!(matches!(
        res,
        Err(super::models::ProcessError::NotRunning(ref id)) if id == "unknown"
    ));
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

#[tokio::test(start_paused = true)]
async fn test_pending_query_guard_complete_delayed() {
    let pending = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let guard = super::models::PendingQueryGuard::new(pending.clone());
    assert_eq!(pending.load(std::sync::atomic::Ordering::SeqCst), 1);

    guard.complete_delayed(Duration::from_millis(50));
    // Immediately after complete_delayed, still 1 due to delay
    assert_eq!(pending.load(std::sync::atomic::Ordering::SeqCst), 1);

    tokio::task::yield_now().await;
    tokio::time::advance(Duration::from_millis(50)).await;
    tokio::task::yield_now().await;
    // After delay, background task has decremented it
    assert_eq!(pending.load(std::sync::atomic::Ordering::SeqCst), 0);
}

#[tokio::test]
async fn maintenance_excludes_server_start_and_releases_ownership() {
    let manager = ServerProcessManager::new();
    let maintenance = manager.reserve_maintenance("server").await.unwrap();
    assert!(manager.try_claim_starting("server").is_err());
    assert!(manager.reserve_maintenance("server").await.is_err());
    drop(maintenance);
    let starting = manager.try_claim_starting("server").unwrap();
    assert!(manager.reserve_maintenance("server").await.is_err());
    drop(starting);
    manager
        .set_server_status("server", ServerStatus::Sleeping)
        .await;
    assert!(manager.reserve_maintenance("server").await.is_err());
    assert!(!manager.is_starting("server"));
}

async fn server_fixture(pid: u32) -> super::models::ActiveServer {
    use super::models::ActiveServer;
    use std::sync::atomic::{AtomicBool, AtomicUsize};
    use tokio::sync::Mutex;
    let mut fixture = tokio::process::Command::new(std::env::current_exe().unwrap());
    fixture
        .arg("--list")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .kill_on_drop(true);
    let mut child = fixture.spawn().unwrap();
    let stdin = child.stdin.take().unwrap();
    child.wait().await.unwrap();
    let (line_tx, _) = tokio::sync::broadcast::channel(4);
    ActiveServer {
        server_id: "server".into(),
        core: crate::server::ServerCoreType::Paper,
        pid,
        port: 25565,
        started_at: 0,
        status: ServerStatus::Running,
        stdin: Arc::new(Mutex::new(stdin)),
        log_history: Arc::new(Mutex::new(Vec::new())),
        online_players: Arc::new(Mutex::new(Vec::new())),
        player_xuids: Arc::new(Mutex::new(std::collections::HashMap::new())),
        line_tx,
        query_lock: Arc::new(Mutex::new(())),
        pending_queries: Arc::new(AtomicUsize::new(0)),
        ready: Arc::new(AtomicBool::new(true)),
    }
}

#[tokio::test]
async fn stale_exit_does_not_remove_replacement_server() {
    let server = server_fixture(20).await;
    let manager = ServerProcessManager::new();
    manager.insert_server(server).await;
    assert!(!manager.remove_if_pid_matches("server", 10).await);
    assert!(manager.owns("server", 20).await);
    assert!(manager.remove_if_pid_matches("server", 20).await);
    assert_eq!(
        manager.get_server_status("server").await,
        ServerStatus::Stopped
    );
}

#[tokio::test]
async fn stale_stopped_observation_cannot_start_over_a_published_server() {
    let manager = ServerProcessManager::new();
    assert_eq!(
        manager.get_server_status("server").await,
        ServerStatus::Stopped
    );
    manager.insert_server(server_fixture(20).await).await;
    assert!(matches!(
        manager.claim_launch("server").await,
        Err(super::models::ProcessError::AlreadyRunning)
    ));
    assert!(!manager.is_starting("server"));
    assert!(manager.owns("server", 20).await);
}

#[tokio::test]
async fn server_claim_reads_latest_metadata_and_releases_missing_target() {
    use crate::minecraft::metadata::MetadataRepository;
    let directory = tempfile::tempdir().unwrap();
    let repository = MetadataRepository::new(directory.path().join("servers.json"));
    let config: crate::server::config::ServerConfig = serde_json::from_value(serde_json::json!({
        "id": "server", "name": "Fixture", "core": "paper", "gameVersion": "1.20", "port": 25565,
        "createdAt": 0, "memoryMinMb": 1024, "memoryMaxMb": 2048
    }))
    .unwrap();
    repository
        .update(|records| {
            records.push(config);
            Ok(())
        })
        .unwrap();
    let manager = ServerProcessManager::new();
    let maintenance = manager.reserve_maintenance("server").await.unwrap();
    assert!(
        super::lifecycle::claim_server_from_repository(&repository, &manager, "server")
            .await
            .is_err()
    );
    repository
        .update(|records| {
            records[0].game_version = "1.21".into();
            Ok(())
        })
        .unwrap();
    drop(maintenance);
    let claimed = super::lifecycle::claim_server_from_repository(&repository, &manager, "server")
        .await
        .unwrap();
    assert_eq!(claimed.config.game_version, "1.21");
    drop(claimed);
    repository
        .update(|records| {
            records.clear();
            Ok(())
        })
        .unwrap();
    assert!(
        super::lifecycle::claim_server_from_repository(&repository, &manager, "server")
            .await
            .is_err()
    );
    assert!(!manager.is_starting("server"));
}

#[tokio::test(start_paused = true)]
async fn stop_timeout_does_not_kill_a_replacement_process() {
    use std::future::{poll_fn, Future};
    use std::task::Poll;
    let manager = ServerProcessManager::new();
    manager.insert_server(server_fixture(10).await).await;
    let killed = std::sync::atomic::AtomicU32::new(0);
    let mut timeout = Box::pin(super::manager::force_stop_after_timeout(
        &manager.servers,
        "server",
        10,
        Duration::from_secs(1),
        |pid| {
            killed.store(pid, std::sync::atomic::Ordering::SeqCst);
        },
    ));
    assert!(poll_fn(|cx| Poll::Ready(timeout.as_mut().poll(cx).is_pending())).await);
    manager.servers.lock().await.get_mut("server").unwrap().pid = 20;
    tokio::time::advance(Duration::from_secs(1)).await;
    timeout.await;
    assert_eq!(killed.load(std::sync::atomic::Ordering::SeqCst), 0);
    assert!(manager.owns("server", 20).await);
}

#[tokio::test(start_paused = true)]
async fn stop_timeout_kills_only_the_original_process_at_deadline() {
    use std::future::{poll_fn, Future};
    use std::task::Poll;
    let manager = ServerProcessManager::new();
    manager.insert_server(server_fixture(10).await).await;
    let killed = std::sync::atomic::AtomicU32::new(0);
    let mut timeout = Box::pin(super::manager::force_stop_after_timeout(
        &manager.servers,
        "server",
        10,
        Duration::from_secs(1),
        |pid| {
            killed.store(pid, std::sync::atomic::Ordering::SeqCst);
        },
    ));
    assert!(poll_fn(|cx| Poll::Ready(timeout.as_mut().poll(cx).is_pending())).await);
    tokio::time::advance(Duration::from_millis(999)).await;
    assert!(poll_fn(|cx| Poll::Ready(timeout.as_mut().poll(cx).is_pending())).await);
    assert_eq!(killed.load(std::sync::atomic::Ordering::SeqCst), 0);
    tokio::time::advance(Duration::from_millis(1)).await;
    timeout.await;
    assert_eq!(killed.load(std::sync::atomic::Ordering::SeqCst), 10);
}
