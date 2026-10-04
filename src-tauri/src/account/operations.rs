//! Per-account ownership spans provider I/O; the short registry lock never spans an await.
use std::{
    collections::HashMap,
    sync::{Arc, Mutex, Weak},
};
use tokio::sync::{Mutex as AsyncMutex, OwnedMutexGuard};

#[derive(Debug, thiserror::Error)]
#[error("Account operation registry lock poisoned")]
pub(crate) struct OperationError;

#[derive(Default)]
pub(crate) struct AccountOperations {
    locks: Mutex<HashMap<String, Weak<AsyncMutex<()>>>>,
}

pub(crate) struct AccountOperation {
    id: String,
    _guard: OwnedMutexGuard<()>,
}

impl AccountOperation {
    pub(crate) fn id(&self) -> &str {
        &self.id
    }
}

impl AccountOperations {
    pub(crate) async fn acquire(&self, id: &str) -> Result<AccountOperation, OperationError> {
        let lock = {
            let mut locks = self.locks.lock().map_err(|_| OperationError)?;
            locks.retain(|_, lock| lock.strong_count() > 0);
            if let Some(lock) = locks.get(id).and_then(Weak::upgrade) {
                lock
            } else {
                let lock = Arc::new(AsyncMutex::new(()));
                locks.insert(id.to_owned(), Arc::downgrade(&lock));
                lock
            }
        };
        Ok(AccountOperation {
            id: id.to_owned(),
            _guard: lock.lock_owned().await,
        })
    }
}

#[cfg(test)]
mod tests;
