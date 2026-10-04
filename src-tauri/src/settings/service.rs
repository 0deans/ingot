use super::{
    repository::SettingsRepository, LauncherBehavior, MemorySettings, SettingsError, SyncSettings,
    WindowSettings,
};

pub(crate) fn set_memory(
    repository: &SettingsRepository,
    min_mb: u32,
    max_mb: u32,
) -> Result<MemorySettings, SettingsError> {
    let memory = MemorySettings::normalized(min_mb, max_mb)?;
    repository.update(|settings| {
        settings.memory = memory.clone();
        Ok(memory)
    })
}

pub(crate) fn set_behavior(
    repository: &SettingsRepository,
    behavior: &str,
) -> Result<LauncherBehavior, SettingsError> {
    let behavior = LauncherBehavior::try_from(behavior)?;
    repository.update(|settings| {
        settings.launcher_behavior = behavior;
        Ok(behavior)
    })
}

pub(crate) fn set_window(
    repository: &SettingsRepository,
    window: WindowSettings,
) -> Result<WindowSettings, SettingsError> {
    window.validate()?;
    repository.update(|settings| {
        settings.window = window.clone();
        Ok(window)
    })
}

pub(crate) fn set_sync(
    repository: &SettingsRepository,
    sync: SyncSettings,
) -> Result<SyncSettings, SettingsError> {
    repository.update(|settings| {
        settings.sync = sync.clone();
        Ok(sync)
    })
}
