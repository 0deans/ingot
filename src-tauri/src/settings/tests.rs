use std::{
    fs,
    sync::{Arc, Barrier},
};

use super::{repository::SettingsRepository, service, *};

#[test]
fn missing_settings_use_defaults_without_creating_directories() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("missing/settings.json");
    let repository = SettingsRepository::new(path.clone());
    assert_eq!(repository.load().unwrap(), LauncherSettings::default());
    assert!(!path.parent().unwrap().exists());
}

#[test]
fn legacy_settings_fill_only_new_fields_with_defaults() {
    let legacy: LauncherSettings =
        serde_json::from_str(include_str!("../../tests/fixtures/settings/legacy.json")).unwrap();
    assert_eq!(legacy.memory.min_ram_mb, 1024);
    assert_eq!(legacy.memory.max_ram_mb, 3072);
    assert_eq!(legacy.launcher_behavior, LauncherBehavior::KeepOpen);
    assert_eq!(legacy.window, WindowSettings::default());
    assert_eq!(legacy.sync, SyncSettings::default());
}

#[test]
fn current_settings_keep_the_existing_json_contract() {
    let fixture = include_str!("../../tests/fixtures/settings/current.json");
    let settings: LauncherSettings = serde_json::from_str(fixture).unwrap();
    settings.validate().unwrap();
    assert_eq!(settings.launcher_behavior, LauncherBehavior::HideToTray);
    assert_eq!(
        serde_json::to_value(settings).unwrap(),
        serde_json::from_str::<serde_json::Value>(fixture).unwrap()
    );
    for (behavior, spelling) in [
        (LauncherBehavior::KeepOpen, "keepOpen"),
        (LauncherBehavior::HideToTray, "hideToTray"),
        (LauncherBehavior::Close, "close"),
    ] {
        assert_eq!(serde_json::to_value(behavior).unwrap(), spelling);
    }
}

#[test]
fn malformed_settings_fail_and_are_not_overwritten() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("settings.json");
    fs::write(&path, b"{broken").unwrap();
    let repository = SettingsRepository::new(path.clone());
    assert!(matches!(
        repository.load(),
        Err(SettingsError::Decode { .. })
    ));
    assert!(matches!(
        service::set_memory(&repository, 1024, 4096),
        Err(SettingsError::Decode { .. })
    ));
    assert_eq!(fs::read(&path).unwrap(), b"{broken");
}

#[test]
fn unreadable_settings_are_not_treated_as_missing() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("settings.json");
    fs::create_dir(&path).unwrap();
    let repository = SettingsRepository::new(path);
    assert!(matches!(
        repository.load(),
        Err(SettingsError::Io {
            operation: "read",
            ..
        })
    ));
}

#[test]
fn updates_replace_existing_files_and_preserve_other_preferences() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("new/settings.json");
    let repository = SettingsRepository::new(path);
    service::set_behavior(&repository, "hideToTray").unwrap();
    service::set_window(
        &repository,
        WindowSettings {
            fullscreen: true,
            width: 1920,
            height: 1080,
        },
    )
    .unwrap();
    let memory = service::set_memory(&repository, 8192, 1024).unwrap();
    assert_eq!((memory.min_ram_mb, memory.max_ram_mb), (1024, 8192));
    let settings = repository.load().unwrap();
    assert_eq!(settings.memory, memory);
    assert_eq!(settings.launcher_behavior, LauncherBehavior::HideToTray);
    assert!(settings.window.fullscreen);
    assert_eq!(settings.window.width, 1920);
}

#[test]
fn invalid_updates_leave_previous_settings_unchanged() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("settings.json");
    let repository = SettingsRepository::new(path.clone());
    service::set_behavior(&repository, "keepOpen").unwrap();
    let before = fs::read(&path).unwrap();
    assert!(matches!(
        service::set_memory(&repository, 0, 4096),
        Err(SettingsError::InvalidMemory { .. })
    ));
    assert!(matches!(
        service::set_behavior(&repository, "typo"),
        Err(SettingsError::InvalidBehavior(_))
    ));
    assert!(matches!(
        service::set_window(
            &repository,
            WindowSettings {
                width: 0,
                ..WindowSettings::default()
            }
        ),
        Err(SettingsError::InvalidWindow { .. })
    ));
    assert_eq!(fs::read(&path).unwrap(), before);
}

#[test]
fn invalid_persisted_values_are_rejected_without_normalizing_the_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("settings.json");
    let invalid = br#"{"memory":{"minRamMb":4096,"maxRamMb":1024}}"#;
    fs::write(&path, invalid).unwrap();
    let repository = SettingsRepository::new(path.clone());
    assert!(matches!(
        repository.load(),
        Err(SettingsError::InvalidMemory { .. })
    ));
    assert_eq!(fs::read(path).unwrap(), invalid);
}

#[test]
fn concurrent_updates_do_not_lose_unrelated_fields() {
    let dir = tempfile::tempdir().unwrap();
    let repository = Arc::new(SettingsRepository::new(dir.path().join("settings.json")));
    let barrier = Arc::new(Barrier::new(3));
    std::thread::scope(|scope| {
        let first_repository = repository.clone();
        let first_barrier = barrier.clone();
        scope.spawn(move || {
            first_barrier.wait();
            service::set_memory(&first_repository, 1024, 8192).unwrap();
        });
        let second_repository = repository.clone();
        let second_barrier = barrier.clone();
        scope.spawn(move || {
            second_barrier.wait();
            service::set_window(
                &second_repository,
                WindowSettings {
                    fullscreen: true,
                    width: 1280,
                    height: 720,
                },
            )
            .unwrap();
        });
        barrier.wait();
    });
    let settings = repository.load().unwrap();
    assert_eq!(settings.memory.max_ram_mb, 8192);
    assert!(settings.window.fullscreen);
    assert_eq!(settings.window.width, 1280);
}

#[test]
fn sync_updates_preserve_category_history_and_other_settings() {
    let dir = tempfile::tempdir().unwrap();
    let repository = SettingsRepository::new(dir.path().join("settings.json"));
    service::set_behavior(&repository, "close").unwrap();
    let sync = SyncSettings {
        sync_options: true,
        initialized_categories: vec!["options".into()],
        ..SyncSettings::default()
    };
    service::set_sync(&repository, sync.clone()).unwrap();
    let settings = repository.load().unwrap();
    assert_eq!(settings.sync, sync);
    assert_eq!(settings.launcher_behavior, LauncherBehavior::Close);
}
