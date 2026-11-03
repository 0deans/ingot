use super::models::{
    InstanceCrashEvent, InstanceLogFileContent, InstanceLogFileSummary, LogError, LogFileKind,
};
use super::parser::parse_log_document;
use crate::version_change::crash::{crash_report, find_suspect_jars};
use flate2::read::GzDecoder;
use std::fs;
use std::io::Read;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

const MAX_READ_BYTES: u64 = 16 * 1024 * 1024;
const MAX_PARSED_LINES: usize = 10_000;

pub fn list_instance_log_files(
    instance_dir: &Path,
) -> Result<Vec<InstanceLogFileSummary>, LogError> {
    let mut files = Vec::new();

    let logs_dir = instance_dir.join("logs");
    if let Ok(entries) = fs::read_dir(&logs_dir) {
        for entry in entries.flatten() {
            let Ok(meta) = entry.metadata() else {
                continue;
            };
            if !meta.is_file() {
                continue;
            }
            let name = entry.file_name().to_string_lossy().into_owned();
            let lower = name.to_ascii_lowercase();
            let is_log = lower.ends_with(".log") || lower.ends_with(".log.gz");
            if !is_log {
                continue;
            }
            let modified_at = meta
                .modified()
                .ok()
                .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .unwrap_or(0);
            files.push(InstanceLogFileSummary {
                compressed: lower.ends_with(".gz"),
                file_name: name,
                kind: LogFileKind::Log,
                size_bytes: meta.len(),
                modified_at,
            });
        }
    }

    let crash_dir = instance_dir.join("crash-reports");
    if let Ok(entries) = fs::read_dir(&crash_dir) {
        for entry in entries.flatten() {
            let Ok(meta) = entry.metadata() else {
                continue;
            };
            if !meta.is_file() {
                continue;
            }
            let name = entry.file_name().to_string_lossy().into_owned();
            let lower = name.to_ascii_lowercase();
            if !lower.ends_with(".txt") && !lower.ends_with(".log") {
                continue;
            }
            let modified_at = meta
                .modified()
                .ok()
                .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .unwrap_or(0);
            files.push(InstanceLogFileSummary {
                compressed: false,
                file_name: name,
                kind: LogFileKind::CrashReport,
                size_bytes: meta.len(),
                modified_at,
            });
        }
    }

    files.sort_by(|a, b| {
        let a_latest = a.kind == LogFileKind::Log && a.file_name == "latest.log";
        let b_latest = b.kind == LogFileKind::Log && b.file_name == "latest.log";
        b_latest
            .cmp(&a_latest)
            .then_with(|| b.modified_at.cmp(&a.modified_at))
            .then_with(|| a.file_name.cmp(&b.file_name))
    });

    Ok(files)
}

pub fn read_instance_log_file(
    instance_dir: &Path,
    kind: LogFileKind,
    file_name: &str,
) -> Result<InstanceLogFileContent, LogError> {
    validate_safe_file_name(file_name)?;
    let subdir = match kind {
        LogFileKind::Log => "logs",
        LogFileKind::CrashReport => "crash-reports",
    };
    let path = instance_dir.join(subdir).join(file_name);
    let meta = fs::metadata(&path).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            LogError::NotFound(file_name.to_string())
        } else {
            LogError::Io(e)
        }
    })?;
    if !meta.is_file() {
        return Err(LogError::NotFound(file_name.to_string()));
    }
    if meta.len() > MAX_READ_BYTES {
        return Err(LogError::FileTooLarge(meta.len()));
    }

    let raw = if file_name.to_ascii_lowercase().ends_with(".gz") {
        let file = fs::File::open(&path)?;
        let decoder = GzDecoder::new(file);
        let mut limited = decoder.take(MAX_READ_BYTES + 1);
        let mut buf = Vec::new();
        limited.read_to_end(&mut buf)?;
        if buf.len() as u64 > MAX_READ_BYTES {
            return Err(LogError::FileTooLarge(buf.len() as u64));
        }
        String::from_utf8_lossy(&buf).into_owned()
    } else {
        let bytes = fs::read(&path)?;
        String::from_utf8_lossy(&bytes).into_owned()
    };

    let mut lines = parse_log_document(&raw);
    if lines.len() > MAX_PARSED_LINES {
        lines.drain(0..(lines.len() - MAX_PARSED_LINES));
    }

    let description = extract_crash_description(&raw);
    let suspects = find_suspect_jars(instance_dir, &["mods"], &raw);

    Ok(InstanceLogFileContent {
        file_name: file_name.to_string(),
        kind,
        lines,
        raw,
        suspects,
        description,
    })
}

