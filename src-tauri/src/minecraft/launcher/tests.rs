use super::arguments::{
    build_game_arguments, build_jvm_arguments, is_version_at_least, GameArgsConfig, JvmArgsConfig,
};
use super::models::QuickPlayOptions;
use super::process::ProcessManager;
use super::progress::ProgressThrottler;
use super::quick_play::{apply_quick_play_arguments, cleanup_quick_play_junction};
use crate::minecraft::version::{ArgumentValue, VersionArguments};
use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tempfile::TempDir;

#[test]
fn test_is_version_at_least() {
    assert!(is_version_at_least("1.20", 1, 20));
    assert!(is_version_at_least("1.20.1", 1, 20));
    assert!(is_version_at_least("1.21", 1, 20));
    assert!(is_version_at_least("2.0", 1, 20));
    assert!(!is_version_at_least("1.19.4", 1, 20));
    assert!(!is_version_at_least("1.16.5", 1, 20));

    // Snapshot versions check year prefix
    assert!(is_version_at_least("23w14a", 1, 20));
    assert!(is_version_at_least("24w05a", 1, 20));
    assert!(!is_version_at_least("22w42a", 1, 20));
}

#[tokio::test]
async fn test_process_manager_registry_and_tracking() {
    let pm = ProcessManager::new();
    assert!(!pm.is_running("inst-1").await);
    assert_eq!(pm.get_running_instances().await.len(), 0);

    pm.register("inst-1", 1234, 1000).await;
    assert!(pm.is_running("inst-1").await);

    let running = pm.get_running_instances().await;
    assert_eq!(running.len(), 1);
    assert_eq!(running[0].instance_id, "inst-1");
    assert_eq!(running[0].pid, 1234);
    assert_eq!(running[0].started_at, 1000);

    // Remove with mismatched pid returns false and keeps process
    let (removed, is_empty) = pm.remove_if_pid_matches("inst-1", 9999).await;
    assert!(!removed);
    assert!(!is_empty);
    assert!(pm.is_running("inst-1").await);

    // Remove with matching pid removes it
    let (removed, is_empty) = pm.remove_if_pid_matches("inst-1", 1234).await;
    assert!(removed);
    assert!(is_empty);
    assert!(!pm.is_running("inst-1").await);
}

#[tokio::test]
async fn test_process_manager_starting_reservation_and_commit() {
    let pm = ProcessManager::new();
    assert!(!pm.is_running("inst-1").await);
    assert!(!pm.is_starting("inst-1").await);

    // First reservation succeeds
    let guard = pm.reserve_starting("inst-1").await;
    assert!(guard.is_ok());
    let guard = guard.unwrap();

    assert!(pm.is_running("inst-1").await);
    assert!(pm.is_starting("inst-1").await);
    // Not running as an OS process yet
    assert_eq!(pm.get_running_instances().await.len(), 0);

    // Concurrent reservation is rejected
    let second = pm.reserve_starting("inst-1").await;
    assert!(matches!(
        second,
        Err(crate::minecraft::launcher::LaunchError::AlreadyRunning)
    ));

    // Commit transitions to Running
    assert!(guard.commit(4321, 2000).is_ok());
    assert!(pm.is_running("inst-1").await);
    assert!(!pm.is_starting("inst-1").await);
    let running = pm.get_running_instances().await;
    assert_eq!(running.len(), 1);
    assert_eq!(running[0].instance_id, "inst-1");
    assert_eq!(running[0].pid, 4321);
}

#[tokio::test]
async fn test_process_manager_starting_reservation_drop_releases() {
    let pm = ProcessManager::new();
    {
        let guard = pm.reserve_starting("inst-2").await.unwrap();
        assert!(pm.is_running("inst-2").await);
        assert!(pm.is_starting("inst-2").await);
        // Guard is dropped here without commit
        drop(guard);
    }

    assert!(!pm.is_running("inst-2").await);
    assert!(!pm.is_starting("inst-2").await);

    // Subsequent reservation succeeds after drop
    let guard2 = pm.reserve_starting("inst-2").await;
    assert!(guard2.is_ok());
}

#[tokio::test]
async fn test_process_manager_cancel_startup_invalidates_reservation_and_start_b() {
    let pm = ProcessManager::new();

    // Start A
    let guard_a = pm.reserve_starting("inst-1").await.unwrap();
    assert!(guard_a.check_active().is_ok());

    // Cancel A (kill_instance removes Starting entry)
    let kill_res = pm.kill_instance("inst-1").await;
    assert!(kill_res.is_ok());
    assert!(!pm.is_running("inst-1").await);
    assert!(guard_a.check_active().is_err());

    // Start B
    let guard_b = pm.reserve_starting("inst-1").await.unwrap();
    assert!(guard_b.check_active().is_ok());

    // Stale A attempts commit: fails
    let stale_commit = guard_a.commit(1111, 100);
    assert!(stale_commit.is_err());

    // Stale A was dropped or finished, but B must still be starting
    assert!(pm.is_starting("inst-1").await);
    assert!(guard_b.check_active().is_ok());

    // Guard B commits successfully
    assert!(guard_b.commit(2222, 200).is_ok());
    assert!(pm.is_running("inst-1").await);
    let running = pm.get_running_instances().await;
    assert_eq!(running.len(), 1);
    assert_eq!(running[0].pid, 2222);
}

