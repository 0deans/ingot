use super::models::HorizonError;
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

const SESSION_EXPIRY: Duration = Duration::from_secs(120);

#[derive(Serialize)]
struct HorizonSessionPayload<'a> {
    name: &'a str,
    uuid: &'a str,
    access_token: &'a str,
    user_type: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    xuid: Option<&'a str>,
}

pub struct HorizonSessionGuard {
    path: PathBuf,
}

impl HorizonSessionGuard {
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for HorizonSessionGuard {
    fn drop(&mut self) {
        if self.path.exists() {
            let _ = fs::remove_file(&self.path);
        }
    }
}

pub fn create_session_file(
    instance_dir: &Path,
    username: &str,
    uuid: &str,
    access_token: &str,
    xuid: Option<&str>,
) -> Result<HorizonSessionGuard, HorizonError> {
    let sessions_dir = instance_dir.join("sessions");
    fs::create_dir_all(&sessions_dir)?;

    cleanup_expired_sessions(&sessions_dir);

    let session_filename = format!("session-{}.json", uuid::Uuid::new_v4());
    let session_path = sessions_dir.join(session_filename);

    let payload = HorizonSessionPayload {
        name: username,
        uuid,
        access_token,
        user_type: "msa",
        xuid,
    };

    let content = serde_json::to_string_pretty(&payload)?;
    fs::write(&session_path, content)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = fs::Permissions::from_mode(0o600);
        let _ = fs::set_permissions(&session_path, perms);
    }

    Ok(HorizonSessionGuard { path: session_path })
}

pub fn cleanup_expired_sessions(sessions_dir: &Path) {
    let Ok(entries) = fs::read_dir(sessions_dir) else {
        return;
    };
    let now = SystemTime::now();

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() {
            let is_session_file = path
                .file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|name| name.starts_with("session-") && name.ends_with(".json"));

            if is_session_file {
                if let Ok(metadata) = path.metadata() {
                    if let Ok(modified) = metadata.modified() {
                        if let Ok(age) = now.duration_since(modified) {
                            if age > SESSION_EXPIRY {
                                let _ = fs::remove_file(&path);
                            }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_and_guard_session() {
        let temp_dir = tempfile::tempdir().unwrap();
        let guard = create_session_file(
            temp_dir.path(),
            "Steve",
            "00000000-0000-0000-0000-000000000000",
            "dummy_token",
            Some("12345678"),
        )
        .unwrap();

        assert!(guard.path().exists());
        let content = fs::read_to_string(guard.path()).unwrap();
        assert!(content.contains("\"name\": \"Steve\""));
        assert!(content.contains("\"user_type\": \"msa\""));

        let path = guard.path().to_path_buf();
        drop(guard);
        assert!(!path.exists());
    }
}
