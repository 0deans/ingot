use super::models::{LaunchError, RunningInstanceSummary};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ActiveChild {
    pub(crate) pid: u32,
    pub(crate) started_at: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProcessEntry {
    Starting { reservation_id: u64 },
    Running(ActiveChild),
}

/// RAII reservation for a game process currently starting up.
/// If dropped before `commit` is called (due to error, timeout, or cancellation),
/// the reservation is cleaned up so the instance is not left permanently locked.
/// Each reservation carries a unique `reservation_id` so that cancelling a launch
/// does not allow a stale future to commit or clear a newer reservation.
pub struct StartingGuard {
    pub(crate) running: Arc<Mutex<HashMap<String, ProcessEntry>>>,
    pub(crate) instance_id: String,
    pub(crate) reservation_id: u64,
    pub(crate) committed: bool,
}

impl StartingGuard {
    /// Verifies that this reservation is still active and owned by this guard.
    /// If cancellation removed the reservation or another launch replaced it, this returns Err.
    pub fn check_active(&self) -> Result<(), LaunchError> {
        let lock = self
            .running
            .lock()
            .map_err(|e| LaunchError::Other(e.to_string()))?;
        match lock.get(&self.instance_id) {
            Some(ProcessEntry::Starting { reservation_id }) if *reservation_id == self.reservation_id => {
                Ok(())
            }
            _ => Err(LaunchError::Other("Launch reservation was cancelled".to_string())),
        }
    }

    /// Commits the starting process to the Running state, matching the exact reservation identity.
    pub fn commit(mut self, pid: u32, started_at: u64) -> Result<(), LaunchError> {
        self.committed = true;
        let mut lock = self
            .running
            .lock()
            .map_err(|e| LaunchError::Other(e.to_string()))?;
        match lock.get(&self.instance_id) {
            Some(ProcessEntry::Starting { reservation_id }) if *reservation_id == self.reservation_id => {
                lock.insert(
                    self.instance_id.clone(),
                    ProcessEntry::Running(ActiveChild { pid, started_at }),
                );
                Ok(())
            }
            _ => Err(LaunchError::Other(
                "Launch reservation is no longer active; cannot commit".to_string(),
            )),
        }
    }
}

impl Drop for StartingGuard {
    fn drop(&mut self) {
        if !self.committed {
            if let Ok(mut lock) = self.running.lock() {
                if let Some(ProcessEntry::Starting { reservation_id }) = lock.get(&self.instance_id) {
                    if *reservation_id == self.reservation_id {
                        lock.remove(&self.instance_id);
                    }
                }
            }
        }
    }
}

#[derive(Clone, Default)]
pub struct ProcessManager {
    running: Arc<Mutex<HashMap<String, ProcessEntry>>>,
    next_reservation_id: Arc<AtomicU64>,
}

impl ProcessManager {
    pub fn new() -> Self {
        Self {
            running: Arc::new(Mutex::new(HashMap::new())),
            next_reservation_id: Arc::new(AtomicU64::new(1)),
        }
    }

    pub async fn reserve_starting(&self, instance_id: &str) -> Result<StartingGuard, LaunchError> {
        let mut lock = self
            .running
            .lock()
            .map_err(|e| LaunchError::Other(e.to_string()))?;
        if lock.contains_key(instance_id) {
            return Err(LaunchError::AlreadyRunning);
        }
        let res_id = self.next_reservation_id.fetch_add(1, Ordering::SeqCst);
        lock.insert(instance_id.to_string(), ProcessEntry::Starting { reservation_id: res_id });
        Ok(StartingGuard {
            running: self.running.clone(),
            instance_id: instance_id.to_string(),
            reservation_id: res_id,
            committed: false,
        })
    }

    pub async fn is_running(&self, instance_id: &str) -> bool {
        let lock = self.running.lock().unwrap_or_else(|e| e.into_inner());
        lock.contains_key(instance_id)
    }

    pub async fn is_starting(&self, instance_id: &str) -> bool {
        let lock = self.running.lock().unwrap_or_else(|e| e.into_inner());
        matches!(lock.get(instance_id), Some(ProcessEntry::Starting { .. }))
    }

    pub async fn get_running_instances(&self) -> Vec<RunningInstanceSummary> {
        let lock = self.running.lock().unwrap_or_else(|e| e.into_inner());
        lock.iter()
            .filter_map(|(id, entry)| match entry {
                ProcessEntry::Running(child) => Some(RunningInstanceSummary {
                    instance_id: id.clone(),
                    pid: child.pid,
                    started_at: child.started_at,
                }),
                ProcessEntry::Starting { .. } => None,
            })
            .collect()
    }

    pub async fn register(&self, instance_id: &str, pid: u32, started_at: u64) {
        let mut lock = self.running.lock().unwrap_or_else(|e| e.into_inner());
        lock.insert(
            instance_id.to_string(),
            ProcessEntry::Running(ActiveChild { pid, started_at }),
        );
    }

    pub async fn remove_if_pid_matches(&self, instance_id: &str, pid: u32) -> (bool, bool) {
        let mut lock = self.running.lock().unwrap_or_else(|e| e.into_inner());
        let matches = match lock.get(instance_id) {
            Some(ProcessEntry::Running(child)) => child.pid == pid,
            _ => false,
        };
        let removed = if matches {
            lock.remove(instance_id);
            true
        } else {
            false
        };
        let is_empty = lock.is_empty();
        (removed, is_empty)
    }

    pub async fn kill_instance(&self, instance_id: &str) -> Result<(), String> {
        let child_pid = {
            let mut lock = self.running.lock().map_err(|e| e.to_string())?;
            match lock.remove(instance_id) {
                Some(ProcessEntry::Running(child)) => Some(child.pid),
                Some(ProcessEntry::Starting { .. }) => None,
                None => return Err(format!("Instance {} is not running", instance_id)),
            }
        };
        if let Some(pid) = child_pid {
            #[cfg(target_os = "windows")]
            {
                let _ = std::process::Command::new("taskkill")
                    .args(["/F", "/T", "/PID", &pid.to_string()])
                    .output();
            }
            #[cfg(not(target_os = "windows"))]
            {
                let _ = std::process::Command::new("kill")
                    .args(["-9", &pid.to_string()])
                    .output();
            }
        }
        Ok(())
    }
}
