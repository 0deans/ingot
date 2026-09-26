//! Live performance numbers for a running server: CPU and memory of its process tree
//! (on Android the Java server runs under PRoot, so children are included) and TPS.

use serde::{Deserialize, Serialize};
use std::sync::Mutex;
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
    let Ok(mut guard) = SYSTEM.lock() else { return (0.0, 0, 0, 0) };
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
    let (mut cpu, mut mem) = (0.0f32, 0u64);
    for (pid, process) in sys.processes() {
        // On Linux/Android every thread is listed as a task sharing its process's memory;
        // counting them would multiply the JVM's RAM by its thread count
        if process.thread_kind().is_some() {
            continue;
        }
        if in_tree(*pid) {
            cpu += process.cpu_usage();
            mem += process.memory();
        }
    }
    let cores = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1) as f32;
    (
        (cpu / cores).clamp(0.0, 100.0),
        mem,
        sys.used_memory(),
        sys.total_memory(),
    )
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

pub async fn stats(pm: &ServerProcessManager, server_id: &str, pid: u32, has_tps: bool) -> ServerStats {
    let (cpu, mem, used, total) = tokio::task::spawn_blocking(move || process_tree_usage(pid))
        .await
        .unwrap_or((0.0, 0, 0, 0));
    let tps = if has_tps {
        pm.query(server_id, "tps", |l| l.contains("TPS from last"), Duration::from_secs(2))
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

/// Interfaces that aren't the real local network (VPNs, virtual switches, containers)
const VIRTUAL_INTERFACES: [&str; 14] = [
    "warp", "vpn", "tun", "tap", "wg", "wireguard", "zerotier", "tailscale", "vethernet", "docker",
    "virtualbox", "vmware", "hyper-v", "utun",
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
        || ["wi-fi", "wifi", "wireless", "ethernet"].iter().any(|p| name.contains(p));
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
        assert_eq!(super::parse_tps("[12:00:00 INFO]: TPS from last 1m, 5m, 15m: 19.5, 20.0, 20.0"), Some(19.5));
        assert_eq!(super::parse_tps("TPS from last 1m, 5m, 15m: *20.0, *20.0, *20.0"), Some(20.0));
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
        assert_eq!(super::lan_score("CloudflareWARP", Ipv4Addr::new(172, 16, 0, 2)), None);
        assert_eq!(super::lan_score("vEthernet (WSL)", Ipv4Addr::new(172, 25, 208, 1)), None);
        assert_eq!(super::lan_score("Wi-Fi", Ipv4Addr::new(8, 8, 8, 8)), None);
        assert!(super::lan_score("wlan0", Ipv4Addr::new(192, 168, 8, 6)) > super::lan_score("rmnet0", Ipv4Addr::new(10, 0, 0, 5)));
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
