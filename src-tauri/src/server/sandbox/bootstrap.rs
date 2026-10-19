use super::error::SandboxError;
use super::{package, publication};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::Mutex;

pub(super) const ALPINE_BRANCH: &str = "v3.24";
pub(super) const ALPINE_VERSION: &str = "3.24.2";
pub(super) const BASE_MARKER: &str = ".ingot-rootfs";

pub fn get_sandbox_dir_in(data_dir: &Path) -> Result<PathBuf, SandboxError> {
    let sandbox_dir = data_dir.join("sandbox");
    std::fs::create_dir_all(&sandbox_dir)?;
    Ok(sandbox_dir)
}

pub fn is_rootfs_installed(rootfs_dir: &Path) -> Result<bool, SandboxError> {
    for relative in [BASE_MARKER, "sbin/apk"] {
        let path = crate::app::safe_enclosed_path(rootfs_dir, relative)
            .ok_or(SandboxError::InvalidRootfs)?;
        match std::fs::metadata(path) {
            Ok(metadata) if metadata.is_file() => {}
            Ok(_) => return Err(SandboxError::InvalidRootfs),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
            Err(error) => return Err(error.into()),
        }
    }
    Ok(true)
}

fn java_package(major: u32) -> (u32, &'static str) {
    match major {
        0..=8 => (8, "openjdk8-jre"),
        9..=11 => (11, "openjdk11-jre-headless"),
        12..=17 => (17, "openjdk17-jre-headless"),
        18..=21 => (21, "openjdk21-jre-headless"),
        _ => (25, "openjdk25-jre-headless"),
    }
}

fn find_guest_java(rootfs: &Path, major: u32) -> Result<Option<String>, SandboxError> {
    let candidates = if major == 8 {
        vec![
            "/usr/lib/jvm/java-1.8-openjdk/bin/java".into(),
            "/usr/lib/jvm/java-1.8-openjdk/jre/bin/java".into(),
        ]
    } else {
        vec![format!("/usr/lib/jvm/java-{major}-openjdk/bin/java")]
    };
    for candidate in candidates {
        // Absolute guest symlinks need not resolve on the host.
        match std::fs::symlink_metadata(rootfs.join(candidate.trim_start_matches('/'))) {
            Ok(metadata) if metadata.is_file() || metadata.file_type().is_symlink() => {
                return Ok(Some(candidate))
            }
            Ok(_) => return Err(SandboxError::InvalidRootfs),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
    }
    Ok(None)
}
/// Owns setup through completion even when a launch awaiting it is cancelled.
#[derive(Clone, Default)]
pub struct SandboxSetup {
    gate: Arc<Mutex<()>>,
    process_cleanup_failed: Arc<AtomicBool>,
}

impl SandboxSetup {
    pub async fn ensure_base_rootfs<F>(
        &self,
        data_dir: &Path,
        client: &reqwest::Client,
        progress: F,
    ) -> Result<(PathBuf, PathBuf), SandboxError>
    where
        F: Fn(&str, f32) + Send + Sync + 'static,
    {
        let data_dir = data_dir.to_path_buf();
        let client = client.clone();
        self.run_owned(async move { prepare_rootfs(&data_dir, &client, &progress).await })
            .await
    }

    pub async fn ensure_java<F>(
        &self,
        data_dir: &Path,
        client: &reqwest::Client,
        java_major: u32,
        progress: F,
    ) -> Result<(PathBuf, PathBuf, String), SandboxError>
    where
        F: Fn(&str, f32) + Send + Sync + 'static,
    {
        let data_dir = data_dir.to_path_buf();
        let client = client.clone();
        self.run_owned(async move {
            let (sandbox, rootfs) = prepare_rootfs(&data_dir, &client, &progress).await?;
            let (major, package) = java_package(java_major);
            if let Some(java) = find_guest_java(&rootfs, major)? {
                return Ok((sandbox, rootfs, java));
            }
            progress(&format!("Installing Java {major} in sandbox..."), 0.30);
            package::install_java(&sandbox, &rootfs, package).await?;
            let java = find_guest_java(&rootfs, major)?.ok_or_else(|| {
                SandboxError::JavaBinaryNotFound {
                    package: package.to_string(),
                }
            })?;
            progress("Sandbox initialization complete!", 1.0);
            Ok((sandbox, rootfs, java))
        })
        .await
    }

    async fn run_owned<T, F>(&self, operation: F) -> Result<T, SandboxError>
    where
        T: Send + 'static,
        F: std::future::Future<Output = Result<T, SandboxError>> + Send + 'static,
    {
        let gate = self.gate.clone();
        let blocked = self.process_cleanup_failed.clone();
        let (sender, receiver) = tokio::sync::oneshot::channel();
        tokio::spawn(async move {
            let _guard = gate.lock().await;
            if sender.is_closed() {
                return;
            }
            if blocked.load(Ordering::Acquire) {
                let _ = sender.send(Err(SandboxError::RecoveryBlocked(
                    "package process cleanup was not confirmed; new sandbox setup is blocked",
                )));
                return;
            }
            let result = operation.await;
            if matches!(&result, Err(SandboxError::ProcessCleanup { .. })) {
                blocked.store(true, Ordering::Release);
            }
            if let Err(Err(error)) = sender.send(result) {
                eprintln!("[Sandbox] Setup completed after caller cancellation: {error}");
            }
        });
        receiver.await.map_err(|_| SandboxError::WorkerStopped)?
    }
}

async fn prepare_rootfs<F>(
    data_dir: &Path,
    client: &reqwest::Client,
    progress: &F,
) -> Result<(PathBuf, PathBuf), SandboxError>
where
    F: Fn(&str, f32) + Send + Sync + 'static,
{
    let data_dir = data_dir.to_path_buf();
    let sandbox = tokio::task::spawn_blocking(move || {
        let sandbox = get_sandbox_dir_in(&data_dir)?;
        publication::recover(&sandbox)?;
        Ok::<_, SandboxError>(sandbox)
    })
    .await??;
    let rootfs = sandbox.join("rootfs");
    if !is_rootfs_installed(&rootfs)? {
        let endpoint = format!(
            "https://dl-cdn.alpinelinux.org/alpine/{ALPINE_BRANCH}/releases/aarch64/alpine-minirootfs-{ALPINE_VERSION}-aarch64.tar.gz"
        );
        publication::install(client, &sandbox, &endpoint, progress).await?;
    }
    Ok((sandbox, rootfs))
}

#[cfg(test)]
mod tests;
