//! Live performance numbers for a running server: CPU and memory of its process tree
//! (on Android the Java server runs under PRoot, so children are included) and TPS.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::{LazyLock, Mutex};
use std::time::Duration;
use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System};

use super::process::ServerProcessManager;

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ServerStats {
    /// Share of the whole machine's CPU, 0-100
    #[specta(type = i32)]
    pub cpu_percent: f32,
    pub memory_mb: u32,
    pub system_memory_used_mb: u32,
    pub system_memory_total_mb: u32,
    /// Ticks per second over the last minute (Paper/Purpur/Folia only)
    #[specta(type = Option<i32>)]
    pub tps: Option<f32>,
}

/// Kept between calls: CPU usage is measured as the difference since the last refresh
static SYSTEM: Mutex<Option<System>> = Mutex::new(None);

/// CPU (% of all cores) and resident memory of `root` and all its descendants
fn process_tree_usage(root: u32) -> (f32, u64, u64, u64) {
    let Ok(mut guard) = SYSTEM.lock() else {
        return (0.0, 0, 0, 0);
    };
    let sys = guard.get_or_insert_with(System::new);
    sys.refresh_processes_specifics(
        ProcessesToUpdate::All,
        true,
        ProcessRefreshKind::nothing().with_cpu().with_memory(),
    );
    sys.refresh_memory();

    let root = Pid::from_u32(root);
    let in_tree = |mut pid: Pid| -> bool {
        // Walk up the parent chain (bounded, in case of cycles in stale data)
        for _ in 0..16 {
            if pid == root {
                return true;
            }
            match sys.process(pid).and_then(|p| p.parent()) {
                Some(parent) => pid = parent,
                None => return false,
            }
        }
        false
    };
    let mut mem = 0u64;
    // (pid, sysinfo's CPU %) of every process in the tree
    let mut tree: Vec<(u32, f32)> = Vec::new();
    for (pid, process) in sys.processes() {
        // On Linux/Android every thread is listed as a task sharing its process's memory;
        // counting them would multiply the JVM's RAM by its thread count
        if process.thread_kind().is_some() {
            continue;
        }
        if in_tree(*pid) {
            mem += process.memory();
            tree.push((pid.as_u32(), process.cpu_usage()));
        }
    }
    let cores = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1) as f32;
    // sysinfo's per-process CPU needs /proc/stat, which Android doesn't let apps read
    // (it silently reports 0); measure from the processes' own counters instead
    #[cfg(target_os = "android")]
    let cpu = proc_cpu_percent(root.as_u32(), tree.iter().map(|(pid, _)| *pid));
    #[cfg(not(target_os = "android"))]
    let cpu: f32 = tree.iter().map(|(_, cpu)| cpu).sum();
    (
        (cpu / cores).clamp(0.0, 100.0),
        mem,
        sys.used_memory(),
        sys.total_memory(),
    )
}

/// Previous CPU-time reading per root process: (when, total clock ticks)
#[cfg(target_os = "android")]
static CPU_SAMPLES: LazyLock<Mutex<HashMap<u32, (std::time::Instant, u64)>>> =
    LazyLock::new(Default::default);

/// CPU time (user + system clock ticks) of a process and all its threads
#[cfg(target_os = "android")]
fn proc_cpu_ticks(pid: u32) -> Option<u64> {
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    parse_cpu_ticks(&stat)
}

/// utime + stime from /proc/<pid>/stat (fields 14 and 15). The name in parentheses can
/// contain spaces, so count fields from after its closing parenthesis.
#[cfg(any(target_os = "android", test))]
fn parse_cpu_ticks(stat: &str) -> Option<u64> {
    let fields: Vec<&str> = stat.rsplit_once(')')?.1.split_whitespace().collect();
    Some(fields.get(11)?.parse::<u64>().ok()? + fields.get(12)?.parse::<u64>().ok()?)
}

