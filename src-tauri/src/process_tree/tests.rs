use super::*;
use std::collections::{HashMap, VecDeque};
use std::error::Error;
use std::future::{pending, Future};
use std::sync::{Arc, Mutex};

fn status(code: u32) -> ExitStatus {
    #[cfg(windows)]
    {
        use std::os::windows::process::ExitStatusExt;
        ExitStatus::from_raw(code)
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        ExitStatus::from_raw((code as i32) << 8)
    }
}

struct FakeEffects {
    targets: Vec<ProcessIdentity>,
    originals: Mutex<HashMap<u32, u64>>,
    stopped: Arc<Mutex<Vec<u32>>>,
    fail_pid: Option<u32>,
    hang_snapshot: bool,
    hang_inspection: bool,
    hang_stop: bool,
    fail_inspection_after_stop: bool,
}

impl FakeEffects {
    fn new(targets: Vec<ProcessIdentity>) -> Self {
        let originals = targets
            .iter()
            .map(|identity| (identity.pid, identity.proc_start))
            .collect();
        Self {
            targets,
            originals: Mutex::new(originals),
            stopped: Arc::default(),
            fail_pid: None,
            hang_snapshot: false,
            hang_inspection: false,
            hang_stop: false,
            fail_inspection_after_stop: false,
        }
    }
}

impl Effects for FakeEffects {
    async fn snapshot(&self, _: ProcessIdentity) -> Result<Vec<ProcessIdentity>, TerminationError> {
        if self.hang_snapshot {
            return pending().await;
        }
        Ok(self.targets.clone())
    }
    async fn alive(&self, identity: ProcessIdentity) -> Result<bool, TerminationError> {
        if self.hang_inspection {
            return pending().await;
        }
        if self.fail_inspection_after_stop && !self.stopped.lock().unwrap().is_empty() {
            return Err(TerminationError::IdentityUnavailable(identity.pid));
        }
        Ok(self.originals.lock().unwrap().get(&identity.pid) == Some(&identity.proc_start))
    }
    async fn stop(
        &self,
        identity: ProcessIdentity,
        _: tokio::time::Instant,
    ) -> Result<(), TerminationError> {
        if self.hang_stop {
            return pending().await;
        }
        self.stopped.lock().unwrap().push(identity.pid);
        self.originals.lock().unwrap().remove(&identity.pid);
        if self.fail_pid == Some(identity.pid) {
            return Err(TerminationError::CommandFailed {
                pid: identity.pid,
                status: status(5),
            });
        }
        Ok(())
    }
}

#[tokio::test]
async fn observed_children_are_stopped_before_root_and_stale_identities_are_skipped() {
    let root = ProcessIdentity::new(10, 100);
    let child = ProcessIdentity::new(11, 101);
    let stale = ProcessIdentity::new(12, 102);
    let effects = FakeEffects::new(vec![child, stale, root]);
    effects.originals.lock().unwrap().insert(stale.pid, 999);
    let stopped = effects.stopped.clone();
    terminate_with(root, effects).await.unwrap();
    assert_eq!(*stopped.lock().unwrap(), vec![11, 10]);
}

#[tokio::test]
async fn child_failure_does_not_skip_root_and_original_cause_is_retained() {
    let root = ProcessIdentity::new(10, 100);
    let child = ProcessIdentity::new(11, 101);
    let mut effects = FakeEffects::new(vec![child, root]);
    effects.fail_pid = Some(child.pid);
    let stopped = effects.stopped.clone();
    assert!(matches!(
        terminate_with(root, effects).await,
        Err(TerminationError::CommandFailed { pid: 11, .. })
    ));
    assert_eq!(*stopped.lock().unwrap(), vec![11, 10]);
}

#[tokio::test(start_paused = true)]
async fn hung_snapshot_and_inspection_have_one_shared_deadline() {
    let root = ProcessIdentity::new(10, 100);
    for snapshot in [true, false] {
        let mut effects = FakeEffects::new(vec![root]);
        effects.hang_snapshot = snapshot;
        effects.hang_inspection = !snapshot;
        let started = tokio::time::Instant::now();
        let error = terminate_with(root, effects).await.unwrap_err();
        assert_eq!(started.elapsed(), OPERATION_TIMEOUT);
        match error {
            TerminationError::OperationTimeout(10) => {}
            TerminationError::Failures {
                primary,
                additional,
            } => {
                assert!(matches!(*primary, TerminationError::OperationTimeout(10)));
                assert!(matches!(
                    additional.as_slice(),
                    [TerminationError::ExitUnconfirmed(10)]
                ));
            }
            other => panic!("Unexpected deadline error: {other}"),
        }
    }
}

