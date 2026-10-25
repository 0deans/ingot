use super::{Effects, ProcessIdentity, TerminationError};
use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System};

pub(super) struct Native;

pub(super) fn capture(pid: u32) -> Option<ProcessIdentity> {
    if pid == 0 {
        return None;
    }
    let mut system = System::new();
    let native_pid = Pid::from_u32(pid);
    system.refresh_processes_specifics(
        ProcessesToUpdate::Some(&[native_pid]),
        true,
        ProcessRefreshKind::nothing(),
    );
    system.process(native_pid).and_then(|process| {
        let start = process.start_time();
        if start == 0 {
            return None;
        }
        Some(ProcessIdentity::new(pid, start))
    })
}

impl Effects for Native {
    async fn snapshot(
        &self,
        root: ProcessIdentity,
    ) -> Result<Vec<ProcessIdentity>, TerminationError> {
        tokio::task::spawn_blocking(move || snapshot(root))
            .await
            .map_err(TerminationError::Inspection)?
    }
    async fn alive(&self, identity: ProcessIdentity) -> Result<bool, TerminationError> {
        tokio::task::spawn_blocking(move || capture(identity.pid) == Some(identity))
            .await
            .map_err(TerminationError::Inspection)
    }
    async fn stop(
        &self,
        identity: ProcessIdentity,
        deadline: tokio::time::Instant,
    ) -> Result<(), TerminationError> {
        super::command::run(identity, deadline).await
    }
}

fn snapshot(root: ProcessIdentity) -> Result<Vec<ProcessIdentity>, TerminationError> {
    let mut system = System::new();
    system.refresh_processes_specifics(ProcessesToUpdate::All, true, ProcessRefreshKind::nothing());
    let Some(process) = system.process(Pid::from_u32(root.pid)) else {
        return Ok(Vec::new());
    };
    if process.start_time() != root.proc_start {
        return Ok(Vec::new());
    }
    {
        let nodes = system
            .processes()
            .iter()
            .map(|(pid, process)| Node {
                identity: ProcessIdentity::new(pid.as_u32(), process.start_time()),
                parent: process.parent().map(Pid::as_u32),
            })
            .collect::<Vec<_>>();
        descendant_order(root, &nodes)
    }
}

#[derive(Clone, Copy)]
pub(super) struct Node {
    pub(super) identity: ProcessIdentity,
    pub(super) parent: Option<u32>,
}

pub(super) fn descendant_order(
    root: ProcessIdentity,
    nodes: &[Node],
) -> Result<Vec<ProcessIdentity>, TerminationError> {
    let mut targets = vec![root];
    let mut cursor = 0;
    while let Some(parent) = targets.get(cursor).copied() {
        for node in nodes {
            if node.parent != Some(parent.pid) {
                continue;
            }
            if node.identity.proc_start == 0 {
                return Err(TerminationError::IdentityUnavailable(node.identity.pid));
            }
            // A reused parent's older descendants cannot belong to this incarnation.
            if node.identity.proc_start < parent.proc_start {
                continue;
            }
            if targets.iter().any(|target| target.pid == node.identity.pid) {
                continue;
            }
            targets.push(node.identity);
        }
        cursor += 1;
    }
    targets.reverse();
    Ok(targets)
}
