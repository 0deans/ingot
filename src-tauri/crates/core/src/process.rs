use serde::{Deserialize, Serialize};

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

/// When the OS says a process started (unix seconds), if it is running
pub fn process_start(pid: u32) -> Option<u64> {
    super::process_tree::capture(pid).map(super::process_tree::ProcessIdentity::start)
}

/// Whether the process in the entry is still running (and is still that process)
pub fn is_alive(entry: &Entry) -> bool {
    process_start(entry.pid).is_some_and(|start| start.abs_diff(entry.proc_start) <= 2)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn knows_its_own_process() {
        let pid = std::process::id();
        let start = process_start(pid).expect("this process is running");
        let entry = Entry {
            id: "x".into(),
            pid,
            started_at: 0,
            proc_start: start,
        };
        assert!(is_alive(&entry));
        // Same pid, another start time: a different process that reused the pid
        let reused = Entry {
            proc_start: start.saturating_sub(3600),
            ..entry
        };
        assert!(!is_alive(&reused));
    }
}
