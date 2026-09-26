pub use crate::server::config::PlayitTunnelStatus;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;
use tauri::{Manager, Runtime};
use tokio::io::{AsyncBufReadExt, AsyncRead, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

const PLAYIT_API: &str = "https://api.playit.gg";
const SECRET_FILE: &str = "secret.key";

/// Manages the playit.gg agent.
///
/// playit v1.x (`playitd`) no longer prints a claim link: without a secret it waits
/// for a GUI frontend over IPC. So Ingot performs the claim flow itself through the
/// playit API, stores the secret, runs the daemon with `--secret`, and reads the
/// tunnel address back from the API.
#[derive(Clone, Default)]
pub struct PlayitManager {
    inner: Arc<Mutex<PlayitInner>>,
}

struct PlayitInner {
    status: PlayitTunnelStatus,
    child: Option<Arc<Mutex<Child>>>,
    task: Option<JoinHandle<()>>,
}

fn stopped_status() -> PlayitTunnelStatus {
    PlayitTunnelStatus {
        is_running: false,
        status: "stopped".to_string(),
        claim_url: None,
        public_address: None,
        ping_ms: None,
        message: None,
    }
}

impl Default for PlayitInner {
    fn default() -> Self {
        Self {
            status: stopped_status(),
            child: None,
            task: None,
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

    /// Resolves or downloads the Playit agent binary.
    /// On Android the binary is launched through PRoot, so it may live on the noexec
    /// app-data partition; the bundled `libplayit.so` is still preferred.
    pub async fn ensure_playit_binary<R: Runtime>(
        &self,
        app: &tauri::AppHandle<R>,
        client: &reqwest::Client,
    ) -> Result<PathBuf, String> {
        // 1. Fast path: bundled libplayit.so in nativeLibraryDir (APK bundle)
        #[cfg(target_os = "android")]
        if let Ok(lib_dir) = std::env::var("ANDROID_APP_LIB_DIR") {
            let p = PathBuf::from(&lib_dir).join("libplayit.so");
            if p.exists() {
                return Ok(p);
            }
        }

        // 2. Desktop: check system PATH
        #[cfg(not(target_os = "android"))]
        if let Ok(path) = std::env::var("PATH") {
            for dir in std::env::split_paths(&path) {
                let p = dir.join(if cfg!(windows) { "playit.exe" } else { "playit" });
                if p.exists() {
                    return Ok(p);
                }
            }
        }

        // 3. App-data cache dir (download target)
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
            // Sanity-check: must be > 1 MB (reject corrupt/partial downloads)
            if std::fs::metadata(&target_path).map(|m| m.len() > 1_000_000).unwrap_or(false) {
                return Ok(target_path);
            }
            let _ = std::fs::remove_file(&target_path);
        }

        // Download official playit-agent binary from GitHub
        let download_url = if cfg!(target_os = "windows") {
            "https://github.com/playit-cloud/playit-agent/releases/latest/download/playit-windows-x86_64.exe"
        } else if cfg!(target_arch = "aarch64") {
            "https://github.com/playit-cloud/playit-agent/releases/latest/download/playit-linux-aarch64"
        } else {
            "https://github.com/playit-cloud/playit-agent/releases/latest/download/playit-linux-amd64"
        };

        eprintln!("[Playit] Downloading agent binary from {download_url}...");
        let res = client
            .get(download_url)
            .header("User-Agent", crate::USER_AGENT)
            .send()
            .await
            .map_err(|e| format!("Failed to download playit binary: {e}"))?;

        if !res.status().is_success() {
            return Err(format!(
                "Failed to download playit binary from {download_url}: HTTP {}",
                res.status()
            ));
        }

        let bytes = res
            .bytes()
            .await
            .map_err(|e| format!("Failed to read playit binary bytes: {e}"))?;

        std::fs::write(&target_path, &bytes).map_err(|e| {
            format!(
                "Failed to write playit binary to {}: {e}",
                target_path.display()
            )
        })?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(
                &target_path,
                std::fs::Permissions::from_mode(0o755),
            );
        }

        Ok(target_path)
    }

    /// Starts the playit tunnel agent. Claiming and connecting continue in the
    /// background; the UI follows progress through `get_status`.
    pub async fn start_tunnel<R: Runtime>(
        &self,
        app: &tauri::AppHandle<R>,
        client: &reqwest::Client,
        secret_key: Option<String>,
    ) -> Result<PlayitTunnelStatus, String> {
        // Stop existing instance if running
        self.stop_tunnel().await?;

        let binary_path = self.ensure_playit_binary(app, client).await?;

        let data_dir = app
            .path()
            .app_data_dir()
            .map_err(|e| format!("Failed to get app data directory: {e}"))?;
        let playit_dir = data_dir.join("playit");
        std::fs::create_dir_all(&playit_dir)
            .map_err(|e| format!("Failed to create playit directory: {e}"))?;

        let sandbox_dir = if cfg!(target_os = "android") {
            Some(crate::server::sandbox::get_sandbox_dir(app)?)
        } else {
            None
        };

        let secret = secret_key
            .filter(|s| !s.trim().is_empty())
            .or_else(|| read_saved_secret(&playit_dir));

        let ctx = AgentContext {
            inner: self.inner.clone(),
            client: client.clone(),
            binary_path,
            playit_dir,
            sandbox_dir,
        };

        {
            let mut guard = self.inner.lock().await;
            guard.status = PlayitTunnelStatus {
                is_running: true,
                status: "starting".to_string(),
                ..stopped_status()
            };
            guard.task = Some(tokio::spawn(run_agent(ctx, secret)));
        }

        // Wait a short moment so an immediate claim URL is included in the response
        tokio::time::sleep(Duration::from_millis(800)).await;
        Ok(self.get_status().await)
    }

    /// Stops the running playit tunnel
    pub async fn stop_tunnel(&self) -> Result<(), String> {
        let (task, child) = {
            let mut guard = self.inner.lock().await;
            guard.status = stopped_status();
            (guard.task.take(), guard.child.take())
        };

        if let Some(task) = task {
            task.abort();
        }
        if let Some(child_arc) = child {
            let mut child = child_arc.lock().await;
            let _ = child.kill().await;
        }

        Ok(())
    }
}

struct AgentContext {
    inner: Arc<Mutex<PlayitInner>>,
    client: reqwest::Client,
    binary_path: PathBuf,
    playit_dir: PathBuf,
    sandbox_dir: Option<PathBuf>,
}

impl AgentContext {
    async fn update(&self, f: impl FnOnce(&mut PlayitTunnelStatus)) {
        let mut guard = self.inner.lock().await;
        f(&mut guard.status);
    }
}

fn read_saved_secret(playit_dir: &Path) -> Option<String> {
    std::fs::read_to_string(playit_dir.join(SECRET_FILE))
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// Background task: claim (if needed), launch the daemon, and track the tunnel address
async fn run_agent(ctx: AgentContext, secret: Option<String>) {
    if let Err(message) = run_agent_inner(&ctx, secret).await {
        eprintln!("[Playit] {message}");
        let child = {
            let mut guard = ctx.inner.lock().await;
            guard.status = PlayitTunnelStatus {
                status: "error".to_string(),
                message: Some(message),
                ..stopped_status()
            };
            guard.child.take()
        };
        if let Some(child_arc) = child {
            let _ = child_arc.lock().await.kill().await;
        }
    }
}

async fn run_agent_inner(ctx: &AgentContext, secret: Option<String>) -> Result<(), String> {
    let secret = match secret {
        Some(s) => s,
        None => {
            let s = claim_agent(ctx).await?;
            let _ = std::fs::write(ctx.playit_dir.join(SECRET_FILE), &s);
            s
        }
    };

    ctx.update(|s| {
        s.status = "connecting".to_string();
        s.claim_url = None;
    })
    .await;

    let mut cmd = build_agent_command(ctx, &secret)?;
    cmd.stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);

    #[cfg(windows)]
    {
        // CREATE_NO_WINDOW flag so a console window does not pop up
        cmd.creation_flags(0x08000000);
    }

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("Failed to spawn playit agent: {e}"))?;

    let last_log = Arc::new(Mutex::new(String::new()));
    if let Some(stdout) = child.stdout.take() {
        tokio::spawn(forward_logs(stdout, last_log.clone()));
    }
    if let Some(stderr) = child.stderr.take() {
        tokio::spawn(forward_logs(stderr, last_log.clone()));
    }

    let child_arc = Arc::new(Mutex::new(child));
    ctx.inner.lock().await.child = Some(child_arc.clone());

    loop {
        if let Ok(Some(exit)) = child_arc.lock().await.try_wait() {
            let last = last_log.lock().await.clone();
            if last.contains("InvalidAgentKey") || last.contains("no longer valid") {
                let _ = std::fs::remove_file(ctx.playit_dir.join(SECRET_FILE));
                return Err("The saved playit secret is no longer valid. Tap Connect again to link this device to playit.gg.".to_string());
            }
            return Err(format!("playit agent exited ({exit}): {last}"));
        }

        match api_call(&ctx.client, "/v1/agents/rundata", json!({}), Some(&secret)).await {
            Ok(data) => apply_rundata(ctx, &data).await,
            Err(e) => eprintln!("[Playit] Failed to fetch tunnel info: {e}"),
        }

        tokio::time::sleep(Duration::from_secs(5)).await;
    }
}

