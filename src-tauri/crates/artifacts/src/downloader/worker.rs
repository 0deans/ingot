use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

pub struct StagingCleanupGuard {
    pub path: PathBuf,
    pub committed: bool,
}

impl Drop for StagingCleanupGuard {
    fn drop(&mut self) {
        if !self.committed && self.path.exists() {
            let _ = std::fs::remove_file(&self.path);
        }
    }
}

pub struct WorkerTaskGuard<T: Send + 'static> {
    pub set: tokio::task::JoinSet<T>,
    staging_path: PathBuf,
    completed: Arc<AtomicBool>,
}

pub type WorkerPool<T> = WorkerTaskGuard<T>;

impl<T: Send + 'static> WorkerTaskGuard<T> {
    pub fn new(staging_path: PathBuf) -> Self {
        Self {
            set: tokio::task::JoinSet::new(),
            staging_path,
            completed: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn spawn<F>(&mut self, task: F)
    where
        F: std::future::Future<Output = T> + Send + 'static,
    {
        self.set.spawn(task);
    }

    pub async fn shutdown(&mut self) {
        self.set.shutdown().await;
    }

    pub fn disarm(&mut self) {
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