#[tokio::test(start_paused = true)]
async fn faulty_command_effect_cannot_extend_the_operation_indefinitely() {
    let root = ProcessIdentity::new(10, 100);
    let mut effects = FakeEffects::new(vec![root]);
    effects.hang_stop = true;
    let started = tokio::time::Instant::now();
    let error = terminate_with(root, effects).await.unwrap_err();
    assert_eq!(started.elapsed(), MAX_TERMINATION_DURATION);
    assert!(matches!(error, TerminationError::Failures { primary, .. }
        if matches!(*primary, TerminationError::OperationTimeout(10))));
}

#[tokio::test]
async fn dropped_caller_does_not_drop_owned_work() {
    let (started, entered) = tokio::sync::oneshot::channel();
    let (release, resume) = tokio::sync::oneshot::channel();
    let (done, completed) = tokio::sync::oneshot::channel();
    let caller = tokio::spawn(owned(async move {
        started.send(()).unwrap();
        resume.await.unwrap();
        done.send(()).unwrap();
        Ok(())
    }));
    entered.await.unwrap();
    caller.abort();
    assert!(caller.await.unwrap_err().is_cancelled());
    release.send(()).unwrap();
    completed.await.unwrap();
}

#[tokio::test]
async fn invalid_or_reused_root_never_grants_termination_authority() {
    assert!(matches!(
        terminate(ProcessIdentity::new(0, 100)).await,
        Err(TerminationError::IdentityUnavailable(0))
    ));
    let identity = capture(std::process::id()).unwrap();
    let stale = ProcessIdentity::new(identity.pid, identity.proc_start + 1);
    let targets = inspection::Native.snapshot(stale).await.unwrap();
    assert!(targets.is_empty());
}

#[test]
fn descendant_snapshot_orders_by_depth_and_rejects_unverifiable_children() {
    use inspection::{descendant_order, Node};
    let root = ProcessIdentity::new(10, 100);
    let child = ProcessIdentity::new(11, 101);
    let grandchild = ProcessIdentity::new(12, 102);
    let mut nodes = vec![
        Node {
            identity: grandchild,
            parent: Some(11),
        },
        Node {
            identity: child,
            parent: Some(10),
        },
        Node {
            identity: ProcessIdentity::new(13, 99),
            parent: Some(10),
        },
        Node {
            identity: root,
            parent: Some(12),
        },
    ];
    assert_eq!(
        descendant_order(root, &nodes).unwrap(),
        vec![grandchild, child, root]
    );
    nodes.push(Node {
        identity: ProcessIdentity::new(14, 0),
        parent: Some(10),
    });
    assert!(matches!(
        descendant_order(root, &nodes),
        Err(TerminationError::IdentityUnavailable(14))
    ));
}

enum WaitStep {
    Exit(u32),
    Fail(io::ErrorKind),
    Pending,
}

struct FakeCommand {
    waits: VecDeque<WaitStep>,
    signal: Option<io::ErrorKind>,
    signals: usize,
}

impl command::Command for FakeCommand {
    async fn wait(&mut self) -> io::Result<ExitStatus> {
        match self.waits.pop_front().unwrap_or(WaitStep::Pending) {
            WaitStep::Exit(code) => Ok(status(code)),
            WaitStep::Fail(kind) => Err(kind.into()),
            WaitStep::Pending => pending().await,
        }
    }
    fn start_kill(&mut self) -> io::Result<()> {
        self.signals += 1;
        match self.signal {
            Some(kind) => Err(kind.into()),
            None => Ok(()),
        }
    }
}

#[tokio::test]
async fn command_nonzero_status_and_spawn_failures_are_explicit() {
    let mut command = FakeCommand {
        waits: VecDeque::from([WaitStep::Exit(7)]),
        signal: None,
        signals: 0,
    };
    assert!(matches!(
        command::wait(
            &mut command,
            10,
            tokio::time::Instant::now() + OPERATION_TIMEOUT
        )
        .await,
        Err(TerminationError::CommandFailed { pid: 10, .. })
    ));
    assert_eq!(command.signals, 0);
    let temp = tempfile::tempdir().unwrap();
    let missing = tokio::process::Command::new(temp.path().join("missing-termination-command"));
    let error = command::run_command(missing, 10, tokio::time::Instant::now() + OPERATION_TIMEOUT)
        .await
        .unwrap_err();
    assert!(matches!(error, TerminationError::Spawn { pid: 10, .. }));
    assert!(error.source().unwrap().is::<io::Error>());
}

