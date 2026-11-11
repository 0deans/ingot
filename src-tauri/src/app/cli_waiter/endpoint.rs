//! Endpoint discovery file persistence, same-PID delegation filtering, and shutdown cleanup.

use super::protocol::WaiterEndpointInfo;
use std::io::{self, Write};
use std::path::Path;

pub(super) fn write_endpoint_file(path: &Path, info: &WaiterEndpointInfo) -> io::Result<()> {
    let Some(parent) = path.parent() else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "endpoint file path has no parent directory",
        ));
    };
    std::fs::create_dir_all(parent)?;
    let encoded = serde_json::to_vec(info).map_err(io::Error::other)?;
    let mut staged = tempfile::NamedTempFile::new_in(parent)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = staged
            .as_file()
            .set_permissions(std::fs::Permissions::from_mode(0o600));
    }
    staged.write_all(&encoded)?;
    staged.flush()?;
    staged.persist(path).map_err(|err| err.error)?;
    Ok(())
}

/// Reads the endpoint file for delegation, skipping when missing, unreadable, malformed,
/// or owned by `current_pid` so a running instance never delegates to itself.
pub(super) fn read_delegation_endpoint(
    path: &Path,
    current_pid: u32,
) -> Option<WaiterEndpointInfo> {
    let Ok(bytes) = std::fs::read(path) else {
        return None;
    };
    let Ok(info) = serde_json::from_slice::<WaiterEndpointInfo>(&bytes) else {
        return None;
    };
    if info.pid == current_pid {
        return None;
    }
    Some(info)
}

/// Removes the endpoint file only if it is corrupt or still owned by `expected_pid`.
pub(super) fn cleanup_endpoint_file_at(path: &Path, expected_pid: u32) {
    let Ok(bytes) = std::fs::read(path) else {
        return;
    };
    let should_remove = match serde_json::from_slice::<WaiterEndpointInfo>(&bytes) {
        Ok(info) => info.pid == expected_pid,
        Err(_) => true,
    };
    if should_remove {
        let _ = std::fs::remove_file(path);
    }
}
