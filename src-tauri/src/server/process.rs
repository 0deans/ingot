use crate::minecraft::java::{ensure_java_runtime, get_required_java_version};
use crate::server::config::{
    get_server_dir, read_server_properties_from_dir, write_server_properties_to_dir,
    RunningServerSummary, ServerConfig, ServerCoreType, ServerLogEvent, ServerStatus,
    ServerStatusEvent,
};
use crate::server::downloader::ensure_server_jar;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::Runtime;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::ChildStdin;
use tokio::sync::Mutex;

struct ActiveServer {
    server_id: String,
    core: ServerCoreType,
    pid: u32,
    port: u16,
    started_at: u64,
    status: ServerStatus,
    stdin: Arc<Mutex<ChildStdin>>,
    log_history: Arc<Mutex<Vec<String>>>,
    online_players: Arc<Mutex<Vec<String>>>,
    player_xuids: Arc<Mutex<HashMap<String, String>>>,
    /// Every cleaned stdout line, for capturing command replies
    line_tx: tokio::sync::broadcast::Sender<String>,
    /// Serializes console queries so replies can't be mixed up
    query_lock: Arc<Mutex<()>>,
    /// While > 0, query replies are hidden from the user's console
    pending_queries: Arc<std::sync::atomic::AtomicUsize>,
    /// Set once the server logs "Done (...)"; commands sent earlier crash on
    /// Paper 26.x because no world is loaded yet
    ready: Arc<std::sync::atomic::AtomicBool>,
}

#[derive(Clone, Default)]
pub struct ServerProcessManager {
    servers: Arc<Mutex<HashMap<String, ActiveServer>>>,
    sleeping_servers: Arc<Mutex<std::collections::HashSet<String>>>,
    /// Servers between "Start" and a spawned process (jar download, sandbox setup).
    /// A std Mutex so the guard in `launch_server` can release it in `Drop`.
    starting_servers: Arc<std::sync::Mutex<std::collections::HashSet<String>>>,
}

impl ServerProcessManager {
    pub fn new() -> Self {
        Self {
            servers: Arc::new(Mutex::new(HashMap::new())),
            sleeping_servers: Arc::new(Mutex::new(std::collections::HashSet::new())),
            starting_servers: Arc::new(std::sync::Mutex::new(std::collections::HashSet::new())),
        }
    }

    pub async fn get_running_servers(&self) -> Vec<RunningServerSummary> {
        let guard = self.servers.lock().await;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        guard
            .values()
            .map(|s| RunningServerSummary {
                server_id: s.server_id.clone(),
                pid: s.pid,
                port: s.port,
                uptime_seconds: now.saturating_sub(s.started_at),
                status: s.status.clone(),
            })
            .collect()
    }

    pub async fn get_server_status(&self, server_id: &str) -> ServerStatus {
        if self.is_starting(server_id) {
            return ServerStatus::Starting;
        }
        let guard = self.servers.lock().await;
        if let Some(s) = guard.get(server_id) {
            return s.status.clone();
        }
        let sleeping_guard = self.sleeping_servers.lock().await;
        if sleeping_guard.contains(server_id) {
            return ServerStatus::Sleeping;
        }
        ServerStatus::Stopped
    }

    fn is_starting(&self, server_id: &str) -> bool {
        self.starting_servers
            .lock()
            .map(|s| s.contains(server_id))
            .unwrap_or(false)
    }

    pub async fn set_server_status(&self, server_id: &str, status: ServerStatus) {
        if status == ServerStatus::Sleeping {
            let mut sleeping_guard = self.sleeping_servers.lock().await;
            sleeping_guard.insert(server_id.to_string());
        } else {
            let mut sleeping_guard = self.sleeping_servers.lock().await;
            sleeping_guard.remove(server_id);
        }

        let mut guard = self.servers.lock().await;
        if let Some(s) = guard.get_mut(server_id) {
            s.status = status;
        }
    }

    pub async fn get_server_logs(&self, server_id: &str) -> Vec<String> {
        let guard = self.servers.lock().await;
        if let Some(s) = guard.get(server_id) {
            let logs = s.log_history.lock().await;
            logs.clone()
        } else {
            Vec::new()
        }
    }

