mod logs;
mod registry;

use super::models::{LaunchError, RunningInstanceSummary};
use crate::logs::{InstanceLogLine, LogLevel};
use logs::InstanceLogStore;
use registry::ProcessRegistry;

pub(crate) use registry::ActiveChild;
pub use registry::{MaintenanceGuard, StartingGuard};

impl StartingGuard {
    /// Commits the starting process to the Running state, matching the exact reservation identity
    /// and capturing the OS process birth identity for safe later termination.
    pub fn commit(self, pid: u32, started_at: u64) -> Result<(), LaunchError> {
        let proc_start = ingot_core::process_start(pid);
        self.commit_with_proc_start(pid, started_at, proc_start)
    }
}

/// Coordinates running-instance reservations, OS child identity registration/adoption,
/// process termination workers, and bounded per-instance log buffers.
#[derive(Clone, Default)]
pub struct ProcessManager {
    registry: ProcessRegistry,
    logs: InstanceLogStore,
}

impl ProcessManager {
    pub fn new() -> Self {
        Self {
            registry: ProcessRegistry::new(),
            logs: InstanceLogStore::new(),
        }
    }

    pub fn clear_instance_logs(&self, instance_id: &str) {
        self.logs.clear(instance_id);
    }

    pub fn append_instance_log(
        &self,
        instance_id: &str,
        raw_line: &str,
        default_level: LogLevel,
    ) -> InstanceLogLine {
        self.logs.append(instance_id, raw_line, default_level)
    }

    pub fn get_instance_logs(&self, instance_id: &str) -> Vec<InstanceLogLine> {
        self.logs.get(instance_id)
    }

    pub async fn reserve_starting(&self, instance_id: &str) -> Result<StartingGuard, LaunchError> {
        self.registry.reserve_starting(instance_id)
    }

    pub fn reserve_maintenance(&self, instance_id: &str) -> Result<MaintenanceGuard, LaunchError> {
        self.registry.reserve_maintenance(instance_id)
    }

    pub async fn is_running(&self, instance_id: &str) -> bool {
        self.registry.is_running(instance_id)
    }

    pub fn has_active_entries(&self) -> bool {
        self.registry.has_active_entries()
    }

    pub async fn is_starting(&self, instance_id: &str) -> bool {
        self.registry.is_starting(instance_id)
    }

    pub async fn get_running_instances(&self) -> Vec<RunningInstanceSummary> {
        self.registry.get_running_instances()
    }

    pub async fn register(&self, instance_id: &str, pid: u32, started_at: u64) {
        let proc_start = ingot_core::process_start(pid);
        self.registry
            .set_running(instance_id, pid, started_at, proc_start);
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
        self.registry
            .adopt_if_vacant(instance_id, pid, started_at, proc_start)
    }

    pub fn claim_exit(&self, instance_id: &str, pid: u32) -> Option<MaintenanceGuard> {
        self.registry.claim_exit(instance_id, pid)
    }

    pub fn is_reservation_active(&self, instance_id: &str, expected_id: u64) -> bool {
        self.registry
            .is_reservation_active(instance_id, expected_id)
    }

    pub async fn wait_for_reservation_end(&self, instance_id: &str, reservation_id: u64) {
        self.registry
            .wait_for_reservation_end(instance_id, reservation_id)
            .await;
    }

    pub async fn remove_if_pid_matches(&self, instance_id: &str, pid: u32) -> (bool, bool) {
        self.registry.remove_if_pid_matches(instance_id, pid)
    }

    #[cfg(test)]
    fn check_running_identity(
        &self,
        instance_id: &str,
        child: ActiveChild,
    ) -> Result<(), LaunchError> {
        self.registry.check_running_identity(instance_id, child)
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
        if let Some(child) = self
            .registry
            .claim_cancel_or_kill_reservation(instance_id, expected_id)?
        {
            self.spawn_checked_termination(instance_id, child, terminate)
                .await?;
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
        if let Some(child) = self.registry.claim_kill_target(instance_id)? {
            self.spawn_checked_termination(instance_id, child, terminate)
                .await?;
        }
        Ok(())
    }

    async fn spawn_checked_termination<F, Fut>(
        &self,
        instance_id: &str,
        child: ActiveChild,
        terminate: F,
    ) -> Result<(), LaunchError>
    where
        F: FnOnce(ActiveChild) -> Fut + Send + 'static,
        Fut: std::future::Future<Output = Result<(), super::termination::TerminationError>>
            + Send
            + 'static,
    {
        let registry = self.registry.clone();
        let instance_id = instance_id.to_owned();
        // The worker owns termination if the caller stops waiting for it.
        tokio::spawn(async move {
            registry.check_running_identity(&instance_id, child)?;
            let result = terminate(child).await;
            if let Err(error) = &result {
                eprintln!("[Launcher] Game termination failed: {error}");
            }
            result.map_err(LaunchError::Termination)
        })
        .await??;
        Ok(())
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