/// Builds the daemon command. On Android, playit (a static musl binary) resolves DNS
/// via `/etc/resolv.conf`, which Android doesn't have, so it runs under PRoot with
/// a resolv.conf bound in. PRoot also lets it execute from noexec storage.
fn build_agent_command(ctx: &AgentContext, secret: &str) -> Result<Command, String> {
    let mut cmd = if let Some(ref sandbox_dir) = ctx.sandbox_dir {
        let resolv = ctx.playit_dir.join("resolv.conf");
        std::fs::write(&resolv, "nameserver 1.1.1.1\nnameserver 8.8.8.8\n")
            .map_err(|e| format!("Failed to write playit resolv.conf: {e}"))?;
        let mut cmd = crate::server::sandbox::proot_command(
            sandbox_dir,
            None,
            false,
            &[(resolv, "/etc/resolv.conf")],
            &ctx.playit_dir.to_string_lossy(),
        )?;
        cmd.env("HOME", &ctx.playit_dir);
        cmd.env("XDG_CONFIG_HOME", &ctx.playit_dir);
        cmd.arg(&ctx.binary_path);
        cmd
    } else {
        Command::new(&ctx.binary_path)
    };

    cmd.current_dir(&ctx.playit_dir);
    cmd.arg("--secret").arg(secret);

    // The default IPC socket lives in a system dir (e.g. /run) that apps can't write to
    #[cfg(unix)]
    {
        let socket = ctx.playit_dir.join("playit.sock");
        let _ = std::fs::remove_file(&socket);
        cmd.arg("--socket-path").arg(socket);
    }

    Ok(cmd)
}