/// CPU use of the process tree since the last call, as % of one core
#[cfg(target_os = "android")]
fn proc_cpu_percent(root: u32, tree: impl Iterator<Item = u32>) -> f32 {
    let ticks: u64 = tree.filter_map(proc_cpu_ticks).sum();
    let now = std::time::Instant::now();
    let Ok(mut samples) = CPU_SAMPLES.lock() else {
        return 0.0;
    };
    let previous = samples.insert(root, (now, ticks));
    let Some((then, before)) = previous else {
        return 0.0;
    };
    let elapsed = now.duration_since(then).as_secs_f32();
    // SAFETY: sysconf only reads a system constant
    let per_second = match unsafe { libc::sysconf(libc::_SC_CLK_TCK) } {
        n if n > 0 => n as f32,
        _ => 100.0,
    };
    if elapsed <= 0.0 {
        return 0.0;
    }
    // A child process that exited takes its ticks with it; that's a drop, not negative use
    ticks.saturating_sub(before) as f32 / per_second / elapsed * 100.0
}

/// "TPS from last 1m, 5m, 15m: 20.0, 19.98, *20.0" -> 20.0
fn parse_tps(line: &str) -> Option<f32> {
    let values = line.rsplit_once(": ")?.1;
    values
        .split(',')
        .next()?
        .trim()
        .trim_start_matches('*')
        .parse()
        .ok()
}

pub async fn stats(
    pm: &ServerProcessManager,
    server_id: &str,
    pid: u32,
    has_tps: bool,
) -> ServerStats {
    let (cpu, mem, used, total) = tokio::task::spawn_blocking(move || process_tree_usage(pid))
        .await
        .unwrap_or((0.0, 0, 0, 0));
    let tps = if has_tps {
        pm.query(
            server_id,
            "tps",
            |l| l.contains("TPS from last"),
            Duration::from_secs(2),
        )
        .await
        .ok()
        .and_then(|l| parse_tps(&l))
    } else {
        None
    };
    let mb = |b: u64| (b / (1024 * 1024)).min(u32::MAX as u64) as u32;
    ServerStats {
        cpu_percent: cpu,
        memory_mb: mb(mem),
        system_memory_used_mb: mb(used),
        system_memory_total_mb: mb(total),
        tps,
    }
}

/// Samples kept per server: two minutes at one sample every SAMPLE_INTERVAL
pub const HISTORY_LEN: usize = 60;
const SAMPLE_INTERVAL: Duration = Duration::from_secs(2);

/// Recent samples per running server. Recorded in the background so the graphs keep
/// their history while the UI shows other pages (or other servers).
static HISTORIES: LazyLock<Mutex<HashMap<String, VecDeque<ServerStats>>>> =
    LazyLock::new(Default::default);
/// Servers with a sampler task running
static SAMPLING: LazyLock<Mutex<HashSet<String>>> = LazyLock::new(Default::default);

async fn running_pid(pm: &ServerProcessManager, server_id: &str) -> Option<u32> {
    pm.get_running_servers()
        .await
        .into_iter()
        .find(|s| s.server_id == server_id && s.pid > 0)
        .map(|s| s.pid)
}

/// Starts recording stats for a server unless already recording. The task ends (and
/// drops the history) once the server stops; a restart begins a fresh history.
pub fn ensure_sampler(pm: ServerProcessManager, server_id: &str, has_tps: bool) {
    {
        let Ok(mut sampling) = SAMPLING.lock() else {
            return;
        };
        if !sampling.insert(server_id.to_string()) {
            return;
        }
    }
    let server_id = server_id.to_string();
    tauri::async_runtime::spawn(async move {
        // The process can take a moment to appear after a start request
        let mut missing = 0;
        let mut last_pid = None;
        loop {
            match running_pid(&pm, &server_id).await {
                Some(pid) => {
                    missing = 0;
                    if last_pid.is_some_and(|last| last != pid) {
                        // New process (restart, wake from sleep): old numbers don't apply
                        if let Ok(mut h) = HISTORIES.lock() {
                            h.remove(&server_id);
                        }
                    }
                    last_pid = Some(pid);
                    let sample = stats(&pm, &server_id, pid, has_tps).await;
                    if let Ok(mut h) = HISTORIES.lock() {
                        let history = h.entry(server_id.clone()).or_default();
                        if history.len() >= HISTORY_LEN {
                            history.pop_front();
                        }
                        history.push_back(sample);
                    }
                }
                None => {
                    missing += 1;
                    if missing > 15 {
                        break;
                    }
                }
            }
            tokio::time::sleep(SAMPLE_INTERVAL).await;
        }
        if let Ok(mut h) = HISTORIES.lock() {
            h.remove(&server_id);
        }
        if let Ok(mut sampling) = SAMPLING.lock() {
            sampling.remove(&server_id);
        }
    });
}