    pub async fn get_server_online_players(&self, server_id: &str) -> Vec<String> {
        let guard = self.servers.lock().await;
        if let Some(s) = guard.get(server_id) {
            let players = s.online_players.lock().await;
            players.clone()
        } else {
            Vec::new()
        }
    }

    pub async fn get_server_core(&self, server_id: &str) -> Option<ServerCoreType> {
        let guard = self.servers.lock().await;
        guard.get(server_id).map(|s| s.core.clone())
    }

    pub async fn is_bedrock(&self, server_id: &str) -> bool {
        let guard = self.servers.lock().await;
        guard
            .get(server_id)
            .map(|s| s.core == ServerCoreType::Bedrock)
            .unwrap_or(false)
    }

    pub async fn get_bedrock_online_players(&self, server_id: &str) -> Vec<(String, Option<String>)> {
        let guard = self.servers.lock().await;
        if let Some(s) = guard.get(server_id) {
            let names = s.online_players.lock().await.clone();
            let xuids = s.player_xuids.lock().await.clone();
            names
                .into_iter()
                .map(|name| {
                    let xuid = xuids.get(&name).cloned();
                    (name, xuid)
                })
                .collect()
        } else {
            Vec::new()
        }
    }

    pub async fn get_player_xuid(&self, server_id: &str, name: &str) -> Option<String> {
        let guard = self.servers.lock().await;
        if let Some(s) = guard.get(server_id) {
            let xuids = s.player_xuids.lock().await;
            xuids.get(name).cloned()
        } else {
            None
        }
    }

    pub async fn send_command(&self, server_id: &str, command: &str) -> Result<(), String> {
        let stdin_opt = {
            let guard = self.servers.lock().await;
            guard.get(server_id).map(|s| s.stdin.clone())
        };

        let stdin_arc = stdin_opt.ok_or_else(|| format!("Server is not running: {server_id}"))?;
        let mut stdin = stdin_arc.lock().await;
        let line = format!("{}\n", command.trim());
        stdin
            .write_all(line.as_bytes())
            .await
            .map_err(|e| format!("Failed to write command to server stdin: {e}"))?;
        stdin
            .flush()
            .await
            .map_err(|e| format!("Failed to flush server stdin: {e}"))?;
        Ok(())
    }