async fn forward_logs<T: AsyncRead + Unpin>(stream: T, last_log: Arc<Mutex<String>>) {
    let mut reader = BufReader::new(stream).lines();
    while let Ok(Some(line)) = reader.next_line().await {
        eprintln!("[PlayitAgent] {line}");
        if !line.trim().is_empty() {
            *last_log.lock().await = line;
        }
    }
}

/// Calls the playit API, which wraps responses as `{"status": "success", "data": ...}`
async fn api_call(
    client: &reqwest::Client,
    path: &str,
    body: Value,
    secret: Option<&str>,
) -> Result<Value, String> {
    let mut req = client.post(format!("{PLAYIT_API}{path}")).json(&body);
    if let Some(secret) = secret {
        req = req.header("Authorization", format!("Agent-Key {secret}"));
    }
    let res: Value = req
        .send()
        .await
        .map_err(|e| format!("playit API request failed: {}", error_chain(&e)))?
        .json()
        .await
        .map_err(|e| format!("Invalid playit API response: {e}"))?;

    if res["status"] == "success" {
        Ok(res["data"].clone())
    } else {
        Err(res["data"].to_string())
    }
}

/// reqwest's Display hides the root cause (DNS, TLS, connect); include every source
fn error_chain(e: &dyn std::error::Error) -> String {
    let mut out = e.to_string();
    let mut source = e.source();
    while let Some(s) = source {
        out.push_str(&format!(": {s}"));
        source = s.source();
    }
    out
}

