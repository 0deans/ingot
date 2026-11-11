use crate::logs::{parse_log_line, InstanceLogLine, LogLevel};
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};

const MAX_INSTANCE_LOG_HISTORY: usize = 2500;

#[derive(Default)]
struct InstanceLogBuffer {
    next_seq: u64,
    last_level: Option<LogLevel>,
    lines: VecDeque<InstanceLogLine>,
}

/// Thread-safe bounded ring-buffer log store partitioned by instance ID.
#[derive(Clone, Default)]
pub(super) struct InstanceLogStore {
    buffers: Arc<Mutex<HashMap<String, InstanceLogBuffer>>>,
}

impl InstanceLogStore {
    pub(super) fn new() -> Self {
        Self {
            buffers: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub(super) fn clear(&self, instance_id: &str) {
        let mut logs = self.buffers.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(buffer) = logs.get_mut(instance_id) {
            buffer.lines.clear();
            buffer.last_level = None;
        }
    }

    pub(super) fn append(
        &self,
        instance_id: &str,
        raw_line: &str,
        default_level: LogLevel,
    ) -> InstanceLogLine {
        let mut logs = self.buffers.lock().unwrap_or_else(|e| e.into_inner());
        let buffer = logs.entry(instance_id.to_string()).or_default();
        buffer.next_seq = buffer.next_seq.saturating_add(1);
        let parsed = parse_log_line(buffer.next_seq, raw_line, default_level, buffer.last_level);
        buffer.last_level = Some(parsed.level);
        if buffer.lines.len() >= MAX_INSTANCE_LOG_HISTORY {
            buffer.lines.pop_front();
        }
        buffer.lines.push_back(parsed.clone());
        parsed
    }

    pub(super) fn get(&self, instance_id: &str) -> Vec<InstanceLogLine> {
        let logs = self.buffers.lock().unwrap_or_else(|e| e.into_inner());
        logs.get(instance_id)
            .map(|buffer| buffer.lines.iter().cloned().collect())
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ring_buffer_evicts_oldest_lines_at_capacity() {
        let store = InstanceLogStore::new();
        for idx in 1..=(MAX_INSTANCE_LOG_HISTORY + 5) {
            store.append("inst-a", &format!("line {idx}"), LogLevel::Info);
        }

        let lines = store.get("inst-a");
        assert_eq!(lines.len(), MAX_INSTANCE_LOG_HISTORY);
        assert_eq!(lines.first().unwrap().seq, 6);
        assert_eq!(
            lines.last().unwrap().seq,
            (MAX_INSTANCE_LOG_HISTORY + 5) as u64
        );
        assert_eq!(lines.first().unwrap().message, "line 6");
    }

    #[test]
    fn append_carries_last_level_and_clear_resets_lines_and_level() {
        let store = InstanceLogStore::new();
        let first = store.append(
            "inst-a",
            "[12:00:00] [main/WARN]: First warning",
            LogLevel::Info,
        );
        assert_eq!(first.level, LogLevel::Warn);

        let continuation = store.append("inst-a", "    at net.minecraft.Main", LogLevel::Info);
        assert_eq!(continuation.level, LogLevel::Warn);

        store.clear("inst-a");
        assert!(store.get("inst-a").is_empty());

        let after_clear = store.append("inst-a", "plain line after clear", LogLevel::Info);
        assert_eq!(after_clear.level, LogLevel::Info);
        assert_eq!(after_clear.seq, 3);
    }

    #[test]
    fn log_buffers_are_isolated_per_instance() {
        let store = InstanceLogStore::new();
        store.append("inst-a", "alpha", LogLevel::Info);
        store.append("inst-b", "beta", LogLevel::Error);

        store.clear("inst-a");
        assert!(store.get("inst-a").is_empty());
        assert_eq!(store.get("inst-b").len(), 1);
        assert_eq!(store.get("inst-b")[0].message, "beta");
    }
}