    /// Sends a console command and returns the first stdout line accepted by `is_reply`.
    /// Used for live data (`data get entity`, `list`) without any server plugin.
    pub async fn query(
        &self,
        server_id: &str,
        command: &str,
        is_reply: impl Fn(&str) -> bool,
        timeout: Duration,
    ) -> Result<String, String> {
        let (query_lock, pending, line_tx) = {
            let guard = self.servers.lock().await;
            let s = guard
                .get(server_id)
                .filter(|s| s.status == ServerStatus::Running)
                .ok_or_else(|| "Server is not running".to_string())?;
            if !s.ready.load(std::sync::atomic::Ordering::SeqCst) {
                return Err("Server is still starting".to_string());
            }
            (s.query_lock.clone(), s.pending_queries.clone(), s.line_tx.clone())
        };

        let _serial = query_lock.lock().await;
        let mut rx = line_tx.subscribe();
        pending.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let result = async {
            self.send_command(server_id, command).await?;
            tokio::time::timeout(timeout, async {
                loop {
                    match rx.recv().await {
                        Ok(line) if is_reply(&line) => return Ok(line),
                        Ok(_) | Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {}
                        Err(_) => return Err("Server stopped".to_string()),
                    }
                }
            })
            .await
            .map_err(|_| format!("No reply to `{command}`"))?
        }
        .await;
        // Keep hiding briefly so a late reply doesn't leak into the user's console
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(300)).await;
            pending.fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
        });
        result
    }

    pub async fn stop_server(&self, server_id: &str) -> Result<(), String> {
        self.stop_server_within(server_id, Duration::from_secs(10)).await
    }

    /// Asks the server to stop (it saves its worlds first) and ends it if it hasn't
    /// exited after `timeout`
    pub async fn stop_server_within(&self, server_id: &str, timeout: Duration) -> Result<(), String> {
        let (stdin_arc, pid) = {
            let mut guard = self.servers.lock().await;
            if let Some(s) = guard.get_mut(server_id) {
                s.status = ServerStatus::Stopping;
                (s.stdin.clone(), s.pid)
            } else {
                return Err(format!("Server is not running: {server_id}"));
            }
        };

        // Send "stop" command to server
        {
            let mut stdin = stdin_arc.lock().await;
            let _ = stdin.write_all(b"stop\n").await;
            let _ = stdin.flush().await;
        }

        // The exit task (which holds the child while it waits) removes the server once
        // the process is gone
        let server_id_clone = server_id.to_string();
        let servers_map = self.servers.clone();
        tokio::spawn(async move {
            let deadline = tokio::time::Instant::now() + timeout;
            while tokio::time::Instant::now() < deadline {
                tokio::time::sleep(Duration::from_millis(500)).await;
                if !has_process(&servers_map, &server_id_clone, pid).await {
                    return;
                }
            }
            eprintln!("[ServerManager] Graceful stop timed out for {server_id_clone}, killing process...");
            crate::running::kill_tree(pid);
        });

        Ok(())
    }

    /// Waits until the server's process has exited, up to `timeout`
    pub async fn wait_stopped(&self, server_id: &str, timeout: Duration) -> bool {
        let deadline = tokio::time::Instant::now() + timeout;
        loop {
            if !self.servers.lock().await.contains_key(server_id) {
                return true;
            }
            if tokio::time::Instant::now() >= deadline {
                return false;
            }
            tokio::time::sleep(Duration::from_millis(250)).await;
        }
    }

    /// Servers with a running process
    pub async fn process_ids(&self) -> Vec<String> {
        self.servers.lock().await.keys().cloned().collect()
    }

    pub async fn sleeping_ids(&self) -> Vec<String> {
        self.sleeping_servers.lock().await.iter().cloned().collect()
    }

    /// Whether this exact process is one Ingot is running now
    pub async fn owns(&self, server_id: &str, pid: u32) -> bool {
        has_process(&self.servers, server_id, pid).await
    }

    pub async fn kill_server(&self, server_id: &str) -> Result<(), String> {
        let pid = {
            let guard = self.servers.lock().await;
            match guard.get(server_id) {
                Some(s) => s.pid,
                None => return Err(format!("Server is not running: {server_id}")),
            }
        };
        // By pid: the exit task holds the child while it waits, and cleans up after it
        crate::running::kill_tree(pid);
        Ok(())
    }
}

async fn has_process(servers: &Mutex<HashMap<String, ActiveServer>>, server_id: &str, pid: u32) -> bool {
    servers.lock().await.get(server_id).is_some_and(|s| s.pid == pid)
}

/// Helper to locate java.exe (or java) for console use
/// Turns sandbox setup progress into startup steps: each distinct message once, plus
/// every 10% of a download
fn sandbox_progress<F>(step: &F) -> impl Fn(&str, f32) + Send + Sync + 'static
where
    F: Fn(&str) + Clone + Send + Sync + 'static,
{
    let last_reported = std::sync::Mutex::new((String::new(), 0u32));
    let step = step.clone();
    move |msg, prog| {
        let pct = (prog * 100.0) as u32;
        let Ok(mut last) = last_reported.lock() else { return };
        if last.0 != msg || pct >= last.1 + 10 {
            *last = (msg.to_string(), pct);
            step(&format!("{msg} ({pct}%)"));
        }
    }
}

fn find_java_console_bin(path: &Path) -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    let bin_name = "java.exe";
    #[cfg(not(target_os = "windows"))]
    let bin_name = "java";

    // 1. If path points directly to an executable file (e.g. /bin/javaw.exe)
    if path.is_file() {
        let sibling_console = path.with_file_name(bin_name);
        if sibling_console.exists() {
            return Some(sibling_console);
        }
        return Some(path.to_path_buf());
    }

    // 2. Direct check in directory (e.g. dir/java.exe or dir/bin/java.exe)
    let direct = path.join(bin_name);
    if direct.exists() {
        return Some(direct);
    }

    let bin_folder = path.join("bin").join(bin_name);
    if bin_folder.exists() {
        return Some(bin_folder);
    }

    // 3. Check nested directory (e.g. jdk-25.0.4.1+1/bin/java.exe)
    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                let nested = p.join("bin").join(bin_name);
                if nested.exists() {
                    return Some(nested);
                }
                let nested_direct = p.join(bin_name);
                if nested_direct.exists() {
                    return Some(nested_direct);
                }
            }
        }
    }

    None
}

