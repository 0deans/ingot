use std::{
    future::Future,
    pin::Pin,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

pub(super) type Sleep<'a> = Pin<Box<dyn Future<Output = ()> + Send + 'a>>;
pub(crate) trait ProtocolClock: Send + Sync {
    fn elapsed(&self) -> Duration;
    fn unix_seconds(&self) -> u64;
    fn sleep(&self, duration: Duration) -> Sleep<'_>;
}

pub(super) struct SystemClock {
    started: Instant,
}
impl Default for SystemClock {
    fn default() -> Self {
        Self {
            started: Instant::now(),
        }
    }
}
impl ProtocolClock for SystemClock {
    fn elapsed(&self) -> Duration {
        self.started.elapsed()
    }
    fn unix_seconds(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|time| time.as_secs())
            .unwrap_or(0)
    }
    fn sleep(&self, duration: Duration) -> Sleep<'_> {
        Box::pin(tokio::time::sleep(duration))
    }
}
