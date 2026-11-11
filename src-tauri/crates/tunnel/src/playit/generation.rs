use super::PlayitManager;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Generation(pub u64);

impl Generation {
    pub const INITIAL: Self = Self(0);

    pub fn next(self) -> Self {
        Self(self.0.saturating_add(1))
    }

    pub fn as_u64(self) -> u64 {
        self.0
    }
}

pub(crate) struct StartingGuard {
    generation: Generation,
    manager: PlayitManager,
    defused: bool,
}

impl StartingGuard {
    pub(crate) fn new(generation: Generation, manager: PlayitManager) -> Self {
        Self {
            generation,
            manager,
            defused: false,
        }
    }

    pub(crate) fn defuse(&mut self) {
        self.defused = true;
    }
}

impl Drop for StartingGuard {
    fn drop(&mut self) {
        if self.defused {
            return;
        }
        let manager = self.manager.clone();
        let generation = self.generation;
        tokio::spawn(async move {
            // Failed cleanup remains registered and visible for an explicit retry.
            if let Err(error) = manager.stop_generation(Some(generation)).await {
                eprintln!("[Playit] Cancelled start cleanup failed: {error}");
            }
        });
    }
}
