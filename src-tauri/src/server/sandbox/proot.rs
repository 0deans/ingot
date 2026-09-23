use std::path::{Path, PathBuf};
use tokio::process::Command;

/// Checks whether the sandbox (PRoot) execution should be used
pub fn is_android_sandbox() -> bool {
    cfg!(target_os = "android") || std::env::var("INGOT_USE_PROOT").map(|v| v == "1").unwrap_or(false)
}

/// Locates the proot executable
/// On Android, it is packaged as `libproot.so` in `nativeLibraryDir`
pub fn locate_proot_binary() -> Option<PathBuf> {
    // 1. Check explicit environment override
    if let Ok(path) = std::env::var("INGOT_PROOT_PATH") {
        let p = PathBuf::from(path);
        if p.exists() {
            return Some(p);
        }
    }

    // 2. Check Android nativeLibraryDir paths
    if let Ok(lib_dir) = std::env::var("ANDROID_APP_LIB_DIR") {
        let candidate = PathBuf::from(lib_dir).join("libproot.so");
        if candidate.exists() {
            return Some(candidate);
        }
    }

    // 3. Fallback check for system or local proot
    if let Ok(path) = std::env::var("PATH") {
        for dir in std::env::split_paths(&path) {
            let p = dir.join(if cfg!(windows) { "proot.exe" } else { "proot" });
            if p.exists() {
                return Some(p);
            }
        }
    }

    None
}

/// Builds a Command to execute the Minecraft server process
/// Either inside PRoot (on Android) or natively (on Desktop)
pub fn build_server_command(
    rootfs_dir: Option<&Path>,
    server_dir: &Path,
    java_bin: &Path,
    jvm_args: &[String],
) -> Result<Command, String> {
    if is_android_sandbox() {
        let rootfs = rootfs_dir.ok_or_else(|| "Sandbox rootfs path is required on Android".to_string())?;
        let proot = locate_proot_binary().unwrap_or_else(|| PathBuf::from("libproot.so"));

        let mut cmd = Command::new(proot);
        // PRoot arguments:
        // -0: fake root (UID/GID 0)
        // -r <rootfs>: new root directory
        // -b <server_dir>:/server: bind mount the server folder
        // -w /server: working directory
        cmd.arg("-0");
        cmd.arg("-r");
        cmd.arg(rootfs);
        cmd.arg("-b");
        cmd.arg(format!("{}:/server", server_dir.to_string_lossy()));
        cmd.arg("-w");
        cmd.arg("/server");

        // Inside rootfs, invoke java
        cmd.arg("/usr/bin/java");
        for arg in jvm_args {
            cmd.arg(arg);
        }

        // Pass server directory as working dir on host as well
        cmd.current_dir(server_dir);
        Ok(cmd)
    } else {
        // Native desktop execution
        let mut cmd = Command::new(java_bin);
        cmd.current_dir(server_dir);
        for arg in jvm_args {
            cmd.arg(arg);
        }
        Ok(cmd)
    }
}
