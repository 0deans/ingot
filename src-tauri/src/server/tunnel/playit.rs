pub use crate::server::config::PlayitTunnelStatus;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;
use tauri::{Manager, Runtime};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::Mutex;

#[derive(Clone, Default)]
pub struct PlayitManager {
    inner: Arc<Mutex<PlayitInner>>,
}

struct PlayitInner {
    status: PlayitTunnelStatus,
    child: Option<Arc<Mutex<Child>>>,
}

impl Default for PlayitInner {
    fn default() -> Self {
        Self {
            status: PlayitTunnelStatus {
                is_running: false,
                status: "stopped".to_string(),
                claim_url: None,
                public_address: None,
                ping_ms: None,
            },
            child: None,
        }
    }
}

impl PlayitManager {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(PlayitInner::default())),
        }
    }

    pub async fn get_status(&self) -> PlayitTunnelStatus {
        let guard = self.inner.lock().await;
        guard.status.clone()
    }

    /// Resolves or downloads the Playit CLI binary
    pub async fn ensure_playit_binary<R: Runtime>(
        &self,
        app: &tauri::AppHandle<R>,
        client: &reqwest::Client,
    ) -> Result<PathBuf, String> {
        // 1. Check if already installed in system PATH
        if let Ok(path) = std::env::var("PATH") {
            for dir in std::env::split_paths(&path) {
                let p = dir.join(if cfg!(windows) { "playit.exe" } else { "playit" });
                if p.exists() {
                    return Ok(p);
                }
            }
        }

        // 2. Check Android native library directory
        if let Ok(lib_dir) = std::env::var("ANDROID_APP_LIB_DIR") {
            let p = PathBuf::from(lib_dir).join("libplayit.so");
            if p.exists() {
                return Ok(p);
            }
        }

        // 3. Check app data tools directory
        let data_dir = app
            .path()
            .app_data_dir()
            .map_err(|e| format!("Failed to get app data directory: {e}"))?;
        let tools_dir = data_dir.join("tools");
        let _ = std::fs::create_dir_all(&tools_dir);

        let bin_name = if cfg!(target_os = "windows") {
            "playit.exe"
        } else {
            "playit"
        };
        let target_path = tools_dir.join(bin_name);

        if target_path.exists() {
            return Ok(target_path);
        }

        // Download official playit-agent binary from GitHub
        let download_url = if cfg!(target_os = "windows") {
            "https://github.com/playit-cloud/playit-agent/releases/latest/download/playit-windows-x86_64.exe"
        } else if cfg!(target_arch = "aarch64") {
            "https://github.com/playit-cloud/playit-agent/releases/latest/download/playit-linux-aarch64"
        } else {
            "https://github.com/playit-cloud/playit-agent/releases/latest/download/playit-linux-x86_64"
        };

        eprintln!("[Playit] Downloading agent binary from {download_url}...");
        let res = client
            .get(download_url)
            .header("User-Agent", "Ingot-Minecraft-Launcher")
            .send()
            .await
            .map_err(|e| format!("Failed to download playit binary: {e}"))?;

        let bytes = res
            .bytes()
            .await
            .map_err(|e| format!("Failed to read playit binary bytes: {e}"))?;

        std::fs::write(&target_path, bytes)
            .map_err(|e| format!("Failed to write playit binary to {}: {e}", target_path.display()))?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&target_path, std::fs::Permissions::from_mode(0o755));
        }

        Ok(target_path)
    }

    /// Starts the playit tunnel agent
    pub async fn start_tunnel<R: Runtime>(
        &self,
        app: &tauri::AppHandle<R>,
        client: &reqwest::Client,
        secret_key: Option<String>,
    ) -> Result<PlayitTunnelStatus, String> {
        let binary_path = self.ensure_playit_binary(app, client).await?;

        // Stop existing instance if running
        self.stop_tunnel().await?;

        let mut cmd = Command::new(binary_path);
        if let Some(ref key) = secret_key {
            cmd.arg("--secret").arg(key);
        }
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let mut child = cmd
            .spawn()
            .map_err(|e| format!("Failed to spawn playit agent: {e}"))?;

        let stdout = child.stdout.take().ok_or("Failed to capture playit stdout")?;
        let child_arc = Arc::new(Mutex::new(child));

        let initial_status = PlayitTunnelStatus {
            is_running: true,
            status: "starting".to_string(),
            claim_url: None,
            public_address: None,
            ping_ms: None,
        };

        {
            let mut guard = self.inner.lock().await;
            guard.status = initial_status.clone();
            guard.child = Some(child_arc.clone());
        }

        // Spawn stdout monitoring loop
        let inner_clone = self.inner.clone();
        tokio::spawn(async move {
            let mut reader = BufReader::new(stdout).lines();
            while let Ok(Some(line)) = reader.next_line().await {
                eprintln!("[PlayitAgent] {line}");

                // Parse claim link
                if line.contains("https://playit.gg/claim/") {
                    if let Some(start) = line.find("https://playit.gg/claim/") {
                        let token_part = &line[start..];
                        let url = token_part.split_whitespace().next().unwrap_or("").to_string();
                        let mut guard = inner_clone.lock().await;
                        guard.status.status = "claiming".to_string();
                        guard.status.claim_url = Some(url);
                    }
                }

                // Parse active tunnel address
                if line.contains(".ply.gg") || line.contains(".joinmc.link") {
                    for word in line.split_whitespace() {
                        if word.contains(".ply.gg") || word.contains(".joinmc.link") {
                            let clean_addr = word.trim_matches(|c: char| !c.is_alphanumeric() && c != '.' && c != ':');
                            let mut guard = inner_clone.lock().await;
                            guard.status.status = "connected".to_string();
                            guard.status.public_address = Some(clean_addr.to_string());
                        }
                    }
                }
            }

            let mut guard = inner_clone.lock().await;
            guard.status.is_running = false;
            guard.status.status = "stopped".to_string();
        });

        // Wait a short moment to check if immediate claim url or address appears
        tokio::time::sleep(Duration::from_millis(800)).await;
        Ok(self.get_status().await)
    }

    /// Stops the running playit tunnel
    pub async fn stop_tunnel(&self) -> Result<(), String> {
        let child_opt = {
            let mut guard = self.inner.lock().await;
            guard.status.is_running = false;
            guard.status.status = "stopped".to_string();
            guard.status.claim_url = None;
            guard.child.take()
        };

        if let Some(child_arc) = child_opt {
            let mut child = child_arc.lock().await;
            let _ = child.kill().await;
        }

        Ok(())
    }
}