#[tokio::test]
async fn test_process_manager_dropped_stale_guard_does_not_clear_newer_reservation() {
    let pm = ProcessManager::new();
    let guard_a = pm.reserve_starting("inst-1").await.unwrap();

    // Cancel A
    pm.kill_instance("inst-1").await.unwrap();

    // Start B
    let guard_b = pm.reserve_starting("inst-1").await.unwrap();

    // Drop A without committing
    drop(guard_a);

    // B is still starting and active
    assert!(pm.is_starting("inst-1").await);
    assert!(guard_b.check_active().is_ok());

    // Guard B commits
    guard_b.commit(3333, 300).unwrap();
    assert_eq!(pm.get_running_instances().await[0].pid, 3333);
}

#[test]
fn test_progress_throttler() {
    let throttler = ProgressThrottler::new(Duration::from_millis(100));

    // Initial boundary step 0 emits immediately
    assert!(throttler.should_emit("Phase 1", 0, 100));

    // Rapid successive step in the same phase within throttle duration is throttled
    assert!(!throttler.should_emit("Phase 1", 5, 100));

    // Phase change emits immediately
    assert!(throttler.should_emit("Phase 2", 5, 100));

    // Final boundary step emits immediately
    assert!(throttler.should_emit("Phase 2", 100, 100));
}

#[test]
fn test_build_jvm_arguments_fallback() {
    let natives_dir = Path::new("/test/natives");
    let libraries_dir = Path::new("/test/libraries");
    let custom_args = vec!["-Dcustom.flag=true".to_string()];

    let cfg = JvmArgsConfig {
        min_ram_mb: 1024,
        max_ram_mb: 4096,
        natives_dir,
        libraries_dir,
        classpath_str: "/test/cp.jar",
        cp_separator: ";",
        custom_jvm_args: Some(&custom_args),
        authlib_agent_path: None,
        version_arguments: None,
        main_class: "net.minecraft.client.main.Main",
    };

    let args = build_jvm_arguments(&cfg);

    assert!(args.contains(&"-Xms1024M".to_string()));
    assert!(args.contains(&"-Xmx4096M".to_string()));
    assert!(args.contains(&"-XX:+UseG1GC".to_string()));
    assert!(args.contains(&"-Dcustom.flag=true".to_string()));
    assert!(args.contains(&"-cp".to_string()));
    assert!(args.contains(&"/test/cp.jar".to_string()));
    assert_eq!(args.last().unwrap(), "net.minecraft.client.main.Main");
}

#[test]
fn test_build_game_arguments_legacy_format() {
    let instance_dir = Path::new("/test/instance");
    let assets_dir = Path::new("/test/assets");
    let legacy_template = "--username ${auth_player_name} --version ${version_name} --gameDir ${game_directory} --flag=${unresolved_param} ${unresolved_standalone}";

    let cfg = GameArgsConfig {
        player_name: "Steve",
        game_version: "1.12.2",
        instance_dir,
        assets_dir,
        asset_index_id: "1.12",
        uuid_str: "uuid-1234",
        access_token: "token-secret",
        user_type: "mojang",
        xuid: None,
        version_arguments: None,
        legacy_arguments: Some(legacy_template),
        is_fullscreen: false,
        window_width: 1920,
        window_height: 1080,
    };

    let args = build_game_arguments(&cfg);

    assert!(args.contains(&"--username".to_string()));
    assert!(args.contains(&"Steve".to_string()));
    assert!(args.contains(&"--version".to_string()));
    assert!(args.contains(&"1.12.2".to_string()));
    // Unresolved placeholders are filtered out
    assert!(!args.contains(&"--flag=${unresolved_param}".to_string()));
    assert!(!args.contains(&"${unresolved_standalone}".to_string()));
    assert!(!args.iter().any(|a| a.contains("${")));
    // Window settings
    assert!(args.contains(&"--width".to_string()));
    assert!(args.contains(&"1920".to_string()));
    assert!(args.contains(&"--height".to_string()));
    assert!(args.contains(&"1080".to_string()));
}

