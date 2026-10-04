//! Exclusive explicit resumption; original device-code claims remain consumed.
use super::*;

impl LoginAttempts {
    pub(crate) fn busy(&self) -> bool {
        self.in_flight.load(Ordering::SeqCst)
    }

    pub(crate) fn with_guard<T, E: From<LoginError>>(
        &self,
        action: impl FnOnce() -> Result<T, E>,
    ) -> Result<T, E> {
        let _guard = self.issued.lock().map_err(|_| LoginError::Unavailable)?;
        action()
    }

    pub(crate) fn resume_with<T, E: From<LoginError>>(
        &self,
        load: impl FnOnce() -> Result<T, E>,
    ) -> Result<(AttemptLease<'_>, T), E> {
        let mut issued = self.issued.lock().map_err(|_| LoginError::Unavailable)?;
        if self.busy() {
            return Err(LoginError::Busy.into());
        }
        // Validate the ticket and recover any authoritative account intent before
        // advancing generation. Invalid requests cannot cancel a current attempt.
        let grant = load()?;
        let previous = self
            .generation
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |id| {
                id.checked_add(1).filter(|next| *next < u64::MAX)
            })
            .map_err(|_| LoginError::Exhausted)?;
        *issued = None;
        self.in_flight.store(true, Ordering::SeqCst);
        Ok((
            AttemptLease {
                id: AttemptId(previous + 1),
                owner: self,
            },
            grant,
        ))
    }

    pub(crate) fn discard_with<T, E: From<LoginError>>(
        &self,
        check: impl FnOnce() -> Result<T, E>,
        discard: impl FnOnce(T) -> Result<(), E>,
    ) -> Result<(), E> {
        let mut issued = self.issued.lock().map_err(|_| LoginError::Unavailable)?;
        let checked = check()?;
        let _ = self
            .generation
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |id| {
                Some(id.saturating_add(1))
            });
        *issued = None;
        self.in_flight.store(false, Ordering::SeqCst);
        discard(checked)
    }
}
