//! One application-owned lock covers each complete read/modify/write operation.
use std::{
    fs, io,
    path::{Path, PathBuf},
    sync::Mutex,
};

use super::{
    persistence::{Filesystem, Persistence},
    policy, AccountError, AccountProfile,
};

pub(crate) struct AccountRepository {
    path: PathBuf,
    lock: Mutex<()>,
}

impl AccountRepository {
    pub(crate) fn new(path: PathBuf) -> Self {
        Self {
            path,
            lock: Mutex::new(()),
        }
    }

    pub(crate) fn load(&self) -> Result<Vec<AccountProfile>, AccountError> {
        let _guard = self.lock.lock().map_err(|_| AccountError::LockPoisoned)?;
        self.load_unlocked()
    }

    /// The closure must be synchronous and contain no provider/vault operations.
    pub(crate) fn update<T>(
        &self,
        change: impl FnOnce(&mut Vec<AccountProfile>) -> Result<T, AccountError>,
    ) -> Result<T, AccountError> {
        self.update_with(change, &Filesystem)
    }

    pub(crate) fn activate(
        &self,
        mut profile: AccountProfile,
    ) -> Result<AccountProfile, AccountError> {
        profile.is_active = true;
        self.update(|accounts| {
            for account in accounts.iter_mut() {
                account.is_active = false;
            }
            if let Some(existing) = accounts.iter_mut().find(|account| account.id == profile.id) {
                *existing = profile.clone();
            } else {
                accounts.push(profile.clone());
            }
            Ok(profile)
        })
    }

    /// Patch fresh metadata; an account removed during provider I/O is not recreated.
    pub(crate) fn update_profile(
        &self,
        id: &str,
        change: impl FnOnce(&mut AccountProfile),
    ) -> Result<(), AccountError> {
        self.update(|accounts| {
            let account = accounts
                .iter_mut()
                .find(|account| account.id == id)
                .ok_or(policy::AccountPolicyError::NotFound)?;
            change(account);
            Ok(())
        })
    }

    fn update_with<T>(
        &self,
        change: impl FnOnce(&mut Vec<AccountProfile>) -> Result<T, AccountError>,
        persistence: &impl Persistence,
    ) -> Result<T, AccountError> {
        let _guard = self.lock.lock().map_err(|_| AccountError::LockPoisoned)?;
        let mut accounts = self.load_unlocked()?;
        let result = change(&mut accounts)?;
        let bytes = serde_json::to_vec_pretty(&accounts)?;
        let parent = self
            .path
            .parent()
            .filter(|path| !path.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        fs::create_dir_all(parent)
            .map_err(|source| self.io_error("create directory for", source))?;
        let mut staged = persistence
            .stage(parent)
            .map_err(|source| self.io_error("stage", source))?;
        persistence
            .write(&mut staged, &bytes)
            .map_err(|source| self.io_error("write", source))?;
        persistence
            .flush(&staged)
            .map_err(|source| self.io_error("flush", source))?;
        persistence
            .replace(staged, &self.path)
            .map_err(|source| self.io_error("replace", source))?;
        Ok(result)
    }

    fn load_unlocked(&self) -> Result<Vec<AccountProfile>, AccountError> {
        let bytes = match fs::read(&self.path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(source) => return Err(self.io_error("read", source)),
        };
        serde_json::from_slice(&bytes).map_err(|source| AccountError::Decode {
            path: self.path.clone(),
            source,
        })
    }

    fn io_error(&self, operation: &'static str, source: io::Error) -> AccountError {
        AccountError::Io {
            operation,
            path: self.path.clone(),
            source,
        }
    }
}

#[cfg(test)]
mod tests;
