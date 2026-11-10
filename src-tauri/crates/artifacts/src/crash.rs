//! The first start after a version change: if it crashes, find which mods or plugins the
//! crash report (or the log) blames, so the user can turn them off or undo the change.

use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use super::models::{backup_dir, ContentKind, Journal, TargetKind, TransactionPhase, JOURNAL};

#[derive(Debug, thiserror::Error)]
pub enum CrashError {
    #[error("No version change has been applied")]
    NoVersionChange,
    #[error("Version change transaction was in phase {0:?}, not committed")]
    IncompleteTransaction(TransactionPhase),
    #[error("File {0} is already disabled")]
    AlreadyDisabled(String),
    #[error("Unexpected file name: {0}")]
    InvalidFileName(String),
    #[error("Unexpected path: {0}")]
    InvalidPath(String),
    #[error("File I/O error during {operation} at {path:?}: {source}")]
    FileIo {
        operation: &'static str,
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("Cannot decode recovery journal")]
    JournalDecode(#[from] serde_json::Error),
    #[error("Failed to {operation} recovery journal at {path:?}: {source}")]
    JournalIo {
        operation: &'static str,
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct CrashSuspect {
    pub title: String,
    pub folder: String,
    pub file_name: String,
}

/// Sent when the first start after a version change fails
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct VersionChangeCrash {
    pub target_kind: TargetKind,
    pub target_id: String,
    /// Instance or server name
    pub name: String,
    pub from_game_version: String,
    pub to_game_version: String,
    pub suspects: Vec<CrashSuspect>,
    /// The start of the crash report, or the end of the log
    pub excerpt: String,
}

pub type CrashReport = VersionChangeCrash;

/// A change was applied and hasn't started successfully yet
pub fn first_start_pending(root: &Path) -> bool {
    read_journal(root).is_some_and(|j| !j.first_start_checked)
}

/// The first start after the change went fine: nothing to watch any more
pub fn first_start_ok(root: &Path) {
    if let Some(mut journal) = read_journal(root).filter(|j| !j.first_start_checked) {
        journal.first_start_checked = true;
        let _ = write_journal_typed(root, &journal);
    }
}

pub fn read_journal(root: &Path) -> Option<Journal> {
    let path = backup_dir(root).join(JOURNAL);
    let raw = fs::read_to_string(&path).ok()?;
    serde_json::from_str::<Journal>(&raw).ok()
}

pub fn write_journal_typed(root: &Path, journal: &Journal) -> Result<(), CrashError> {
    use std::io::Write;
    let backup = backup_dir(root);
    let path = backup.join(JOURNAL);
    fs::create_dir_all(&backup).map_err(|source| CrashError::JournalIo {
        operation: "create directory for",
        path: path.clone(),
        source,
    })?;
    let raw = serde_json::to_vec_pretty(journal).map_err(CrashError::JournalDecode)?;
    let mut staged =
        tempfile::NamedTempFile::new_in(&backup).map_err(|source| CrashError::JournalIo {
            operation: "stage",
            path: path.clone(),
            source,
        })?;
    staged
        .write_all(&raw)
        .map_err(|source| CrashError::JournalIo {
            operation: "write",
            path: path.clone(),
            source,
        })?;
    staged
        .as_file()
        .sync_all()
        .map_err(|source| CrashError::JournalIo {
            operation: "sync",
            path: path.clone(),
            source,
        })?;
    staged
        .persist(&path)
        .map_err(|error| CrashError::JournalIo {
            operation: "publish",
            path: path.clone(),
            source: error.error,
        })?;
    Ok(())
}

/// The newest crash report written since `since`
pub fn crash_report(root: &Path, since: SystemTime) -> Option<PathBuf> {
    fs::read_dir(root.join("crash-reports"))
        .ok()?
        .flatten()
        .filter_map(|e| Some((e.metadata().ok()?.modified().ok()?, e.path())))
        .filter(|(modified, path)| {
            *modified >= since && path.extension().is_some_and(|x| x == "txt")
        })
        .max_by_key(|(modified, _)| *modified)
        .map(|(_, path)| path)
}

/// Mod ids, names and file names a crash report or log blames
pub fn blamed(text: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut push = |s: &str| {
        let s = s
            .trim()
            .trim_matches(|c| c == '\'' || c == '"' || c == ',' || c == '.');
        if !s.is_empty() && !found.iter().any(|f: &String| f.eq_ignore_ascii_case(s)) {
            found.push(s.to_string());
        }
    };
    // Text between `start` and `end` after each `marker`
    let between = |line: &str, marker: &str, end: char| -> Option<String> {
        let rest = &line[line.find(marker)? + marker.len()..];
        Some(rest.split(end).next().unwrap_or(rest).to_string())
    };

    let mut in_suspects = false;
    for line in text.lines() {
        let trimmed = line.trim();
        // Fabric/Quilt: "Suspected Mods: Name (id), Other (id2)" or one per indented line
        if trimmed.starts_with("Suspected Mod") {
            let rest = trimmed.split_once(':').map(|(_, r)| r).unwrap_or("");
            // Listed on the next lines when the header line has none
            in_suspects = rest.trim().is_empty();
            for part in rest.split(',') {
                if let Some(id) = between(part, "(", ')') {
                    push(&id);
                }
            }
            continue;
        }
        if in_suspects {
            if line.starts_with([' ', '\t']) && !trimmed.is_empty() {
                if let Some(id) = between(trimmed, "(", ')') {
                    push(&id);
                } else {
                    push(trimmed.split(',').next().unwrap_or(trimmed));
                }
                continue;
            }
            in_suspects = false;
        }
        // Fabric: "Mod 'Name' (id) 1.0 requires ..." / "Incompatible mod set!"
        if let Some(id) = trimmed
            .find("Mod '")
            .and_then(|i| between(&trimmed[i..], "' (", ')'))
        {
            push(&id);
        }
        // Forge/NeoForge: "-- Mod loading issue for: id --", "Mod ID: 'id'", "Mod File: /path/x.jar"
        if let Some(id) = between(trimmed, "Mod loading issue for:", '-') {
            push(&id);
        }
        if let Some(id) = between(trimmed, "Mod ID: '", '\'') {
            push(&id);
        }
        for marker in ["Mod File: ", "Mod file: "] {
            if let Some(path) = between(trimmed, marker, '\n') {
                push(path.rsplit(['/', '\\']).next().unwrap_or(&path));
            }
        }
        // Mixin: "Mixin apply for mod id failed"
        if let Some(id) = between(trimmed, "Mixin apply for mod ", ' ') {
            push(&id);
        }
        // Paper: "Could not load 'plugins/X.jar'", "Error occurred while enabling Name v1.0"
        if let Some(path) = between(trimmed, "Could not load '", '\'') {
            push(path.rsplit(['/', '\\']).next().unwrap_or(&path));
        }
        if let Some(name) = between(trimmed, "Error occurred while enabling ", ' ') {
            push(&name);
        }
    }
    found
}

/// Ids and names a jar declares (Fabric, Quilt, Forge, NeoForge, Bukkit)
pub fn jar_names(path: &Path) -> Vec<String> {
    let mut names = Vec::new();
    let Ok(file) = fs::File::open(path) else {
        return names;
    };
    let Ok(mut zip) = zip::ZipArchive::new(file) else {
        return names;
    };
    let mut read = |name: &str| -> Option<String> {
        let mut text = String::new();
        zip.by_name(name).ok()?.read_to_string(&mut text).ok()?;
        Some(text)
    };
    if let Some(json) =
        read("fabric.mod.json").and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok())
    {
        names.extend(
            ["id", "name"]
                .iter()
                .filter_map(|k| json.get(k)?.as_str().map(str::to_string)),
        );
    }
    if let Some(json) =
        read("quilt.mod.json").and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok())
    {
        names.extend(
            json.pointer("/quilt_loader/id")
                .and_then(|v| v.as_str())
                .map(str::to_string),
        );
    }
    for toml in ["META-INF/neoforge.mods.toml", "META-INF/mods.toml"] {
        if let Some(doc) = read(toml).and_then(|t| t.parse::<toml_edit::DocumentMut>().ok()) {
            for table in doc
                .get("mods")
                .and_then(|m| m.as_array_of_tables())
                .into_iter()
                .flat_map(|a| a.iter())
            {
                names.extend(
                    ["modId", "displayName"]
                        .iter()
                        .filter_map(|k| table.get(k)?.as_str().map(str::to_string)),
                );
            }
        }
    }
    if let Some((name, ..)) = crate::metadata::read_raw_jar_metadata(path) {
        names.push(name);
    }
    names
}

pub fn find_suspect_jars(root: &Path, folders: &[&str], text: &str) -> Vec<CrashSuspect> {
    let blamed = blamed(text);
    if blamed.is_empty() {
        return Vec::new();
    }
    let mut suspects = Vec::new();
    for folder in folders {
        let Ok(entries) = fs::read_dir(root.join(folder)) else {
            continue;
        };
        for entry in entries.flatten() {
            let file_name = entry.file_name().to_string_lossy().into_owned();
            if !file_name.to_ascii_lowercase().ends_with(".jar") {
                continue;
            }
            let names = jar_names(&entry.path());
            let hit = blamed.iter().any(|b| {
                b.eq_ignore_ascii_case(&file_name)
                    || names.iter().any(|n| n.eq_ignore_ascii_case(b))
            });
            if hit {
                let title = names
                    .iter()
                    .rev()
                    .find(|n| n.contains(' ') || n.chars().any(char::is_uppercase))
                    .or(names.first());
                suspects.push(CrashSuspect {
                    title: title.cloned().unwrap_or_else(|| file_name.clone()),
                    folder: folder.to_string(),
                    file_name,
                });
            }
        }
    }
    suspects
}

/// After a start that failed, while a change is still unconfirmed: what went wrong.
/// `folders` are the content folders of the target (mods or plugins).
pub fn analyze(
    root: &Path,
    target_kind: TargetKind,
    target_id: &str,
    name: &str,
    folders: &[&str],
    since: SystemTime,
) -> Option<VersionChangeCrash> {
    let journal = read_journal(root).filter(|j| !j.first_start_checked)?;
    let (text, excerpt) = match crash_report(root, since).and_then(|p| fs::read_to_string(p).ok()) {
        Some(report) => {
            let excerpt = report
                .lines()
                .skip_while(|l| !l.starts_with("Description:"))
                .take(20)
                .collect::<Vec<_>>()
                .join("\n");
            (report, excerpt)
        }
        None => {
            let log = fs::read_to_string(root.join("logs/latest.log")).unwrap_or_default();
            let lines: Vec<&str> = log.lines().collect();
            let tail = lines[lines.len().saturating_sub(400)..].join("\n");
            let excerpt = lines[lines.len().saturating_sub(20)..].join("\n");
            (tail, excerpt)
        }
    };

    let suspects = find_suspect_jars(root, folders, &text);
    Some(VersionChangeCrash {
        target_kind,
        target_id: target_id.to_string(),
        name: name.to_string(),
        from_game_version: journal.from_game_version,
        to_game_version: journal.to_game_version,
        suspects,
        excerpt: excerpt.chars().take(4000).collect(),
    })
}

/// Call when the game or server stops. While a change is unconfirmed: a clean stop ends
/// the watch; a crash returns what went wrong.
pub fn after_exit(
    root: &Path,
    clean: bool,
    target_kind: TargetKind,
    target_id: &str,
    name: &str,
    folders: &[&str],
    since: SystemTime,
) -> Option<VersionChangeCrash> {
    if !first_start_pending(root) {
        return None;
    }
    if clean && crash_report(root, since).is_none() {
        first_start_ok(root);
        return None;
    }
    analyze(root, target_kind, target_id, name, folders, since)
}

fn checked_path(root: &Path, path: PathBuf) -> Result<PathBuf, CrashError> {
    if path.starts_with(root) {
        Ok(path)
    } else {
        Err(CrashError::InvalidPath(path.to_string_lossy().into_owned()))
    }
}

/// Turns off a file the crash blamed, as part of the change: undo turns it back on
pub fn disable(root: &Path, folder: &str, file_name: &str) -> Result<(), CrashError> {
    disable_with(root, folder, file_name, write_journal_typed)
}

pub fn disable_with(
    root: &Path,
    folder: &str,
    file_name: &str,
    checkpoint: impl FnOnce(&Path, &Journal) -> Result<(), CrashError>,
) -> Result<(), CrashError> {
    let mut journal = match read_journal(root) {
        Some(journal) => journal,
        None => return Err(CrashError::NoVersionChange),
    };
    if !matches!(
        journal.phase,
        TransactionPhase::Committed | TransactionPhase::Prepared
    ) {
        return Err(CrashError::IncompleteTransaction(journal.phase));
    }
    super::models::safe_rel_typed(folder).map_err(CrashError::InvalidPath)?;
    let kind = if file_name.to_ascii_lowercase().ends_with(".jar") {
        ContentKind::Mod
    } else {
        ContentKind::ResourcePack
    };
    super::models::safe_file_name_typed(file_name, kind).map_err(CrashError::InvalidFileName)?;
    let rel = format!("{folder}/{file_name}");
    let original = checked_path(root, root.join(&rel))?;
    let disabled = checked_path(root, root.join(format!("{rel}.disabled")))?;
    if disabled.exists() {
        return Err(CrashError::AlreadyDisabled(file_name.to_owned()));
    }
    fs::symlink_metadata(&original).map_err(|source| CrashError::FileIo {
        operation: "inspect file before disabling",
        path: original.clone(),
        source,
    })?;
    if !journal.disabled.contains(&rel) {
        journal.disabled.push(rel);
    }
    checkpoint(root, &journal)?;
    fs::rename(&original, &disabled).map_err(|source| CrashError::FileIo {
        operation: "disable file",
        path: original,
        source,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_blamed_mods() {
        let fabric = "Description: Mod loading\n\nA detailed walkthrough...\nSuspected Mods: Sodium (sodium), Iris (iris)\n\tStack: ...";
        assert_eq!(blamed(fabric), ["sodium", "iris"]);
        let fabric_multi = "Suspected Mod:\n\tLithium (lithium), Version: 0.15\n\tFabric API (fabric-api), Version: 0.1\nother";
        assert_eq!(blamed(fabric_multi), ["lithium", "fabric-api"]);
        let loader = " - Mod 'Mod Menu' (modmenu) 13.0.4 requires version 0.119 of fabric-api";
        assert_eq!(blamed(loader), ["modmenu"]);
        let forge = "-- Mod loading issue for: create --\nDetails:\n\tMod File: /home/x/mods/create-6.0.jar\n\tMod ID: 'create'";
        assert_eq!(blamed(forge), ["create", "create-6.0.jar"]);
        let paper = "[ERROR]: Could not load 'plugins/OldPlugin.jar' in folder 'plugins'\nError occurred while enabling Essentials v2.0 (Is it up to date?)";
        assert_eq!(blamed(paper), ["OldPlugin.jar", "Essentials"]);
    }

    #[test]
    fn failed_disable_checkpoint_leaves_original_file_untouched() {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir(root.path().join("mods")).unwrap();
        fs::write(root.path().join("mods/a.jar"), b"original").unwrap();
        let journal = Journal::default();
        write_journal_typed(root.path(), &journal).unwrap();
        let result = disable_with(root.path(), "mods", "a.jar", |_, _| {
            Err(CrashError::FileIo {
                operation: "checkpoint",
                path: root.path().to_path_buf(),
                source: std::io::Error::from(std::io::ErrorKind::PermissionDenied),
            })
        });
        assert!(matches!(result, Err(CrashError::FileIo { .. })));
        assert_eq!(
            fs::read(root.path().join("mods/a.jar")).unwrap(),
            b"original"
        );
        assert!(!root.path().join("mods/a.jar.disabled").exists());
        assert!(read_journal(root.path()).unwrap().disabled.is_empty());
    }

    #[test]
    fn missing_disable_target_does_not_add_an_unrestorable_intent() {
        let root = tempfile::tempdir().unwrap();
        write_journal_typed(root.path(), &Journal::default()).unwrap();
        let result = disable(root.path(), "mods", "missing.jar");
        assert!(matches!(result, Err(CrashError::FileIo { source, .. })
            if source.kind() == std::io::ErrorKind::NotFound));
        assert!(read_journal(root.path()).unwrap().disabled.is_empty());
    }
}