/// Links this agent to the user's playit.gg account and returns its secret key
async fn claim_agent(ctx: &AgentContext) -> Result<String, String> {
    let code: String = uuid::Uuid::new_v4().simple().to_string()[..10].to_string();
    let claim_url = format!("https://playit.gg/claim/{code}");

    ctx.update(|s| {
        s.status = "claiming".to_string();
        s.claim_url = Some(claim_url);
    })
    .await;

    let version = format!("ingot {}", env!("CARGO_PKG_VERSION"));
    loop {
        let setup = api_call(
            &ctx.client,
            "/claim/setup",
            json!({ "code": code, "agent_type": "self-managed", "version": version }),
            None,
        )
        .await;

        match setup.as_ref().ok().and_then(|v| v.as_str()) {
            Some("UserAccepted") => break,
            Some("UserRejected") => return Err("The playit.gg claim was rejected.".to_string()),
            Some(_) => {}
            None => {
                let err = setup.err().unwrap_or_default();
                if err.contains("CodeExpired") || err.contains("InvalidCode") {
                    return Err("The playit.gg claim link expired. Tap Connect to get a new one.".to_string());
                }
                eprintln!("[Playit] Claim status check failed: {err}");
            }
        }
        tokio::time::sleep(Duration::from_secs(2)).await;
    }

    for _ in 0..10 {
        match api_call(&ctx.client, "/claim/exchange", json!({ "code": code }), None).await {
            Ok(data) => {
                if let Some(secret) = data["secret_key"].as_str() {
                    return Ok(secret.to_string());
                }
                return Err(format!("Unexpected claim exchange response: {data}"));
            }
            Err(e) if e.contains("NotAccepted") || e.contains("request failed") => {
                tokio::time::sleep(Duration::from_secs(2)).await;
            }
            Err(e) => return Err(format!("Failed to finish playit.gg claim: {e}")),
        }
    }
    Err("Timed out finishing the playit.gg claim.".to_string())
}

/// Picks the tunnel to show from `/v1/agents/rundata`, preferring Minecraft Java ones
async fn apply_rundata(ctx: &AgentContext, data: &Value) {
    let tunnels: Vec<&Value> = data["tunnels"]
        .as_array()
        .map(|t| t.iter().filter(|t| t["disabled_reason"].is_null()).collect())
        .unwrap_or_default();

    let tunnel = tunnels
        .iter()
        .find(|t| t["tunnel_type"] == "minecraft-java")
        .or_else(|| tunnels.first());

    let address = tunnel
        .and_then(|t| t["display_address"].as_str())
        .map(str::to_string);
    let has_pending = data["pending"].as_array().is_some_and(|p| !p.is_empty());
    let notice = data["notices"]
        .as_array()
        .and_then(|n| n.first())
        .and_then(|n| n["message"].as_str())
        .map(str::to_string);

    ctx.update(|s| {
        s.public_address = address.clone();
        if address.is_some() {
            s.status = "connected".to_string();
            s.message = notice;
        } else if has_pending {
            s.status = "connecting".to_string();
            s.message = Some("playit.gg is allocating your tunnel...".to_string());
        } else {
            s.status = "no_tunnel".to_string();
            s.message = Some(notice.unwrap_or_else(|| {
                "Agent connected, but it has no tunnels. Add a \"Minecraft Java\" tunnel for this agent on playit.gg pointing to your server port.".to_string()
            }));
        }
    })
    .await;
}
