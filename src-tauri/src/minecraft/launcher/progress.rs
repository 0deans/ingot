use super::models::LaunchProgressEvent;
use std::sync::Arc;
use std::time::{Duration, Instant};

pub struct ProgressThrottler {
    last_phase: std::sync::Mutex<String>,
    last_time: std::sync::Mutex<Instant>,
    throttle_duration: Duration,
}

impl ProgressThrottler {
    pub fn new(throttle_duration: Duration) -> Self {
        Self {
            last_phase: std::sync::Mutex::new(String::new()),
            last_time: std::sync::Mutex::new(Instant::now()),
            throttle_duration,
        }
    }

    pub fn should_emit(&self, phase: &str, current: u64, total: u64) -> bool {
        let mut phase_lock = self.last_phase.lock().unwrap();
        let mut time_lock = self.last_time.lock().unwrap();
        let phase_changed = *phase_lock != phase;
        let is_boundary = current == 0 || (total > 0 && current >= total);
        let elapsed_ok = time_lock.elapsed() >= self.throttle_duration;
        if phase_changed || is_boundary || elapsed_ok {
            *phase_lock = phase.to_string();
            *time_lock = Instant::now();
            true
        } else {
            false
        }
    }
}

pub type ProgressCallback = Arc<dyn Fn(&str, u64, u64, &str) + Send + Sync>;

pub fn make_progress_reporter<F>(instance_id: String, on_progress: F) -> ProgressCallback
where
    F: Fn(LaunchProgressEvent) + Send + Sync + 'static,
{
    let cb = Arc::new(on_progress);
    let throttler = Arc::new(ProgressThrottler::new(Duration::from_millis(80)));

    Arc::new(move |phase: &str, current: u64, total: u64, detail: &str| {
        let percentage = if total > 0 {
            (current as f32 / total as f32) * 100.0
        } else {
            0.0
        };

        if throttler.should_emit(phase, current, total) {
            cb(LaunchProgressEvent {
                instance_id: instance_id.clone(),
                phase: phase.to_string(),
                current_step: current,
                total_steps: total,
                percentage,
                detail: detail.to_string(),
            });
        }
    })
}
