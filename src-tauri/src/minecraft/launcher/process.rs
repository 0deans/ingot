use super::models::{LaunchError, RunningInstanceSummary};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ActiveChild {
    pub(crate) pid: u32,
    pub(crate) started_at: u64,
    // Launch timestamps are display data; only the OS birth identity authorizes termination.
    pub(crate) proc_start: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProcessEntry {
    Starting { reservation_id: u64 },
    Running(ActiveChild),
    Maintenance { reservation_id: u64 },
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
        let proc_start = crate::running::process_start(pid);
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
                    ProcessEntry::Running(ActiveChild {
                        pid,
                        started_at,
                        proc_start,
                    }),
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
            if let Ok(mut lock) = self.running.lock() {
                if let Some(ProcessEntry::Starting { reservation_id }) = lock.get(&self.instance_id)
                {
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
            instance_id: instance_id.to_string(),
            reservation_id: res_id,
            committed: false,
        })
    }

    pub(crate) fn reserve_maintenance(
        &self,
        instance_id: &str,
    ) -> Result<MaintenanceGuard, LaunchError> {
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
            instance_id: instance_id.to_string(),
            reservation_id,
        })
    }

    pub async fn is_running(&self, instance_id: &str) -> bool {
        let lock = self.running.lock().unwrap_or_else(|e| e.into_inner());
        lock.contains_key(instance_id)
    }

    pub(crate) fn has_active_entries(&self) -> bool {
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
                ProcessEntry::Running(child) => Some(RunningInstanceSummary {
                    instance_id: id.clone(),
                    pid: child.pid,
                    started_at: child.started_at,
                }),
                ProcessEntry::Starting { .. } | ProcessEntry::Maintenance { .. } => None,
            })
            .collect()
    }

    pub async fn register(&self, instance_id: &str, pid: u32, started_at: u64) {
        let proc_start = crate::running::process_start(pid);
        let mut lock = self.running.lock().unwrap_or_else(|e| e.into_inner());
        lock.insert(
            instance_id.to_string(),
            ProcessEntry::Running(ActiveChild {
                pid,
                started_at,
                proc_start,
            }),
        );
    }

    #[cfg(test)]
    pub(crate) fn register_if_vacant(&self, instance_id: &str, pid: u32, started_at: u64) -> bool {
        self.adopt_if_vacant(
            instance_id,
            pid,
            started_at,
            crate::running::process_start(pid),
        )
    }

    pub(crate) fn adopt_if_vacant(
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
            ProcessEntry::Running(ActiveChild {
                pid,
                started_at,
                proc_start,
            }),
        );
        true
    }

    pub(crate) fn claim_exit(&self, instance_id: &str, pid: u32) -> Option<MaintenanceGuard> {
        let mut entries = self
            .running
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let owns_process = matches!(entries.get(instance_id), Some(ProcessEntry::Running(child)) if child.pid == pid);
        if !owns_process {
            return None;
        }
        let reservation_id = self.next_reservation_id.fetch_add(1, Ordering::SeqCst);
        entries.insert(
            instance_id.to_owned(),
            ProcessEntry::Maintenance { reservation_id },
        );
        Some(MaintenanceGuard {
            entries: self.running.clone(),
            instance_id: instance_id.to_owned(),
            reservation_id,
        })
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

    fn check_running_identity(
        &self,
        instance_id: &str,
        child: ActiveChild,
    ) -> Result<(), LaunchError> {
        let entries = self
            .running
            .lock()
            .map_err(|_| LaunchError::RegistryPoisoned)?;
        if entries.get(instance_id) != Some(&ProcessEntry::Running(child)) {
            return Err(LaunchError::ReservationCancelled);
        }
        Ok(())
    }

    pub async fn kill_instance(&self, instance_id: &str) -> Result<(), LaunchError> {
        self.kill_instance_with(instance_id, super::termination::terminate)
            .await
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
        let active_child = {
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
                Some(ProcessEntry::Running(child)) => Some(child),
                Some(ProcessEntry::Starting { .. }) => {
                    lock.remove(instance_id);
                    None
                }
                None => return Err(LaunchError::NotRunning(instance_id.to_string())),
                Some(ProcessEntry::Maintenance { .. }) => return Err(LaunchError::Maintenance),
            }
        };
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

pub(crate) struct MaintenanceGuard {
    entries: Arc<Mutex<HashMap<String, ProcessEntry>>>,
    instance_id: String,
    reservation_id: u64,
}

impl Drop for MaintenanceGuard {
    fn drop(&mut self) {
        let mut entries = self
            .entries
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if matches!(entries.get(&self.instance_id), Some(ProcessEntry::Maintenance { reservation_id }) if *reservation_id == self.reservation_id)
        {
            entries.remove(&self.instance_id);
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