#[tokio::test(start_paused = true)]
async fn wait_failure_keeps_signal_and_reap_failures() {
    let mut command = FakeCommand {
        waits: VecDeque::from([
            WaitStep::Fail(io::ErrorKind::BrokenPipe),
            WaitStep::Fail(io::ErrorKind::PermissionDenied),
        ]),
        signal: Some(io::ErrorKind::PermissionDenied),
        signals: 0,
    };
    let error = command::wait(
        &mut command,
        10,
        tokio::time::Instant::now() + OPERATION_TIMEOUT,
    )
    .await
    .unwrap_err();
    assert!(
        matches!(error, TerminationError::Cleanup { primary, signal: Some(signal), reap: Some(reap) }
        if matches!(primary.as_ref(), TerminationError::Wait { source, .. } if source.kind() == io::ErrorKind::BrokenPipe)
            && signal.kind() == io::ErrorKind::PermissionDenied && reap.kind() == io::ErrorKind::PermissionDenied)
    );
    assert_eq!(command.signals, 1);
}

#[tokio::test(start_paused = true)]
async fn command_timeout_attempts_kill_and_reaping_with_a_separate_bound() {
    let mut command = FakeCommand {
        waits: VecDeque::from([WaitStep::Pending, WaitStep::Pending]),
        signal: None,
        signals: 0,
    };
    let started = tokio::time::Instant::now();
    let error = command::wait(&mut command, 10, started + Duration::from_secs(2))
        .await
        .unwrap_err();
    assert_eq!(started.elapsed(), Duration::from_secs(5));
    assert!(
        matches!(error, TerminationError::Cleanup { primary, signal: None, reap: Some(reap) }
        if matches!(*primary, TerminationError::CommandTimeout(10)) && reap.kind() == io::ErrorKind::TimedOut)
    );
    assert_eq!(command.signals, 1);
}

#[tokio::test]
async fn successful_reaping_keeps_the_primary_wait_failure() {
    let mut command = FakeCommand {
        waits: VecDeque::from([WaitStep::Fail(io::ErrorKind::BrokenPipe), WaitStep::Exit(0)]),
        signal: None,
        signals: 0,
    };
    assert!(
        matches!(command::wait(&mut command, 10, tokio::time::Instant::now() + OPERATION_TIMEOUT).await,
        Err(TerminationError::Wait { source, .. }) if source.kind() == io::ErrorKind::BrokenPipe)
    );
    assert_eq!(command.signals, 1);
}

#[tokio::test]
async fn owned_worker_panic_is_a_typed_failure() {
    let error = owned(async { panic!("injected worker failure") })
        .await
        .unwrap_err();
    assert!(matches!(error, TerminationError::Worker(source) if source.is_panic()));
}

#[tokio::test]
async fn later_inspection_failures_do_not_discard_the_primary_command_failure() {
    let root = ProcessIdentity::new(10, 100);
    let child = ProcessIdentity::new(11, 101);
    let mut effects = FakeEffects::new(vec![child, root]);
    effects.fail_pid = Some(child.pid);
    effects.fail_inspection_after_stop = true;
    let error = terminate_with(root, effects).await.unwrap_err();
    assert!(
        matches!(error, TerminationError::Failures { primary, additional }
        if matches!(*primary, TerminationError::CommandFailed { pid: 11, .. })
            && additional.iter().any(|failure| matches!(failure, TerminationError::IdentityUnavailable(10))))
    );
}

#[tokio::test(start_paused = true)]
async fn delayed_polling_does_not_renew_the_command_cleanup_budget() {
    let mut command = FakeCommand {
        waits: VecDeque::from([WaitStep::Pending, WaitStep::Pending]),
        signal: None,
        signals: 0,
    };
    let started = tokio::time::Instant::now();
    let mut operation = Box::pin(command::wait(
        &mut command,
        10,
        started + Duration::from_secs(2),
    ));
    std::future::poll_fn(|context| {
        assert!(operation.as_mut().poll(context).is_pending());
        std::task::Poll::Ready(())
    })
    .await;
    tokio::time::advance(Duration::from_secs(20)).await;
    let error = operation.await.unwrap_err();
    assert_eq!(started.elapsed(), Duration::from_secs(20));
    assert!(
        matches!(error, TerminationError::Cleanup { primary, reap: Some(reap), .. }
        if matches!(*primary, TerminationError::CommandTimeout(10)) && reap.kind() == io::ErrorKind::TimedOut)
    );
    assert_eq!(command.signals, 1);
}
