//! Remembers the games and servers Ingot started, so they can be found again after
//! Ingot closes: games are re-attached, servers left running without a console are
//! offered to be stopped.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Mutex;
use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System};
use tauri::{AppHandle, Manager, Runtime};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    pub id: String,
    pub pid: u32,
    /// When Ingot started it (unix seconds), for playtime
    pub started_at: u64,
    /// When the OS says the process started: pids get reused, this tells them apart
    pub proc_start: u64,
}

/// A server that keeps Ingot from simply quitting
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct BusyServer {
    pub server_id: String,
    pub name: String,
    /// Sleeping: no process to stop, but it can only wake while Ingot runs
    pub sleeping: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct QuitRequest {
    pub servers: Vec<BusyServer>,
    /// Games keep running when Ingot quits, and are picked up again when it opens
    #[specta(type = i32)]
    pub games: u32,
}

/// A server still running from before Ingot last closed: its console is gone
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct LeftoverServer {
    pub server_id: String,
    pub name: String,
    #[specta(type = i32)]
    pub pid: u32,
    #[specta(type = i32)]
    pub started_at: u64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Kind {
    Game,
    Server,
}

#[derive(Default, Serialize, Deserialize)]
struct Registry {
    #[serde(default)]
    games: Vec<Entry>,
    #[serde(default)]
    servers: Vec<Entry>,
}

impl Registry {
    fn list(&mut self, kind: Kind) -> &mut Vec<Entry> {
        match kind {
            Kind::Game => &mut self.games,
            Kind::Server => &mut self.servers,
        }
    }
}

static LOCK: Mutex<()> = Mutex::new(());

fn path<R: Runtime>(app: &AppHandle<R>) -> Option<PathBuf> {
    app.path().app_data_dir().ok().map(|d| d.join("running.json"))
}

fn read(path: &PathBuf) -> Registry {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn update<R: Runtime>(app: &AppHandle<R>, f: impl FnOnce(&mut Registry)) {
    let Some(path) = path(app) else { return };
    let _guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let mut registry = read(&path);
    f(&mut registry);
    let Ok(data) = serde_json::to_string_pretty(&registry) else { return };
    let tmp = path.with_extension("json.tmp");
    if std::fs::write(&tmp, data).is_ok() {
        let _ = std::fs::rename(&tmp, &path);
    }
}

/// Remembers a process Ingot just started
pub fn add<R: Runtime>(app: &AppHandle<R>, kind: Kind, id: &str, pid: u32, started_at: u64) {
    let Some(proc_start) = process_start(pid) else { return };
    let entry = Entry { id: id.to_string(), pid, started_at, proc_start };
    update(app, |r| {
        let list = r.list(kind);
        list.retain(|e| e.id != entry.id);
        list.push(entry);
    });
}

/// Forgets a process once it has exited (only that process: the id may have been started again)
pub fn remove<R: Runtime>(app: &AppHandle<R>, kind: Kind, id: &str, pid: u32) {
    update(app, |r| r.list(kind).retain(|e| !(e.id == id && e.pid == pid)));
}

pub fn load<R: Runtime>(app: &AppHandle<R>, kind: Kind) -> Vec<Entry> {
    let Some(path) = path(app) else { return Vec::new() };
    let _guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    read(&path).list(kind).clone()
}

/// When the OS says a process started (unix seconds), if it is running
pub fn process_start(pid: u32) -> Option<u64> {
    let pid = Pid::from_u32(pid);
    let mut sys = System::new();
    sys.refresh_processes_specifics(ProcessesToUpdate::Some(&[pid]), true, ProcessRefreshKind::nothing());
    sys.process(pid).map(|p| p.start_time())
}

/// Whether the process in the entry is still running (and is still that process)
pub fn is_alive(entry: &Entry) -> bool {
    process_start(entry.pid).is_some_and(|start| start.abs_diff(entry.proc_start) <= 2)
}

/// Ends a process and everything it started, without waiting for it to save
pub fn kill_tree(pid: u32) {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        let _ = std::process::Command::new("taskkill")
            .args(["/F", "/T", "/PID", &pid.to_string()])
            .creation_flags(0x08000000) // CREATE_NO_WINDOW
            .output();
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = std::process::Command::new("kill")
            .args(["-9", &pid.to_string()])
            .output();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn knows_its_own_process() {
        let pid = std::process::id();
        let start = process_start(pid).expect("this process is running");
        let entry = Entry { id: "x".into(), pid, started_at: 0, proc_start: start };
        assert!(is_alive(&entry));
        // Same pid, another start time: a different process that reused the pid
        let reused = Entry { proc_start: start.saturating_sub(3600), ..entry };
        assert!(!is_alive(&reused));
    }
}
