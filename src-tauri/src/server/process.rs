use crate::minecraft::java::{ensure_java_runtime, get_required_java_version};
use crate::server::config::{
    get_server_dir, read_server_properties_from_dir, write_server_properties_to_dir,
    RunningServerSummary, ServerConfig, ServerLogEvent, ServerStatus, ServerStatusEvent,
};
use crate::server::downloader::ensure_server_jar;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::Runtime;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, Command};
use tokio::sync::Mutex;

struct ActiveServer {
    server_id: String,
    pid: u32,
    port: u16,
    started_at: u64,
    status: ServerStatus,
    stdin: Arc<Mutex<ChildStdin>>,
    child: Arc<Mutex<Child>>,
    log_history: Arc<Mutex<Vec<String>>>,
    online_players: Arc<Mutex<Vec<String>>>,
}

#[derive(Clone, Default)]
pub struct ServerProcessManager {
    servers: Arc<Mutex<HashMap<String, ActiveServer>>>,
}

impl ServerProcessManager {
    pub fn new() -> Self {
        Self {
            servers: Arc::new(Mutex::new(HashMap::new())),
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
        let guard = self.servers.lock().await;
        guard
            .get(server_id)
            .map(|s| s.status.clone())
            .unwrap_or(ServerStatus::Stopped)
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

    pub async fn stop_server(&self, server_id: &str) -> Result<(), String> {
        let (stdin_arc, child_arc) = {
            let mut guard = self.servers.lock().await;
            if let Some(s) = guard.get_mut(server_id) {
                s.status = ServerStatus::Stopping;
                (s.stdin.clone(), s.child.clone())
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

        // Wait asynchronously for up to 10 seconds for graceful shutdown
        let server_id_clone = server_id.to_string();
        let servers_map = self.servers.clone();
        tokio::spawn(async move {
            for _ in 0..20 {
                tokio::time::sleep(Duration::from_millis(500)).await;
                let is_exited = {
                    let mut child = child_arc.lock().await;
                    match child.try_wait() {
                        Ok(Some(_)) => true,
                        _ => false,
                    }
                };
                if is_exited {
                    let mut guard = servers_map.lock().await;
                    guard.remove(&server_id_clone);
                    return;
                }
            }

            // Force kill if timed out
            eprintln!("[ServerManager] Graceful stop timed out for {server_id_clone}, killing process...");
            let mut child = child_arc.lock().await;
            let _ = child.kill().await;
            let mut guard = servers_map.lock().await;
            guard.remove(&server_id_clone);
        });

        Ok(())
    }

    pub async fn kill_server(&self, server_id: &str) -> Result<(), String> {
        let child_arc = {
            let mut guard = self.servers.lock().await;
            if let Some(s) = guard.remove(server_id) {
                s.child
            } else {
                return Err(format!("Server is not running: {server_id}"));
            }
        };

        let mut child = child_arc.lock().await;
        child
            .kill()
            .await
            .map_err(|e| format!("Failed to kill server process: {e}"))?;
        Ok(())
    }
}

/// Helper to locate java.exe (or java) for console use
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
    let server_dir = get_server_dir(&app, &server_id)?;

    // Check if already running
    if pm.get_server_status(&server_id).await != ServerStatus::Stopped {
        return Err("Server is already running".to_string());
    }

    on_status(ServerStatusEvent {
        server_id: server_id.clone(),
        status: ServerStatus::Starting,
        pid: None,
    });

    // 1. Ensure server.jar is downloaded
    let core_type = config.core.clone();
    let game_ver = config.game_version.clone();
    let build_num = config.build_number.as_deref();

    if let Err(e) = ensure_server_jar(&client, &server_dir, &core_type, &game_ver, build_num).await {
        on_status(ServerStatusEvent {
            server_id: server_id.clone(),
            status: ServerStatus::Stopped,
            pid: None,
        });
        return Err(format!("Failed to download server jar: {e}"));
    }

    // 2. Ensure eula.txt is accepted
    let eula_path = server_dir.join("eula.txt");
    if !eula_path.exists() {
        let _ = std::fs::write(eula_path, "# Agreed by Ingot\neula=true\n");
    }

    // 3. Ensure server.properties exists and has configured port
    if let Ok(mut props) = read_server_properties_from_dir(&server_dir) {
        if props.server_port != config.port {
            props.server_port = config.port;
            let _ = write_server_properties_to_dir(&server_dir, &props);
        }
    }

    // 4. Resolve Java runtime
    let required_java = get_required_java_version(&config.game_version, None);
    let java_bin = if let Some(ref path) = config.java_path {
        let p = PathBuf::from(path);
        find_java_console_bin(&p).unwrap_or(p)
    } else {
        // Ensure Adoptium runtime is installed
        let resolved = ensure_java_runtime(&app, &client, required_java, None, None).await?;
        find_java_console_bin(&resolved).ok_or_else(|| {
            format!(
                "Could not find java console binary in {}",
                resolved.display()
            )
        })?
    };

    // 5. Construct command arguments with Aikar's G1GC flags
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

    // Modern Java module flags
    if required_java >= 24 {
        args.push("--sun-misc-unsafe-memory-access=allow".into());
    }
    if required_java >= 17 {
        args.push("--enable-native-access=ALL-UNNAMED".into());
    }

    // Instance custom JVM args if configured
    if let Some(ref custom_args) = config.jvm_args {
        args.extend(custom_args.clone());
    }

    // Jar & nogui
    args.push("-jar".into());
    args.push("server.jar".into());
    args.push("nogui".into());

    // 6. Spawn process
    let mut cmd = Command::new(&java_bin);
    cmd.args(&args)
        .current_dir(&server_dir)
        .stdin(Stdio::piped())
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
    let log_history = Arc::new(Mutex::new(Vec::<String>::new()));
    let online_players: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));

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
                pid,
                port: config.port,
                started_at: now,
                status: ServerStatus::Running,
                stdin: stdin_arc.clone(),
                child: child_arc.clone(),
                log_history: log_history.clone(),
                online_players: online_players.clone(),
            },
        );
    }

    on_status(ServerStatusEvent {
        server_id: server_id.clone(),
        status: ServerStatus::Running,
        pid: Some(pid),
    });

    // 7. Background task: Read STDOUT
    let s_id_out = server_id.clone();
    let logs_out = log_history.clone();
    let players_out = online_players.clone();
    let on_log_arc = Arc::new(on_log);
    let on_log_out = on_log_arc.clone();
    tokio::spawn(async move {
        let mut lines = BufReader::new(stdout).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            let level = if line.contains("[WARN]") || line.contains("WARN:") {
                "warn".to_string()
            } else if line.contains("[ERROR]") || line.contains("ERROR:") || line.contains("Exception:") {
                "error".to_string()
            } else {
                "info".to_string()
            };

            // Parse player join / leave events from Minecraft server output.
            // Typical formats:
            //   "[HH:MM:SS] [Server thread/INFO]: PlayerName joined the game"
            //   "[HH:MM:SS] [Server thread/INFO]: PlayerName left the game"
            if line.contains("joined the game") || line.contains("left the game") {
                // Extract the player name: the word just before "joined" or "left"
                let extract_player = |line: &str, keyword: &str| -> Option<String> {
                    let idx = line.find(keyword)?;
                    let before = line[..idx].trim();
                    // The name is the last word before the keyword
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
    });


    // 8. Background task: Read STDERR
    let s_id_err = server_id.clone();
    let logs_err = log_history.clone();
    let on_log_err = on_log_arc.clone();
    tokio::spawn(async move {
        let mut lines = BufReader::new(stderr).lines();
        while let Ok(Some(line)) = lines.next_line().await {
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
    tokio::spawn(async move {
        let mut child = child_arc.lock().await;
        let _ = child.wait().await;

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
