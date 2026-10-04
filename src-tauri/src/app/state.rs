use crate::settings::repository::SettingsRepository;

use super::AppPaths;

pub(crate) struct AppState {
    pub(crate) settings: SettingsRepository,
}

impl AppState {
    pub(crate) fn new(paths: AppPaths) -> Self {
        Self {
            settings: SettingsRepository::new(paths.settings_file()),
        }
    }
}