#[test]
fn test_build_game_arguments_modern_manifest_and_fullscreen() {
    let instance_dir = Path::new("/test/instance");
    let assets_dir = Path::new("/test/assets");

    let version_args = VersionArguments {
        game: Some(vec![
            ArgumentValue::Simple("--username".to_string()),
            ArgumentValue::Simple("${auth_player_name}".to_string()),
            ArgumentValue::Simple("--versionType".to_string()),
            ArgumentValue::Simple("${version_type}".to_string()),
        ]),
        jvm: None,
    };

    let cfg = GameArgsConfig {
        player_name: "Alex",
        game_version: "1.20.4",
        instance_dir,
        assets_dir,
        asset_index_id: "1.20",
        uuid_str: "uuid-alex",
        access_token: "token-alex",
        user_type: "msa",
        xuid: Some("12345678"),
        version_arguments: Some(&version_args),
        legacy_arguments: None,
        is_fullscreen: true,
        window_width: 854,
        window_height: 480,
    };

    let args = build_game_arguments(&cfg);

    assert!(args.contains(&"--username".to_string()));
    assert!(args.contains(&"Alex".to_string()));
    assert!(args.contains(&"--versionType".to_string()));
    assert!(args.contains(&"release".to_string()));
    assert!(args.contains(&"--fullscreen".to_string()));
    assert!(!args.contains(&"--width".to_string()));
}

#[test]
fn test_quick_play_arguments_multiplayer() {
    let instance_dir = Path::new("/test/instance");

    // Modern 1.20+
    let qp_modern = QuickPlayOptions {
        server: Some("hypixel.net".to_string()),
        world: None,
    };
    let mut args_modern = Vec::new();
    apply_quick_play_arguments(&mut args_modern, "1.20.2", instance_dir, Some(&qp_modern));
    assert_eq!(
        args_modern,
        vec![
            "--quickPlayMultiplayer".to_string(),
            "hypixel.net".to_string()
        ]
    );

    // Legacy with explicit port
    let qp_port = QuickPlayOptions {
        server: Some("local.host:25570".to_string()),
        world: None,
    };
    let mut args_legacy = Vec::new();
    apply_quick_play_arguments(&mut args_legacy, "1.19.2", instance_dir, Some(&qp_port));
    assert_eq!(
        args_legacy,
        vec![
            "--server".to_string(),
            "local.host".to_string(),
            "--port".to_string(),
            "25570".to_string()
        ]
    );
}

#[test]
fn test_quick_play_arguments_singleplayer_ascii() {
    let instance_dir = Path::new("/test/instance");

    let qp = QuickPlayOptions {
        server: None,
        world: Some("SurvivalWorld".to_string()),
    };
    let mut args = Vec::new();
    apply_quick_play_arguments(&mut args, "1.20.1", instance_dir, Some(&qp));
    assert_eq!(
        args,
        vec![
            "--quickPlaySingleplayer".to_string(),
            "SurvivalWorld".to_string()
        ]
    );
}

#[test]
fn test_quick_play_cleanup() {
    let temp = TempDir::new().unwrap();
    let saves = temp.path().join("saves");
    fs::create_dir_all(&saves).unwrap();
    let qp_junction = saves.join("qp_world");
    fs::create_dir(&qp_junction).unwrap();
    assert!(qp_junction.exists());

    cleanup_quick_play_junction(temp.path());
    assert!(!qp_junction.exists());
}

#[tokio::test]
async fn test_cancellation_between_validation_and_commit_terminates_and_reaps_child() {
    let pm = ProcessManager::new();
    let guard = pm.reserve_starting("inst-cancel-test").await.unwrap();

    // 1. Validation succeeds while guard is active
    assert!(guard.check_active().is_ok());

    // 2. Prepare command that would sleep for 30s if not reaped
    #[cfg(target_os = "windows")]
    let mut cmd = tokio::process::Command::new("powershell");
    #[cfg(target_os = "windows")]
    cmd.args(["-NoProfile", "-Command", "Start-Sleep -Seconds 30"]);
    #[cfg(not(target_os = "windows"))]
    let mut cmd = tokio::process::Command::new("sleep");
    #[cfg(not(target_os = "windows"))]
    cmd.arg("30");

    let pm_clone = pm.clone();
    let captured_pid = Arc::new(std::sync::atomic::AtomicU32::new(0));
    let captured_pid_clone = captured_pid.clone();

    // 3. Spawn process and cancel AFTER spawn but BEFORE commit
    let res = super::service::spawn_and_commit_process_with_hook(
        cmd,
        guard,
        1000,
        |spawned_pid| async move {
            captured_pid_clone.store(spawned_pid, std::sync::atomic::Ordering::SeqCst);
            // Cancel the reservation after child has been spawned
            let kill_res = pm_clone.kill_instance("inst-cancel-test").await;
            assert!(kill_res.is_ok());
        },
    )
    .await;

    // 4. Verification: commit failed due to invalidated reservation
    assert!(
        res.is_err(),
        "Expected commit failure due to invalidated state"
    );

    let spawned_pid = captured_pid.load(std::sync::atomic::Ordering::SeqCst);
    assert!(spawned_pid > 0, "Child process must have been spawned");

    // 5. Assert that the actual spawned child has exited and been reaped by production cleanup
    let mut exited = false;
    for _ in 0..50 {
        if crate::running::process_start(spawned_pid).is_none() {
            exited = true;
            break;
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    }
    assert!(
        exited,
        "Spawned child PID {spawned_pid} must exit after failed commit"
    );

    // 6. Verify the child process is not tracked as running
    assert!(!pm.is_running("inst-cancel-test").await);
}