/// Strips ANSI escape sequences, carriage returns and JLine's "> " prompt
/// from a console line so it renders cleanly in the UI
fn clean_console_line(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut chars = raw.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            // CSI sequence: ESC [ params... final byte (0x40-0x7E)
            if chars.peek() == Some(&'[') {
                chars.next();
                for c in chars.by_ref() {
                    if ('\u{40}'..='\u{7e}').contains(&c) {
                        break;
                    }
                }
            }
            continue;
        }
        if c != '\r' {
            out.push(c);
        }
    }
    let mut trimmed = out.trim_start();
    while let Some(rest) = trimmed.strip_prefix('>') {
        trimmed = rest.trim_start();
    }
    trimmed.trim_end().to_string()
}

/// Removes a server from the "starting" set when launch finishes or fails
struct StartingGuard {
    set: Arc<std::sync::Mutex<std::collections::HashSet<String>>>,
    server_id: String,
}

impl Drop for StartingGuard {
    fn drop(&mut self) {
        if let Ok(mut set) = self.set.lock() {
            set.remove(&self.server_id);
        }
    }
}

/// Launches the Minecraft server process
pub async fn launch_server<R: Runtime, FLog, FStatus>(
    app: tauri::AppHandle<R>,
    pm: ServerProcessManager,
    client: reqwest::Client,
    config: ServerConfig,
    on_log: FLog,
    on_status: FStatus,
) -> Result<u32, String>
where
    FLog: Fn(ServerLogEvent) + Send + Sync + 'static,
    FStatus: Fn(ServerStatusEvent) + Send + Sync + 'static,
{
    let server_id = config.id.clone();

    // Check if already running or starting, and claim the "starting" slot atomically
    let cur_status = pm.get_server_status(&server_id).await;
    if cur_status != ServerStatus::Stopped && cur_status != ServerStatus::Sleeping {
        return Err("Server is already running".to_string());
    }
    let claimed = pm
        .starting_servers
        .lock()
        .map(|mut s| s.insert(server_id.clone()))
        .unwrap_or(false);
    if !claimed {
        return Err("Server is already starting".to_string());
    }
    let _starting = StartingGuard {
        set: pm.starting_servers.clone(),
        server_id: server_id.clone(),
    };

    let on_status = Arc::new(on_status);
    let status_cb = on_status.clone();
    let on_log = Arc::new(on_log);
    let log_cb = on_log.clone();
    let result = launch_server_inner(app, pm.clone(), client, config, move |e| log_cb(e), move |e| status_cb(e)).await;

    if let Err(ref e) = result {
        on_log(ServerLogEvent {
            server_id: server_id.clone(),
            line: format!("[Ingot] Failed to start: {e}"),
            level: "error".to_string(),
        });
    }
    if result.is_err() && pm.get_server_status(&server_id).await != ServerStatus::Running {
        on_status(ServerStatusEvent {
            server_id: server_id.clone(),
            status: ServerStatus::Stopped,
            pid: None,
        });
    }
    result
}

