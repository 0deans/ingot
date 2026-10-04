//! Application-owned Microsoft device codes, deadlines, and publication ownership.
use crate::auth::microsoft::MicrosoftDeviceCode;
use std::{
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Mutex, MutexGuard,
    },
    time::{Duration, Instant},
};

#[derive(Debug, thiserror::Error)]
pub(crate) enum LoginError {
    #[error("Microsoft sign-in state is unavailable")]
    Unavailable,
    #[error("Microsoft sign-in attempt is no longer current")]
    Stale,
    #[error("Microsoft sign-in code was not issued for this attempt")]
    UnknownCode,
    #[error("Microsoft sign-in code has expired; please start again")]
    Expired,
    #[error("Microsoft sign-in completion has already been requested; please start again")]
    AlreadyClaimed,
    #[error("Microsoft sign-in is already in progress")]
    Busy,
    #[error("Microsoft sign-in generation is exhausted; please restart the application")]
    Exhausted,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct AttemptId(u64);

pub(crate) struct StartingAttempt {
    id: AttemptId,
    started_at: Duration,
}

pub(crate) struct ClaimedAttempt<'a> {
    pub(crate) id: AttemptId,
    pub(crate) code: MicrosoftDeviceCode,
    _lease: AttemptLease<'a>,
}

pub(crate) struct AttemptLease<'a> {
    pub(crate) id: AttemptId,
    owner: &'a LoginAttempts,
}

impl Drop for AttemptLease<'_> {
    fn drop(&mut self) {
        let _guard = self
            .owner
            .issued
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if !self.owner.is_cancelled(self.id) {
            self.owner.in_flight.store(false, Ordering::SeqCst);
        }
    }
}

struct IssuedCode {
    code: MicrosoftDeviceCode,
    deadline: Duration,
    completion: Completion,
}

#[derive(PartialEq, Eq)]
enum Completion {
    Available,
    Claimed,
}

pub(crate) struct LoginAttempts {
    generation: AtomicU64,
    in_flight: AtomicBool,
    issued: Mutex<Option<IssuedCode>>,
    elapsed: Box<dyn Fn() -> Duration + Send + Sync>,
}

impl Default for LoginAttempts {
    fn default() -> Self {
        let origin = Instant::now();
        Self::with_clock(move || origin.elapsed())
    }
}

/// Holds the synchronous publication boundary, never a provider request or await.
pub(crate) struct Publication<'a> {
    _issued: MutexGuard<'a, Option<IssuedCode>>,
}

impl LoginAttempts {
    pub(super) fn with_clock(elapsed: impl Fn() -> Duration + Send + Sync + 'static) -> Self {
        Self {
            generation: AtomicU64::new(0),
            in_flight: AtomicBool::new(false),
            issued: Mutex::new(None),
            elapsed: Box::new(elapsed),
        }
    }

    #[cfg(test)]
    pub(crate) fn begin(&self) -> Result<StartingAttempt, LoginError> {
        self.begin_with(|| Ok::<(), LoginError>(()))
    }

