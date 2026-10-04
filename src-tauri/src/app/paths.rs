use std::path::PathBuf;

pub(crate) struct AppPaths {
    data_dir: PathBuf,
}

impl AppPaths {
    pub(crate) fn new(data_dir: PathBuf) -> Self {
        Self { data_dir }
    }

    pub(crate) fn settings_file(&self) -> PathBuf {
        self.data_dir.join("settings.json")
    }
}