async fn launch_server_inner<R: Runtime, FLog, FStatus>(
    app: tauri::AppHandle<R>,
    pm: ServerProcessManager,
    client: reqwest::Client,
    config: ServerConfig,
    on_log: FLog,
    on_status: FStatus,
) -> Result<u32, String>
where
    FLog: Fn(ServerLogEvent) + Send + Sync + 'static,
    FStatus: Fn(ServerStatusEvent) + Send + Sync + 'static,
{
    let server_id = config.id.clone();
    let server_dir = get_server_dir(&app, &server_id)?;

    pm.set_server_status(&server_id, ServerStatus::Starting).await;
    on_status(ServerStatusEvent {
        server_id: server_id.clone(),
        status: ServerStatus::Starting,
        pid: None,
    });

    // Setup steps are reported as console lines (and kept for the console history)
    let on_log = Arc::new(on_log);
    let setup_log: Arc<std::sync::Mutex<Vec<String>>> = Arc::default();
    let step = {
        let on_log = on_log.clone();
        let setup_log = setup_log.clone();
        let server_id = server_id.clone();
        move |msg: &str| {
            let line = format!("[Ingot] {msg}");
            if let Ok(mut log) = setup_log.lock() {
                log.push(line.clone());
            }
            on_log(ServerLogEvent {
                server_id: server_id.clone(),
                line,
                level: "info".to_string(),
            });
        }
    };

    // 1. Ensure server.jar is downloaded
    step("Checking server files...");
    let core_type = config.core.clone();
    let game_ver = config.game_version.clone();
    let build_num = config.build_number.as_deref();

    if let Err(e) = ensure_server_jar(&client, &server_dir, &core_type, &game_ver, build_num).await {
        pm.set_server_status(&server_id, ServerStatus::Stopped).await;
        on_status(ServerStatusEvent {
            server_id: server_id.clone(),
            status: ServerStatus::Stopped,
            pid: None,
        });
        return Err(format!("Failed to download server jar: {e}"));
    }

    crate::server::config::repair_offline_uuids(&server_dir);

    // 2. Ensure eula.txt is accepted
    let eula_path = server_dir.join("eula.txt");
    if !eula_path.exists() {
        let _ = std::fs::write(eula_path, "# Agreed by Ingot\neula=true\n");
    }

    // 3. Ensure server.properties exists and has configured port & loopback bind when behind proxy
    if let Ok(mut props) = read_server_properties_from_dir(&server_dir) {
        if props.server_port != config.port {
            props.server_port = config.port;
            let _ = write_server_properties_to_dir(&server_dir, &props);
        }
    }
    let bind_ip = if config.sleep_enabled.unwrap_or(true) {
        "127.0.0.1"
    } else {
        ""
    };
    let _ = crate::server::files::write_properties(
        &server_dir,
        &[crate::server::files::PropertyEntry {
            key: "server-ip".to_string(),
            value: bind_ip.to_string(),
        }],
    );

    // 4. Build execution command (Pumpkin native vs Android PRoot sandbox vs Desktop Java)
    let mut cmd = if config.core == ServerCoreType::Pumpkin {
        step("Preparing Pumpkin server...");
        step("Downloading Pumpkin (first start only)...");
        let bin = crate::server::pumpkin::ensure_pumpkin_binary(&client, &server_dir).await?;
        // Android runs it inside the sandbox (it can't execute downloaded programs directly)
        let sandbox = if crate::server::sandbox::is_android_sandbox() {
            step("Preparing Linux sandbox...");
            Some(crate::server::sandbox::ensure_base_rootfs(&app, &client, sandbox_progress(&step)).await?)
        } else {
            None
        };
        // With sleep on, this runs on the internal port (public + 10000) behind the proxy
        let public_port = if config.sleep_enabled.unwrap_or(true) {
            if config.port >= 11024 {
                config.port - 10000
            } else {
                config.port + 10000
            }
        } else {
            config.port
        };
        crate::server::pumpkin::build_pumpkin_command(
            &bin,
            &server_dir,
            config.port,
            public_port,
            sandbox.as_ref().map(|(dir, rootfs)| (dir.as_path(), rootfs.as_path())),
        )?
    } else if config.core == ServerCoreType::Bedrock {
        step("Preparing Bedrock Dedicated Server...");
        step("Downloading Bedrock files (first start only)...");
        let bin = crate::server::bedrock::ensure_bedrock_server_binary(&client, &server_dir, &game_ver).await?;
        crate::server::bedrock::configure_bedrock_properties(&server_dir, &config)?;
        let mut b_cmd = tokio::process::Command::new(bin);
        b_cmd.current_dir(&server_dir);
        b_cmd
    } else {
        // Construct command arguments with Aikar's G1GC flags
        let mut args: Vec<String> = Vec::new();
        args.push(format!("-Xms{}M", config.memory_min_mb));
        args.push(format!("-Xmx{}M", config.memory_max_mb));

        // Aikar's high-performance server GC flags
        args.push("-XX:+UseG1GC".into());
        args.push("-XX:+ParallelRefProcEnabled".into());
        args.push("-XX:MaxGCPauseMillis=200".into());
        args.push("-XX:+UnlockExperimentalVMOptions".into());
        args.push("-XX:+DisableExplicitGC".into());
        args.push("-XX:+AlwaysPreTouch".into());
        args.push("-XX:G1NewSizePercent=30".into());
        args.push("-XX:G1MaxNewSizePercent=40".into());
        args.push("-XX:G1ReservePercent=20".into());
        args.push("-XX:G1HeapWastePercent=5".into());
        args.push("-XX:G1MixedGCCountTarget=4".into());
        args.push("-XX:InitiatingHeapOccupancyPercent=15".into());
        args.push("-XX:G1MixedGCLiveThresholdPercent=90".into());
        args.push("-XX:G1RSetUpdatingPauseTimePercent=5".into());
        args.push("-XX:SurvivorRatio=32".into());
        args.push("-XX:+PerfDisableSharedMem".into());
        args.push("-XX:MaxTenuringThreshold=1".into());
        args.push("-Dusing.aikars.flags=https://mcflags.emc.gs".into());
        args.push("-Daikars.new.flags=true".into());

        let required_java = get_required_java_version(&config.game_version, None);
        if required_java >= 24 {
            args.push("--sun-misc-unsafe-memory-access=allow".into());
        }
        if required_java >= 17 {
            args.push("--enable-native-access=ALL-UNNAMED".into());
        }

        if let Some(ref custom_args) = config.jvm_args {
            args.extend(custom_args.clone());
        }

        // Paper/Spigot: plain console without JLine prompts or ANSI colors, since
        // output goes to a pipe rendered by our UI (ignored by vanilla)
        args.push("-Dterminal.jline=false".into());
        args.push("-Dterminal.ansi=false".into());

        // Java runs natively, or inside the Linux sandbox on Android: (sandbox, java)
        let (sandbox, java_bin) = if crate::server::sandbox::is_android_sandbox() {
            step("Preparing Linux sandbox...");
            let (sandbox_dir, rootfs, guest_java) = crate::server::sandbox::ensure_sandbox_rootfs(
                &app,
                &client,
                required_java,
                sandbox_progress(&step),
            )
            .await?;
            (Some((sandbox_dir, rootfs)), PathBuf::from(guest_java))
        } else if let Some(ref path) = config.java_path {
            let p = PathBuf::from(path);
            (None, find_java_console_bin(&p).unwrap_or(p))
        } else {
            let resolved = ensure_java_runtime(&app, &client, required_java, None, None).await?;
            let bin = find_java_console_bin(&resolved)
                .ok_or_else(|| format!("Could not find java console binary in {}", resolved.display()))?;
            (None, bin)
        };
        let java = |java_args: &[String]| {
            crate::server::sandbox::build_server_command(
                sandbox.as_ref().map(|(dir, rootfs)| (dir.as_path(), rootfs.as_path())),
                &server_dir,
                &java_bin,
                java_args,
            )
        };

        if crate::server::installer::uses_installer(&config.core) {
            if crate::server::installer::launch_args(&server_dir, &config.core).is_none() {
                step(&format!("Installing {} (first start only, this can take a few minutes)...", config.core));
            }
            let launch = crate::server::installer::ensure_installed(
                &client,
                &server_dir,
                &config.core,
                &game_ver,
                config.build_number.as_deref(),
                &java,
            )
            .await?;
            args.extend(launch);
        } else {
            args.push("-jar".into());
            args.push("server.jar".into());
        }
        args.push("nogui".into());
        java(&args)?
    };

    step("Launching server process...");
    cmd.stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    #[cfg(windows)]
    {
        // CREATE_NO_WINDOW flag so command prompt window does not pop up
        cmd.creation_flags(0x08000000);
    }

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("Failed to spawn server process: {e}"))?;

    let pid = child
        .id()
        .ok_or_else(|| "Failed to get server PID".to_string())?;

    let stdin = child
        .stdin
        .take()
        .ok_or_else(|| "Failed to attach stdin".to_string())?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "Failed to attach stdout".to_string())?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| "Failed to attach stderr".to_string())?;

    let stdin_arc = Arc::new(Mutex::new(stdin));
    let child_arc = Arc::new(Mutex::new(child));
    let setup_lines = setup_log.lock().map(|l| l.clone()).unwrap_or_default();
    let log_history = Arc::new(Mutex::new(setup_lines));
    let (line_tx, _) = tokio::sync::broadcast::channel::<String>(512);
    let pending_queries = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let ready = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let ready_flag = ready.clone();
    let online_players: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let player_xuids: Arc<Mutex<HashMap<String, String>>> = Arc::new(Mutex::new(HashMap::new()));

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    {
        let mut guard = pm.servers.lock().await;
        guard.insert(
            server_id.clone(),
            ActiveServer {
                server_id: server_id.clone(),
                core: config.core.clone(),
                pid,
                port: config.port,
                started_at: now,
                status: ServerStatus::Running,
                stdin: stdin_arc.clone(),
                log_history: log_history.clone(),
                online_players: online_players.clone(),
                player_xuids: player_xuids.clone(),
                line_tx: line_tx.clone(),
                query_lock: Arc::new(Mutex::new(())),
                pending_queries: pending_queries.clone(),
                ready: ready.clone(),
            },
        );
    }
    crate::running::add(&app, crate::running::Kind::Server, &server_id, pid, now);

    on_status(ServerStatusEvent {
        server_id: server_id.clone(),
        status: ServerStatus::Running,
        pid: Some(pid),
    });

    // 7. Background task: Read STDOUT
    let s_id_out = server_id.clone();
    let server_dir_out = server_dir.clone();
    let logs_out = log_history.clone();
    let players_out = online_players.clone();
    let xuids_out = player_xuids.clone();
    let on_log_arc = on_log;
    let on_log_out = on_log_arc.clone();
    tokio::spawn(async move {
        let mut lines = BufReader::new(stdout).lines();
        while let Ok(Some(raw)) = lines.next_line().await {
            let line = clean_console_line(&raw);
            // Vanilla/Paper/Fabric: "Done (12.345s)! For help, type "help"", Bedrock: "Server started."
            if (line.contains("Done (") && line.contains("For help")) || line.contains("Server started") {
                ready.store(true, std::sync::atomic::Ordering::SeqCst);
                // Started fine after a version change: nothing more to watch
                crate::version_change::crash::first_start_ok(&server_dir_out);
            }
            let _ = line_tx.send(line.clone());
            if pending_queries.load(std::sync::atomic::Ordering::SeqCst) > 0
                && crate::server::live::is_query_reply(&line)
            {
                continue;
            }
            // Vanilla logs "[Server thread/WARN]", Paper logs "[12:00:00 WARN]"
            let level = if line.contains("WARN]") || line.contains("WARN:") {
                "warn".to_string()
            } else if line.contains("ERROR]") || line.contains("ERROR:") || line.contains("Exception:") {
                "error".to_string()
            } else {
                "info".to_string()
            };

            // Parse player join / leave events from Minecraft server output.
            // Typical formats:
            //   "[HH:MM:SS] [Server thread/INFO]: PlayerName joined the game"
            //   "[HH:MM:SS] [Server thread/INFO]: PlayerName left the game"
            //   Bedrock: "Player connected: PlayerName, xuid: ..."
            //   Bedrock: "Player disconnected: PlayerName, xuid: ..."
            if line.contains("joined the game") || line.contains("left the game")
                || line.contains("Player connected: ") || line.contains("Player disconnected: ")
            {
                let extract_player = |line: &str, keyword: &str| -> Option<String> {
                    let idx = line.find(keyword)?;
                    let before = line[..idx].trim();
                    before.split_whitespace().last().map(|s| s.to_string())
                };

                let mut guard = players_out.lock().await;
                if line.contains("joined the game") {
                    if let Some(name) = extract_player(&line, " joined the game") {
                        if !guard.contains(&name) {
                            guard.push(name);
                        }
                    }
                } else if line.contains("left the game") {
                    if let Some(name) = extract_player(&line, " left the game") {
                        guard.retain(|p| p != &name);
                    }
                } else if line.contains("Player connected: ") {
                    if let Some(rest) = line.split("Player connected: ").nth(1) {
                        let name = rest.split(',').next().unwrap_or(rest).trim().to_string();
                        if !name.is_empty() {
                            if !guard.contains(&name) {
                                guard.push(name.clone());
                            }
                            if let Some(xuid_str) = rest.split("xuid:").nth(1) {
                                let xuid = xuid_str.split(',').next().unwrap_or(xuid_str).trim().to_string();
                                if !xuid.is_empty() {
                                    let mut x_guard = xuids_out.lock().await;
                                    x_guard.insert(name, xuid);
                                }
                            }
                        }
                    }
                } else if line.contains("Player disconnected: ") {
                    if let Some(rest) = line.split("Player disconnected: ").nth(1) {
                        let name = rest.split(',').next().unwrap_or(rest).trim().to_string();
                        guard.retain(|p| p != &name);
                        let mut x_guard = xuids_out.lock().await;
                        x_guard.remove(&name);
                    }
                }
            }

            {
                let mut guard = logs_out.lock().await;
                guard.push(line.clone());
                if guard.len() > 1000 {
                    guard.remove(0);
                }
            }

            on_log_out(ServerLogEvent {
                server_id: s_id_out.clone(),
                line,
                level,
            });
        }
        // Clear player list when stdout closes (server stopped)
        let mut guard = players_out.lock().await;
        guard.clear();
        let mut x_guard = xuids_out.lock().await;
        x_guard.clear();
    });


    // 8. Background task: Read STDERR
    let s_id_err = server_id.clone();
    let logs_err = log_history.clone();
    let on_log_err = on_log_arc.clone();
    tokio::spawn(async move {
        let mut lines = BufReader::new(stderr).lines();
        while let Ok(Some(raw)) = lines.next_line().await {
            let line = clean_console_line(&raw);
            {
                let mut guard = logs_err.lock().await;
                guard.push(line.clone());
                if guard.len() > 1000 {
                    guard.remove(0);
                }
            }

            on_log_err(ServerLogEvent {
                server_id: s_id_err.clone(),
                line,
                level: "error".to_string(),
            });
        }
    });

    // 9. Background task: Monitor process exit
    let pm_exit = pm.clone();
    let s_id_exit = server_id.clone();
    let on_status_arc = Arc::new(on_status);
    let on_status_exit = on_status_arc.clone();
    let started_at = std::time::SystemTime::now() - Duration::from_secs(2);
    let (app_exit, dir_exit, name_exit, core_exit) = (app.clone(), server_dir.clone(), config.name.clone(), config.core.clone());
    let ready_exit = ready_flag.clone();
    tokio::spawn(async move {
        let mut child = child_arc.lock().await;
        let exited = child.wait().await;
        // Stopped by the user before it was ready, or crashed on the way up?
        let stopping = pm_exit.get_server_status(&s_id_exit).await == ServerStatus::Stopping;
        let started = ready_exit.load(std::sync::atomic::Ordering::SeqCst);
        if !started && !stopping {
            let clean = exited.is_ok_and(|s| s.success());
            let folder = crate::server::plugins::platform(&core_exit).map(|p| p.folder);
            if let Some(crash) = crate::version_change::crash::after_exit(
                &dir_exit,
                clean,
                crate::version_change::TargetKind::Server,
                &s_id_exit,
                &name_exit,
                folder.as_slice(),
                started_at,
            ) {
                crate::ipc::emit_version_change_crash(&app_exit, crash);
            }
        }

        crate::running::remove(&app_exit, crate::running::Kind::Server, &s_id_exit, pid);
        {
            let mut guard = pm_exit.servers.lock().await;
            guard.remove(&s_id_exit);
        }

        on_status_exit(ServerStatusEvent {
            server_id: s_id_exit,
            status: ServerStatus::Stopped,
            pid: None,
        });
    });

    Ok(pid)
}