    pub(crate) fn begin_with<E: From<LoginError>>(
        &self,
        reset: impl FnOnce() -> Result<(), E>,
    ) -> Result<StartingAttempt, E> {
        let mut issued = self.issued.lock().map_err(|_| LoginError::Unavailable)?;
        // MAX is an exhausted, cancelled generation; never reuse an issued ID.
        let previous = self
            .generation
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |id| {
                id.checked_add(1).filter(|next| *next < u64::MAX)
            })
            .map_err(|_| LoginError::Exhausted)?;
        *issued = None;
        self.in_flight.store(false, Ordering::SeqCst);
        reset()?;
        Ok(StartingAttempt {
            id: AttemptId(previous + 1),
            started_at: (self.elapsed)(),
        })
    }

    pub(crate) fn issued(
        &self,
        attempt: StartingAttempt,
        code: MicrosoftDeviceCode,
    ) -> Result<MicrosoftDeviceCode, LoginError> {
        let mut issued = self.issued.lock().map_err(|_| LoginError::Unavailable)?;
        if self.is_cancelled(attempt.id) {
            return Err(LoginError::Stale);
        }
        // Conservatively include time spent obtaining the code in its lifetime.
        let deadline = attempt
            .started_at
            .checked_add(Duration::from_secs(u64::from(code.expires_in)))
            .ok_or(LoginError::Expired)?;
        remaining_seconds(deadline, (self.elapsed)())?;
        *issued = Some(IssuedCode {
            code: code.clone(),
            deadline,
            completion: Completion::Available,
        });
        Ok(code)
    }

    pub(crate) fn claim(
        &self,
        supplied: &MicrosoftDeviceCode,
    ) -> Result<ClaimedAttempt<'_>, LoginError> {
        let mut issued = self.issued.lock().map_err(|_| LoginError::Unavailable)?;
        let issued = issued.as_mut().ok_or(LoginError::UnknownCode)?;
        if !same_code(&issued.code, supplied) {
            return Err(LoginError::UnknownCode);
        }
        if issued.completion == Completion::Claimed {
            return Err(LoginError::AlreadyClaimed);
        }
        let remaining = remaining_seconds(issued.deadline, (self.elapsed)())?;
        // Failure or dropping the polling future also consumes the attempt: the
        // provider may have redeemed the code already. Retry with a fresh code.
        issued.completion = Completion::Claimed;
        let mut code = issued.code.clone();
        code.expires_in = remaining;
        let id = AttemptId(self.generation.load(Ordering::SeqCst));
        self.in_flight.store(true, Ordering::SeqCst);
        Ok(ClaimedAttempt {
            id,
            code,
            _lease: AttemptLease { id, owner: self },
        })
    }

    pub(crate) fn publication(&self, attempt: AttemptId) -> Result<Publication<'_>, LoginError> {
        let issued = self.issued.lock().map_err(|_| LoginError::Unavailable)?;
        if self.is_cancelled(attempt) {
            return Err(LoginError::Stale);
        }
        // Expiry applies to redemption. A redeemed session can finish its exchange
        // or await account ownership after the deadline if it is still current.
        Ok(Publication { _issued: issued })
    }

    #[cfg(test)]
    pub(crate) fn invalidate(&self) {
        let _ = self.invalidate_with(|| Ok::<(), LoginError>(()));
    }

    pub(crate) fn invalidate_with<E>(
        &self,
        reset: impl FnOnce() -> Result<(), E>,
    ) -> Result<(), E> {
        // Cancellation must invalidate callbacks even after state was poisoned.
        // Other operations report poison instead of trying to resume sign-in.
        let mut issued = self
            .issued
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        let _ = self
            .generation
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |id| {
                Some(id.saturating_add(1))
            });
        *issued = None;
        self.in_flight.store(false, Ordering::SeqCst);
        reset()
    }

    #[cfg(test)]
    pub(crate) fn current(&self) -> AttemptId {
        AttemptId(self.generation.load(Ordering::SeqCst))
    }

    pub(crate) fn is_cancelled(&self, attempt: AttemptId) -> bool {
        self.generation.load(Ordering::SeqCst) != attempt.0
    }
}

#[path = "login/resume.rs"]
mod resume;

fn remaining_seconds(deadline: Duration, now: Duration) -> Result<u32, LoginError> {
    let remaining = deadline
        .checked_sub(now)
        .ok_or(LoginError::Expired)?
        .as_secs();
    if remaining == 0 {
        return Err(LoginError::Expired);
    }
    u32::try_from(remaining).map_err(|_| LoginError::Expired)
}

fn same_code(issued: &MicrosoftDeviceCode, supplied: &MicrosoftDeviceCode) -> bool {
    issued.device_code == supplied.device_code
        && issued.user_code == supplied.user_code
        && issued.verification_uri == supplied.verification_uri
        && issued.interval == supplied.interval
        && issued.expires_in == supplied.expires_in
}

#[cfg(test)]
#[path = "login/tests.rs"]
mod tests;
