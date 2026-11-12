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

    pub(crate) fn instances_file(&self) -> PathBuf {
        self.data_dir.join("instances.json")
    }

    pub(crate) fn instances_dir(&self) -> PathBuf {
        self.data_dir.join("instances")
    }

    pub(crate) fn servers_file(&self) -> PathBuf {
        self.data_dir.join("servers.json")
    }

    pub(crate) fn servers_dir(&self) -> PathBuf {
        self.data_dir.join("servers")
    }

    pub(crate) fn accounts_file(&self) -> PathBuf {
        self.data_dir.join("accounts.json")
    }

    pub(crate) fn news_cache_dir(&self) -> PathBuf {
        self.data_dir.join("cache").join("news")
    }

    #[cfg(any(desktop, test))]
    pub(crate) fn cli_waiter_file(&self) -> PathBuf {
        self.data_dir.join("cli-waiter.json")
    }
}

#[cfg(any(desktop, test))]
pub(crate) fn default_cli_waiter_file() -> Option<PathBuf> {
    default_app_data_dir().map(|dir| dir.join("cli-waiter.json"))
}

#[cfg(any(desktop, test))]
pub(crate) fn default_app_data_dir() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        let app_data = std::env::var_os("APPDATA")?;
        if app_data.is_empty() {
            return None;
        }
        Some(PathBuf::from(app_data).join("ingot"))
    }
    #[cfg(target_os = "macos")]
    {
        let home = std::env::var_os("HOME")?;
        if home.is_empty() {
            return None;
        }
        Some(
            PathBuf::from(home)
                .join("Library")
                .join("Application Support")
                .join("ingot"),
        )
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        if let Some(xdg) = std::env::var_os("XDG_DATA_HOME") {
            if !xdg.is_empty() {
                return Some(PathBuf::from(xdg).join("ingot"));
            }
        }
        let home = std::env::var_os("HOME")?;
        if home.is_empty() {
            return None;
        }
        Some(
            PathBuf::from(home)
                .join(".local")
                .join("share")
                .join("ingot"),
        )
    }
}
