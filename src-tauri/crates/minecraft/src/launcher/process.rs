use super::models::{LaunchError, RunningInstanceSummary};
use crate::logs::{parse_log_line, InstanceLogLine, LogLevel};
use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

const MAX_INSTANCE_LOG_HISTORY: usize = 2500;

#[derive(Default)]
struct InstanceLogBuffer {
    next_seq: u64,
    last_level: Option<LogLevel>,
    lines: VecDeque<InstanceLogLine>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ActiveChild {
    pub(crate) pid: u32,
    pub(crate) started_at: u64,
    // Launch timestamps are display data; only the OS birth identity authorizes termination.
    pub(crate) proc_start: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProcessEntry {
    Starting {
        reservation_id: u64,
    },
    Running {
        child: ActiveChild,
        reservation_id: Option<u64>,
    },
    Maintenance {
        reservation_id: u64,
    },
}

/// RAII reservation for a game process currently starting up.
/// If dropped before `commit` is called (due to error, timeout, or cancellation),
/// the reservation is cleaned up so the instance is not left permanently locked.
/// Each reservation carries a unique `reservation_id` so that cancelling a launch
/// does not allow a stale future to commit or clear a newer reservation.
pub struct StartingGuard {
    pub(crate) running: Arc<Mutex<HashMap<String, ProcessEntry>>>,
    pub(crate) state_changed: Arc<tokio::sync::Notify>,
    pub(crate) instance_id: String,
    pub(crate) reservation_id: u64,
    pub(crate) committed: bool,
}

impl StartingGuard {
    pub fn reservation_id(&self) -> u64 {
        self.reservation_id
    }

    /// Verifies that this reservation is still active and owned by this guard.
    /// If cancellation removed the reservation or another launch replaced it, this returns Err.
    pub fn check_active(&self) -> Result<(), LaunchError> {
        let lock = self
            .running
            .lock()
            .map_err(|_| LaunchError::RegistryPoisoned)?;
        match lock.get(&self.instance_id) {
            Some(ProcessEntry::Starting { reservation_id })
                if *reservation_id == self.reservation_id =>
            {
                Ok(())
            }
            _ => Err(LaunchError::ReservationCancelled),
        }
    }

    /// Commits the starting process to the Running state, matching the exact reservation identity.
    pub fn commit(mut self, pid: u32, started_at: u64) -> Result<(), LaunchError> {
        self.committed = true;
        let proc_start = ingot_core::process_start(pid);
        let mut lock = self
            .running
            .lock()
            .map_err(|_| LaunchError::RegistryPoisoned)?;
        match lock.get(&self.instance_id) {
            Some(ProcessEntry::Starting { reservation_id })
                if *reservation_id == self.reservation_id =>
            {
                lock.insert(
                    self.instance_id.clone(),
                    ProcessEntry::Running {
                        child: ActiveChild {
                            pid,
                            started_at,
                            proc_start,
                        },
                        reservation_id: Some(self.reservation_id),
                    },
                );
                Ok(())
            }
            _ => Err(LaunchError::ReservationCancelled),
        }
    }
}

impl Drop for StartingGuard {
    fn drop(&mut self) {
        if !self.committed {
            let mut removed = false;
            if let Ok(mut lock) = self.running.lock() {
                if let Some(ProcessEntry::Starting { reservation_id }) = lock.get(&self.instance_id)
                {
                    if *reservation_id == self.reservation_id {
                        lock.remove(&self.instance_id);
                        removed = true;
                    }
                }
            }
            if removed {
                self.state_changed.notify_waiters();
            }
        }
    }
}

#[derive(Clone, Default)]
pub struct ProcessManager {
    running: Arc<Mutex<HashMap<String, ProcessEntry>>>,
    logs: Arc<Mutex<HashMap<String, InstanceLogBuffer>>>,
    next_reservation_id: Arc<AtomicU64>,
    state_changed: Arc<tokio::sync::Notify>,
}

impl ProcessManager {
    pub fn new() -> Self {
        Self {
            running: Arc::new(Mutex::new(HashMap::new())),
            logs: Arc::new(Mutex::new(HashMap::new())),
            next_reservation_id: Arc::new(AtomicU64::new(1)),
            state_changed: Arc::new(tokio::sync::Notify::new()),
        }
    }

    pub fn clear_instance_logs(&self, instance_id: &str) {
        let mut logs = self.logs.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(buffer) = logs.get_mut(instance_id) {
            buffer.lines.clear();
            buffer.last_level = None;
        }
    }

    pub fn append_instance_log(
        &self,
        instance_id: &str,
        raw_line: &str,
        default_level: LogLevel,
    ) -> InstanceLogLine {
        let mut logs = self.logs.lock().unwrap_or_else(|e| e.into_inner());
        let buffer = logs.entry(instance_id.to_string()).or_default();
        buffer.next_seq = buffer.next_seq.saturating_add(1);
        let parsed = parse_log_line(buffer.next_seq, raw_line, default_level, buffer.last_level);
        buffer.last_level = Some(parsed.level);
        if buffer.lines.len() >= MAX_INSTANCE_LOG_HISTORY {
            buffer.lines.pop_front();
        }
        buffer.lines.push_back(parsed.clone());
        parsed
    }

    pub fn get_instance_logs(&self, instance_id: &str) -> Vec<InstanceLogLine> {
        let logs = self.logs.lock().unwrap_or_else(|e| e.into_inner());
        logs.get(instance_id)
            .map(|b| b.lines.iter().cloned().collect())
            .unwrap_or_default()
    }

    pub async fn reserve_starting(&self, instance_id: &str) -> Result<StartingGuard, LaunchError> {
        let mut lock = self
            .running
            .lock()
            .map_err(|_| LaunchError::RegistryPoisoned)?;
        if lock.contains_key(instance_id) {
            return Err(LaunchError::AlreadyRunning);
        }
        let res_id = self.next_reservation_id.fetch_add(1, Ordering::SeqCst);
        lock.insert(
            instance_id.to_string(),
            ProcessEntry::Starting {
                reservation_id: res_id,
            },
        );
        Ok(StartingGuard {
            running: self.running.clone(),
            state_changed: self.state_changed.clone(),
            instance_id: instance_id.to_string(),
            reservation_id: res_id,
            committed: false,
        })
    }

    pub fn reserve_maintenance(&self, instance_id: &str) -> Result<MaintenanceGuard, LaunchError> {
        let mut entries = self
            .running
            .lock()
            .map_err(|_| LaunchError::RegistryPoisoned)?;
        if entries.contains_key(instance_id) {
            return Err(LaunchError::AlreadyRunning);
        }
        let reservation_id = self.next_reservation_id.fetch_add(1, Ordering::SeqCst);
        entries.insert(
            instance_id.to_string(),
            ProcessEntry::Maintenance { reservation_id },
        );
        Ok(MaintenanceGuard {
            entries: self.running.clone(),
            state_changed: self.state_changed.clone(),
            instance_id: instance_id.to_string(),
            reservation_id,
        })
    }

    pub async fn is_running(&self, instance_id: &str) -> bool {
        let lock = self.running.lock().unwrap_or_else(|e| e.into_inner());
        lock.contains_key(instance_id)
    }

    pub fn has_active_entries(&self) -> bool {
        !self
            .running
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .is_empty()
    }

    pub async fn is_starting(&self, instance_id: &str) -> bool {
        let lock = self.running.lock().unwrap_or_else(|e| e.into_inner());
        matches!(lock.get(instance_id), Some(ProcessEntry::Starting { .. }))
    }

    pub async fn get_running_instances(&self) -> Vec<RunningInstanceSummary> {
        let lock = self.running.lock().unwrap_or_else(|e| e.into_inner());
        lock.iter()
            .filter_map(|(id, entry)| match entry {
                ProcessEntry::Running { child, .. } => Some(RunningInstanceSummary {
                    instance_id: id.clone(),
                    pid: child.pid,
                    started_at: child.started_at,
                }),
                ProcessEntry::Starting { .. } | ProcessEntry::Maintenance { .. } => None,
            })
            .collect()
    }

    pub async fn register(&self, instance_id: &str, pid: u32, started_at: u64) {
        let proc_start = ingot_core::process_start(pid);
        let mut lock = self.running.lock().unwrap_or_else(|e| e.into_inner());
        lock.insert(
            instance_id.to_string(),
            ProcessEntry::Running {
                child: ActiveChild {
                    pid,
                    started_at,
                    proc_start,
                },
                reservation_id: None,
            },
        );
    }

    #[cfg(test)]
    pub(crate) fn register_if_vacant(&self, instance_id: &str, pid: u32, started_at: u64) -> bool {
        self.adopt_if_vacant(instance_id, pid, started_at, ingot_core::process_start(pid))
    }

    pub fn adopt_if_vacant(
        &self,
        instance_id: &str,
        pid: u32,
        started_at: u64,
        proc_start: Option<u64>,
    ) -> bool {
        let mut entries = self
            .running
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if entries.contains_key(instance_id) {
            return false;
        }
        entries.insert(
            instance_id.to_owned(),
            ProcessEntry::Running {
                child: ActiveChild {
                    pid,
                    started_at,
                    proc_start,
                },
                reservation_id: None,
            },
        );
        true
    }

    pub fn claim_exit(&self, instance_id: &str, pid: u32) -> Option<MaintenanceGuard> {
        let mut entries = self
            .running
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let reservation_id = match entries.get(instance_id) {
            Some(ProcessEntry::Running {
                child,
                reservation_id,
            }) if child.pid == pid => reservation_id
                .unwrap_or_else(|| self.next_reservation_id.fetch_add(1, Ordering::SeqCst)),
            _ => return None,
        };
        entries.insert(
            instance_id.to_owned(),
            ProcessEntry::Maintenance { reservation_id },
        );
        Some(MaintenanceGuard {
            entries: self.running.clone(),
            state_changed: self.state_changed.clone(),
            instance_id: instance_id.to_owned(),
            reservation_id,
        })
    }

    pub fn is_reservation_active(&self, instance_id: &str, expected_id: u64) -> bool {
        let lock = self.running.lock().unwrap_or_else(|e| e.into_inner());
        matches!(
            lock.get(instance_id),
            Some(
                ProcessEntry::Starting { reservation_id }
                | ProcessEntry::Running {
                    reservation_id: Some(reservation_id),
                    ..
                }
                | ProcessEntry::Maintenance { reservation_id }
            ) if *reservation_id == expected_id
        )
    }

    pub async fn wait_for_reservation_end(&self, instance_id: &str, reservation_id: u64) {
        loop {
            let notified = self.state_changed.notified();
            tokio::pin!(notified);
            notified.as_mut().enable();
            if !self.is_reservation_active(instance_id, reservation_id) {
                return;
            }
            notified.await;
        }
    }

    pub async fn remove_if_pid_matches(&self, instance_id: &str, pid: u32) -> (bool, bool) {
        let (removed, is_empty) = {
            let mut lock = self.running.lock().unwrap_or_else(|e| e.into_inner());
            let matches = match lock.get(instance_id) {
                Some(ProcessEntry::Running { child, .. }) => child.pid == pid,
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
        };
        if removed {
            self.state_changed.notify_waiters();
        }
        (removed, is_empty)
    }

    fn check_running_identity(
        &self,
        instance_id: &str,
        child: ActiveChild,
    ) -> Result<(), LaunchError> {
        let entries = self
            .running
            .lock()
            .map_err(|_| LaunchError::RegistryPoisoned)?;
        let matches = matches!(
            entries.get(instance_id),
            Some(ProcessEntry::Running {
                child: current,
                ..
            }) if *current == child
        );
        if !matches {
            return Err(LaunchError::ReservationCancelled);
        }
        Ok(())
    }

    pub async fn kill_instance(&self, instance_id: &str) -> Result<(), LaunchError> {
        self.kill_instance_with(instance_id, super::termination::terminate)
            .await
    }

    pub async fn cancel_or_kill_reservation(
        &self,
        instance_id: &str,
        expected_id: u64,
    ) -> Result<(), LaunchError> {
        self.cancel_or_kill_reservation_with(
            instance_id,
            expected_id,
            super::termination::terminate,
        )
        .await
    }

    async fn cancel_or_kill_reservation_with<F, Fut>(
        &self,
        instance_id: &str,
        expected_id: u64,
        terminate: F,
    ) -> Result<(), LaunchError>
    where
        F: FnOnce(ActiveChild) -> Fut + Send + 'static,
        Fut: std::future::Future<Output = Result<(), super::termination::TerminationError>>
            + Send
            + 'static,
    {
        let (active_child, removed_starting) = {
            let mut lock = self
                .running
                .lock()
                .map_err(|_| LaunchError::RegistryPoisoned)?;
            match lock.get(instance_id).copied() {
                Some(ProcessEntry::Starting { reservation_id })
                    if reservation_id == expected_id =>
                {
                    lock.remove(instance_id);
                    (None, true)
                }
                Some(ProcessEntry::Running {
                    child,
                    reservation_id: Some(reservation_id),
                }) if reservation_id == expected_id => (Some(child), false),
                _ => (None, false),
            }
        };
        if removed_starting {
            self.state_changed.notify_waiters();
            return Ok(());
        }
        if let Some(child) = active_child {
            let manager = self.clone();
            let instance_id = instance_id.to_owned();
            tokio::spawn(async move {
                manager.check_running_identity(&instance_id, child)?;
                let result = terminate(child).await;
                if let Err(error) = &result {
                    eprintln!("[Launcher] Game termination failed: {error}");
                }
                result.map_err(LaunchError::Termination)
            })
            .await??;
        }
        Ok(())
    }

    async fn kill_instance_with<F, Fut>(
        &self,
        instance_id: &str,
        terminate: F,
    ) -> Result<(), LaunchError>
    where
        F: FnOnce(ActiveChild) -> Fut + Send + 'static,
        Fut: std::future::Future<Output = Result<(), super::termination::TerminationError>>
            + Send
            + 'static,
    {
        let (active_child, removed_starting) = {
            let mut lock = self
                .running
                .lock()
                .map_err(|_| LaunchError::RegistryPoisoned)?;
            if matches!(
                lock.get(instance_id),
                Some(ProcessEntry::Maintenance { .. })
            ) {
                return Err(LaunchError::Maintenance);
            }
            match lock.get(instance_id).copied() {
                Some(ProcessEntry::Running { child, .. }) => (Some(child), false),
                Some(ProcessEntry::Starting { .. }) => {
                    lock.remove(instance_id);
                    (None, true)
                }
                None => return Err(LaunchError::NotRunning(instance_id.to_string())),
                Some(ProcessEntry::Maintenance { .. }) => return Err(LaunchError::Maintenance),
            }
        };
        if removed_starting {
            self.state_changed.notify_waiters();
        }
        if let Some(child) = active_child {
            let manager = self.clone();
            let instance_id = instance_id.to_owned();
            // The worker owns termination if the caller stops waiting for it.
            tokio::spawn(async move {
                manager.check_running_identity(&instance_id, child)?;
                let result = terminate(child).await;
                if let Err(error) = &result {
                    eprintln!("[Launcher] Game termination failed: {error}");
                }
                result.map_err(LaunchError::Termination)
            })
            .await??;
        }
        Ok(())
    }
}

pub struct MaintenanceGuard {
    entries: Arc<Mutex<HashMap<String, ProcessEntry>>>,
    state_changed: Arc<tokio::sync::Notify>,
    instance_id: String,
    reservation_id: u64,
}

impl Drop for MaintenanceGuard {
    fn drop(&mut self) {
        let mut removed = false;
        {
            let mut entries = self
                .entries
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            if matches!(entries.get(&self.instance_id), Some(ProcessEntry::Maintenance { reservation_id }) if *reservation_id == self.reservation_id)
            {
                entries.remove(&self.instance_id);
                removed = true;
            }
        }
        if removed {
            self.state_changed.notify_waiters();
        }
    }
}

#[cfg(test)]
mod maintenance_tests {
    use super::*;
    #[tokio::test]
    async fn maintenance_excludes_start_and_releases_on_drop() {
        let manager = ProcessManager::new();
        let guard = manager.reserve_maintenance("instance").unwrap();
        assert!(manager.reserve_starting("instance").await.is_err());
        assert!(manager.reserve_maintenance("instance").is_err());
        assert!(manager.kill_instance("instance").await.is_err());
        drop(guard);
        let starting = manager.reserve_starting("instance").await.unwrap();
        assert!(manager.reserve_maintenance("instance").is_err());
        drop(starting);
        assert!(manager.reserve_maintenance("instance").is_ok());
    }

    #[tokio::test]
    async fn stale_exit_cannot_claim_replacement_or_release_its_ownership() {
        let manager = ProcessManager::new();
        assert!(manager.register_if_vacant("instance", 20, 100));
        assert!(!manager.register_if_vacant("instance", 30, 200));
        assert!(manager.claim_exit("instance", 10).is_none());
        let exit = manager.claim_exit("instance", 20).unwrap();
        assert!(manager.reserve_starting("instance").await.is_err());
        assert!(manager.reserve_maintenance("instance").is_err());
        assert!(manager.kill_instance("instance").await.is_err());
        drop(exit);
        assert!(manager.reserve_starting("instance").await.is_ok());
    }
}

#[cfg(test)]
#[path = "process_tests.rs"]
mod error_tests;