/// Recorded samples, oldest first
pub fn history(server_id: &str) -> Vec<ServerStats> {
    HISTORIES
        .lock()
        .ok()
        .and_then(|h| h.get(server_id).map(|q| q.iter().cloned().collect()))
        .unwrap_or_default()
}

/// Interfaces that aren't the real local network (VPNs, virtual switches, containers)
const VIRTUAL_INTERFACES: [&str; 14] = [
    "warp",
    "vpn",
    "tun",
    "tap",
    "wg",
    "wireguard",
    "zerotier",
    "tailscale",
    "vethernet",
    "docker",
    "virtualbox",
    "vmware",
    "hyper-v",
    "utun",
];

/// How likely an interface is the Wi-Fi/Ethernet network friends are on
fn lan_score(name: &str, ip: std::net::Ipv4Addr) -> Option<u32> {
    let name = name.to_ascii_lowercase();
    if ip.is_loopback() || ip.is_link_local() || !ip.is_private() {
        return None;
    }
    if VIRTUAL_INTERFACES.iter().any(|v| name.contains(v)) {
        return None;
    }
    // Linux/Android/macOS names (wlan0, eth0, en0) by prefix; Windows names by word
    let physical = ["wlan", "eth", "en"].iter().any(|p| name.starts_with(p))
        || ["wi-fi", "wifi", "wireless", "ethernet"]
            .iter()
            .any(|p| name.contains(p));
    let home_range = ip.octets()[0] == 192;
    Some(u32::from(physical) * 2 + u32::from(home_range))
}

/// This device's address on the local network
pub fn lan_address() -> Option<String> {
    let best = if_addrs::get_if_addrs()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|iface| match iface.ip() {
            std::net::IpAddr::V4(ip) => lan_score(&iface.name, ip).map(|score| (score, ip)),
            std::net::IpAddr::V6(_) => None,
        })
        .max_by_key(|(score, _)| *score);
    if let Some((_, ip)) = best {
        return Some(ip.to_string());
    }
    // Fallback: the interface of the default route (connecting a UDP socket sends nothing)
    let socket = std::net::UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect("8.8.8.8:80").ok()?;
    let ip = socket.local_addr().ok()?.ip();
    (!ip.is_loopback() && !ip.is_unspecified()).then(|| ip.to_string())
}

#[cfg(test)]
mod tests {
    #[test]
    fn parses_tps() {
        assert_eq!(
            super::parse_tps("[12:00:00 INFO]: TPS from last 1m, 5m, 15m: 19.5, 20.0, 20.0"),
            Some(19.5)
        );
        assert_eq!(
            super::parse_tps("TPS from last 1m, 5m, 15m: *20.0, *20.0, *20.0"),
            Some(20.0)
        );
    }

    #[test]
    fn parses_cpu_ticks() {
        // Process names can contain spaces and parentheses
        let stat = "1234 (java (main) x) S 1 1234 1234 0 -1 4194560 100 0 0 0 250 75 0 0 20 0 42";
        assert_eq!(super::parse_cpu_ticks(stat), Some(325));
        assert_eq!(super::parse_cpu_ticks("garbage"), None);
    }

    #[test]
    fn measures_own_process() {
        let (_, mem, _, total) = super::process_tree_usage(std::process::id());
        assert!(mem > 0 && total > 0);
    }
}

#[cfg(test)]
mod lan_tests {
    use std::net::Ipv4Addr;

    #[test]
    fn prefers_wifi_over_vpn() {
        assert_eq!(
            super::lan_score("CloudflareWARP", Ipv4Addr::new(172, 16, 0, 2)),
            None
        );
        assert_eq!(
            super::lan_score("vEthernet (WSL)", Ipv4Addr::new(172, 25, 208, 1)),
            None
        );
        assert_eq!(super::lan_score("Wi-Fi", Ipv4Addr::new(8, 8, 8, 8)), None);
        assert!(
            super::lan_score("wlan0", Ipv4Addr::new(192, 168, 8, 6))
                > super::lan_score("rmnet0", Ipv4Addr::new(10, 0, 0, 5))
        );
    }

    #[test]
    #[ignore]
    fn print_lan_address() {
        for iface in if_addrs::get_if_addrs().unwrap() {
            println!("{} {}", iface.name, iface.ip());
        }
        println!("=> {:?}", super::lan_address());
    }
}
