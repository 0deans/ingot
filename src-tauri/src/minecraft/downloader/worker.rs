use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

pub(crate) struct StagingCleanupGuard {
    pub(crate) path: PathBuf,
    pub(crate) committed: bool,
}

impl Drop for StagingCleanupGuard {
    fn drop(&mut self) {
        if !self.committed && self.path.exists() {
            let _ = std::fs::remove_file(&self.path);
        }
    }
}

pub(crate) struct WorkerTaskGuard<T: Send + 'static> {
    pub(crate) set: tokio::task::JoinSet<T>,
    staging_path: PathBuf,
    completed: Arc<AtomicBool>,
}

impl<T: Send + 'static> WorkerTaskGuard<T> {
    pub(crate) fn new(staging_path: PathBuf) -> Self {
        Self {
            set: tokio::task::JoinSet::new(),
            staging_path,
            completed: Arc::new(AtomicBool::new(false)),
        }
    }

    pub(crate) fn spawn<F>(&mut self, task: F)
    where
        F: std::future::Future<Output = T> + Send + 'static,
    {
        self.set.spawn(task);
    }

    pub(crate) async fn shutdown(&mut self) {
        self.set.shutdown().await;
    }

    pub(crate) fn disarm(&mut self) {
        self.completed.store(true, Ordering::SeqCst);
    }
}

impl<T: Send + 'static> Drop for WorkerTaskGuard<T> {
    fn drop(&mut self) {
        if !self.completed.load(Ordering::SeqCst) {
            self.set.abort_all();
            let mut set = std::mem::take(&mut self.set);
            let staging_path = self.staging_path.clone();
            if let Ok(handle) = tokio::runtime::Handle::try_current() {
                handle.spawn(async move {
                    while set.join_next().await.is_some() {}
                    if staging_path.exists() {
                        let _ = tokio::fs::remove_file(&staging_path).await;
                    }
                });
            }
        }
    }
}