pub fn analyze_instance_crash(
    instance_dir: &Path,
    instance_id: &str,
    instance_name: &str,
    clean_exit: bool,
    exit_code: Option<i32>,
    since: SystemTime,
) -> Option<InstanceCrashEvent> {
    let newest_report = crash_report(instance_dir, since);
    if clean_exit && newest_report.is_none() {
        return None;
    }

    let (text, excerpt, crash_report_file, description) = match newest_report {
        Some(path) => {
            let file_name = path.file_name().map(|n| n.to_string_lossy().into_owned());
            let report = fs::read_to_string(&path).unwrap_or_default();
            let desc = extract_crash_description(&report);
            let excerpt = report
                .lines()
                .skip_while(|l| !l.starts_with("Description:"))
                .take(25)
                .collect::<Vec<_>>()
                .join("\n");
            let fallback_excerpt = if excerpt.trim().is_empty() {
                report.lines().take(25).collect::<Vec<_>>().join("\n")
            } else {
                excerpt
            };
            (report, fallback_excerpt, file_name, desc)
        }
        None => {
            let log = fs::read_to_string(instance_dir.join("logs/latest.log")).unwrap_or_default();
            let lines: Vec<&str> = log.lines().collect();
            let tail = lines[lines.len().saturating_sub(400)..].join("\n");
            let excerpt = lines[lines.len().saturating_sub(25)..].join("\n");
            let desc = extract_crash_description(&tail);
            (tail, excerpt, None, desc)
        }
    };

    if text.trim().is_empty() && exit_code.is_none() {
        return None;
    }

    let suspects = find_suspect_jars(instance_dir, &["mods"], &text);
    Some(InstanceCrashEvent {
        instance_id: instance_id.to_string(),
        instance_name: instance_name.to_string(),
        exit_code,
        crash_report_file,
        description,
        excerpt: excerpt.chars().take(4000).collect(),
        suspects,
    })
}

fn extract_crash_description(text: &str) -> Option<String> {
    for line in text.lines() {
        if let Some(rest) = line.trim().strip_prefix("Description:") {
            let desc = rest.trim();
            if !desc.is_empty() {
                return Some(desc.to_string());
            }
        }
    }
    None
}

pub(crate) fn validate_safe_file_name(file_name: &str) -> Result<(), LogError> {
    let trimmed = file_name.trim();
    if trimmed.is_empty()
        || trimmed.contains('/')
        || trimmed.contains('\\')
        || trimmed.contains("..")
        || trimmed.contains(':')
    {
        return Err(LogError::InvalidFileName(file_name.to_string()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::write::GzEncoder;
    use flate2::Compression;
    use std::io::Write;

    #[test]
    fn lists_and_reads_plain_gzipped_and_crash_report_files() {
        let dir = tempfile::tempdir().unwrap();
        let logs_dir = dir.path().join("logs");
        let crash_dir = dir.path().join("crash-reports");
        fs::create_dir_all(&logs_dir).unwrap();
        fs::create_dir_all(&crash_dir).unwrap();

        fs::write(
            logs_dir.join("latest.log"),
            "[12:00:00] [main/INFO]: Starting game\n",
        )
        .unwrap();

        let gz_file = fs::File::create(logs_dir.join("2026-11-01-1.log.gz")).unwrap();
        let mut encoder = GzEncoder::new(gz_file, Compression::default());
        encoder
            .write_all(b"[11:00:00] [main/WARN]: Archived warning\n")
            .unwrap();
        encoder.finish().unwrap();

        fs::write(
            crash_dir.join("crash-2026-11-03-client.txt"),
            "---- Minecraft Crash Report ----\nDescription: Initializing game\n\njava.lang.NullPointerException: Oops\n",
        )
        .unwrap();

        let listed = list_instance_log_files(dir.path()).unwrap();
        assert_eq!(listed.len(), 3);
        assert_eq!(listed[0].file_name, "latest.log");

        let gz_content =
            read_instance_log_file(dir.path(), LogFileKind::Log, "2026-11-01-1.log.gz").unwrap();
        assert_eq!(gz_content.lines.len(), 1);
        assert_eq!(gz_content.lines[0].message, "Archived warning");

        let crash_content = read_instance_log_file(
            dir.path(),
            LogFileKind::CrashReport,
            "crash-2026-11-03-client.txt",
        )
        .unwrap();
        assert_eq!(
            crash_content.description.as_deref(),
            Some("Initializing game")
        );
    }

    #[test]
    fn rejects_path_traversal_in_log_file_names() {
        let dir = tempfile::tempdir().unwrap();
        let err =
            read_instance_log_file(dir.path(), LogFileKind::Log, "../instances.json").unwrap_err();
        assert!(matches!(err, LogError::InvalidFileName(_)));
    }
}
