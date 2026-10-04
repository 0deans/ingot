//! Each application owns its sign-in generation; cancellation invalidates existing waits.
use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Default)]
pub(crate) struct LoginAttempts(AtomicU64);

impl LoginAttempts {
    pub(crate) fn invalidate(&self) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }

    pub(crate) fn current(&self) -> u64 {
        self.0.load(Ordering::SeqCst)
    }

    pub(crate) fn is_cancelled(&self, attempt: u64) -> bool {
        self.current() != attempt
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cancellation_and_new_attempts_invalidate_only_their_own_application() {
        let first = LoginAttempts::default();
        let second = LoginAttempts::default();
        first.invalidate();
        let attempt = first.current();
        assert!(!first.is_cancelled(attempt));
        second.invalidate();
        second.invalidate();
        assert!(!first.is_cancelled(attempt));
        first.invalidate();
        assert!(first.is_cancelled(attempt));
        assert!(!first.is_cancelled(first.current()));
    }
}
