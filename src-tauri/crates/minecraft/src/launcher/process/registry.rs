use super::super::models::{LaunchError, RunningInstanceSummary};
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

    pub(super) fn commit_with_proc_start(
        mut self,
        pid: u32,
        started_at: u64,
        proc_start: Option<u64>,
    ) -> Result<(), LaunchError> {
        self.committed = true;
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

/// RAII reservation preventing concurrent launches or kills during post-exit cleanup or maintenance.
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
            if matches!(
                entries.get(&self.instance_id),
                Some(ProcessEntry::Maintenance { reservation_id })
                    if *reservation_id == self.reservation_id
            ) {
                entries.remove(&self.instance_id);
                removed = true;
            }
        }
        if removed {
            self.state_changed.notify_waiters();
        }
    }
}

/// Pure in-memory reservation and state registry for instance lifecycles.
#[derive(Clone, Default)]
pub(super) struct ProcessRegistry {
    pub(super) running: Arc<Mutex<HashMap<String, ProcessEntry>>>,
    next_reservation_id: Arc<AtomicU64>,
    state_changed: Arc<tokio::sync::Notify>,
}

impl ProcessRegistry {
    pub(super) fn new() -> Self {
        Self {
            running: Arc::new(Mutex::new(HashMap::new())),
            next_reservation_id: Arc::new(AtomicU64::new(1)),
            state_changed: Arc::new(tokio::sync::Notify::new()),
        }
    }

    pub(super) fn reserve_starting(&self, instance_id: &str) -> Result<StartingGuard, LaunchError> {
        let mut lock = self
            .running
            .lock()
            .map_err(|_| LaunchError::RegistryPoisoned)?;
        if lock.contains_key(instance_id) {
            return Err(LaunchError::AlreadyRunning);
        }
        let reservation_id = self.next_reservation_id.fetch_add(1, Ordering::SeqCst);
        lock.insert(
            instance_id.to_string(),
            ProcessEntry::Starting { reservation_id },
        );
        Ok(StartingGuard {
            running: self.running.clone(),
            state_changed: self.state_changed.clone(),
            instance_id: instance_id.to_string(),
            reservation_id,
            committed: false,
        })
    }

    pub(super) fn reserve_maintenance(
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
            state_changed: self.state_changed.clone(),
            instance_id: instance_id.to_string(),
            reservation_id,
        })
    }

    pub(super) fn is_running(&self, instance_id: &str) -> bool {
        let lock = self.running.lock().unwrap_or_else(|e| e.into_inner());
        lock.contains_key(instance_id)
    }

    pub(super) fn has_active_entries(&self) -> bool {
        !self
            .running
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .is_empty()
    }

    pub(super) fn is_starting(&self, instance_id: &str) -> bool {
        let lock = self.running.lock().unwrap_or_else(|e| e.into_inner());
        matches!(lock.get(instance_id), Some(ProcessEntry::Starting { .. }))
    }

    pub(super) fn get_running_instances(&self) -> Vec<RunningInstanceSummary> {
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

    pub(super) fn set_running(
        &self,
        instance_id: &str,
        pid: u32,
        started_at: u64,
        proc_start: Option<u64>,
    ) {
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

    pub(super) fn adopt_if_vacant(
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

    pub(super) fn claim_exit(&self, instance_id: &str, pid: u32) -> Option<MaintenanceGuard> {
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

    pub(super) fn is_reservation_active(&self, instance_id: &str, expected_id: u64) -> bool {
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

    pub(super) async fn wait_for_reservation_end(&self, instance_id: &str, reservation_id: u64) {
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

    pub(super) fn remove_if_pid_matches(&self, instance_id: &str, pid: u32) -> (bool, bool) {
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

    pub(super) fn check_running_identity(
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

    pub(super) fn claim_cancel_or_kill_reservation(
        &self,
        instance_id: &str,
        expected_id: u64,
    ) -> Result<Option<ActiveChild>, LaunchError> {
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
        }
        Ok(active_child)
    }

    pub(super) fn claim_kill_target(
        &self,
        instance_id: &str,
    ) -> Result<Option<ActiveChild>, LaunchError> {
        let (active_child, removed_starting) = {
            let mut lock = self
                .running
                .lock()
                .map_err(|_| LaunchError::RegistryPoisoned)?;
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
        Ok(active_child)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commit_with_proc_start_records_birth_identity_and_preserves_reservation_id() {
        let registry = ProcessRegistry::new();
        let guard = registry.reserve_starting("inst").unwrap();
        let res_id = guard.reservation_id();

        guard.commit_with_proc_start(700, 1234, Some(9988)).unwrap();

        let expected = ActiveChild {
            pid: 700,
            started_at: 1234,
            proc_start: Some(9988),
        };
        registry.check_running_identity("inst", expected).unwrap();
        assert!(registry.is_reservation_active("inst", res_id));

        let wrong_birth = ActiveChild {
            proc_start: Some(1111),
            ..expected
        };
        assert!(matches!(
            registry.check_running_identity("inst", wrong_birth),
            Err(LaunchError::ReservationCancelled)
        ));
    }

    #[test]
    fn adopt_if_vacant_preserves_existing_entry_and_stored_birth_time() {
        let registry = ProcessRegistry::new();
        assert!(registry.adopt_if_vacant("inst", 50, 100, Some(42)));
        assert!(!registry.adopt_if_vacant("inst", 51, 101, Some(43)));

        registry
            .check_running_identity(
                "inst",
                ActiveChild {
                    pid: 50,
                    started_at: 100,
                    proc_start: Some(42),
                },
            )
            .unwrap();
    }

    #[test]
    fn claim_kill_target_distinguishes_missing_starting_running_and_maintenance() {
        let registry = ProcessRegistry::new();
        assert!(matches!(
            registry.claim_kill_target("inst"),
            Err(LaunchError::NotRunning(_))
        ));

        let _maint = registry.reserve_maintenance("inst").unwrap();
        assert!(matches!(
            registry.claim_kill_target("inst"),
            Err(LaunchError::Maintenance)
        ));
        drop(_maint);

        let starting = registry.reserve_starting("inst").unwrap();
        assert!(registry.claim_kill_target("inst").unwrap().is_none());
        assert!(matches!(
            starting.check_active(),
            Err(LaunchError::ReservationCancelled)
        ));

        let starting = registry.reserve_starting("inst").unwrap();
        starting.commit_with_proc_start(88, 200, Some(55)).unwrap();
        assert_eq!(
            registry.claim_kill_target("inst").unwrap(),
            Some(ActiveChild {
                pid: 88,
                started_at: 200,
                proc_start: Some(55),
            })
        );
    }
}
