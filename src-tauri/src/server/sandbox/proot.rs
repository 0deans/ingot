use std::path::{Path, PathBuf};
use tokio::process::Command;

/// Checks whether the sandbox (PRoot) execution should be used
pub fn is_android_sandbox() -> bool {
    cfg!(target_os = "android") || std::env::var("INGOT_USE_PROOT").map(|v| v == "1").unwrap_or(false)
}

/// Directory holding the APK's extracted native libraries (`nativeLibraryDir`).
/// Only files in here may be `execve`'d on Android 10+ (W^X policy).
fn android_lib_dir() -> Option<PathBuf> {
    std::env::var("ANDROID_APP_LIB_DIR").ok().map(PathBuf::from)
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
    if let Some(lib_dir) = android_lib_dir() {
        let candidate = lib_dir.join("libproot.so");
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

/// Fake /proc entries that Android hides from apps (SELinux denies reading them).
/// The JVM and apk read some of these, so we bind static replacements when needed.
const FAKE_PROC_FILES: &[(&str, &str)] = &[
    ("loadavg", "0.12 0.07 0.02 2/165 765\n"),
    (
        "stat",
        "cpu  1957 0 2877 93280 262 342 254 87 0 0\ncpu0 31 0 226 12027 82 10 4 9 0 0\nintr 0\nctxt 0\nbtime 0\nprocesses 0\nprocs_running 1\nprocs_blocked 0\n",
    ),
    ("uptime", "124.08 932.80\n"),
    ("version", "Linux version 6.1.0-ingot (ingot@localhost) #1 SMP PREEMPT\n"),
    ("vmstat", "nr_free_pages 0\n"),
];

/// Prepares host-side support files for PRoot and returns a Command with the
/// common PRoot arguments/environment set. The caller appends the guest program.
///
/// With `rootfs: None` the guest sees the host (Android) filesystem; this is used to
/// overlay files like `/etc/resolv.conf` for static musl binaries such as playit.
/// `fake_root` (-0) is only needed for package management; servers run as a normal user.
///
/// Termux's proot is dynamically linked against `libtalloc.so.2`, but Android only
/// extracts `lib*.so` files from the APK, so it is bundled as `libtalloc.so` and
/// exposed under its soname via a symlink in `<sandbox>/lib`.
pub fn proot_command(
    sandbox_dir: &Path,
    rootfs: Option<&Path>,
    fake_root: bool,
    binds: &[(PathBuf, &str)],
    guest_cwd: &str,
) -> Result<Command, String> {
    let proot = locate_proot_binary().ok_or_else(|| {
        "PRoot binary not found. The APK must bundle libproot.so (run scripts/download-android-libs.ps1 before building).".to_string()
    })?;

    let tmp_dir = sandbox_dir.join("tmp");
    std::fs::create_dir_all(&tmp_dir).map_err(|e| format!("Failed to create PRoot tmp dir: {e}"))?;

    let mut cmd = Command::new(&proot);
    cmd.env_clear();
    cmd.env("PROOT_TMP_DIR", &tmp_dir);
    cmd.env("HOME", "/root");
    cmd.env("PATH", "/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin");
    cmd.env("LANG", "C.UTF-8");
    cmd.env("TERM", "dumb");
    cmd.env("TMPDIR", "/tmp");

    if let Some(lib_dir) = android_lib_dir() {
        let loader = lib_dir.join("libproot-loader.so");
        if !loader.exists() {
            return Err(format!(
                "PRoot loader not found at {}. The APK must bundle libproot-loader.so.",
                loader.display()
            ));
        }
        cmd.env("PROOT_LOADER", &loader);
        let loader32 = lib_dir.join("libproot-loader32.so");
        if loader32.exists() {
            cmd.env("PROOT_LOADER_32", &loader32);
        }

        // nativeLibraryDir changes on every app update, so always refresh the symlink.
        let soname_dir = sandbox_dir.join("lib");
        std::fs::create_dir_all(&soname_dir).map_err(|e| format!("Failed to create PRoot lib dir: {e}"))?;
        let talloc_link = soname_dir.join("libtalloc.so.2");
        let _ = std::fs::remove_file(&talloc_link);
        #[cfg(unix)]
        std::os::unix::fs::symlink(lib_dir.join("libtalloc.so"), &talloc_link)
            .map_err(|e| format!("Failed to link libtalloc.so.2: {e}"))?;

        let ld_path = std::env::join_paths([soname_dir.as_path(), lib_dir.as_path()])
            .map_err(|e| format!("Invalid library path: {e}"))?;
        cmd.env("LD_LIBRARY_PATH", ld_path);
    }

    // --link2symlink: Android forbids hard links (apk/dpkg need them),
    // --kill-on-exit: don't leave orphaned guest processes behind
    cmd.arg("--kill-on-exit");
    cmd.arg("--link2symlink");

    if fake_root {
        cmd.arg("-0");
    }
    if let Some(rootfs) = rootfs {
        add_rootfs_args(&mut cmd, sandbox_dir, rootfs);
    }

    for (host, guest) in binds {
        cmd.arg("-b");
        cmd.arg(format!("{}:{guest}", host.to_string_lossy()));
    }

    cmd.arg("-w");
    cmd.arg(guest_cwd);

    if let Some(rootfs) = rootfs {
        // Keep the host's LD_LIBRARY_PATH (needed by proot itself) out of the musl guest
        cmd.arg("/usr/bin/env");
        cmd.arg("-u");
        cmd.arg("LD_LIBRARY_PATH");
        if install_stack_guard(rootfs) {
            cmd.arg(format!("LD_PRELOAD={STACK_GUARD_GUEST_PATH}"));
        }
    }

    Ok(cmd)
}

/// musl LD_PRELOAD shim that gives thread stacks a guard page. Without it the JVM dies
/// at startup on Android with "Failed to mark memory page as executable" because
/// SELinux denies `execstack`. See stackguard.c for details.
const STACK_GUARD_SO: &[u8] = include_bytes!("stackguard-aarch64.so");
const STACK_GUARD_GUEST_PATH: &str = "/usr/lib/ingot-stackguard.so";

/// Writes the stack guard shim into the rootfs (aarch64 only); returns whether it's usable
fn install_stack_guard(rootfs: &Path) -> bool {
    if !cfg!(target_arch = "aarch64") {
        return false;
    }
    let host_path = rootfs.join(STACK_GUARD_GUEST_PATH.trim_start_matches('/'));
    if std::fs::read(&host_path).is_ok_and(|b| b == STACK_GUARD_SO) {
        return true;
    }
    std::fs::write(&host_path, STACK_GUARD_SO).is_ok()
}

/// Arguments for running inside a full Linux rootfs: the host pseudo-filesystems,
/// with replacements for /proc files Android hides
fn add_rootfs_args(cmd: &mut Command, sandbox_dir: &Path, rootfs: &Path) {
    cmd.arg("-r");
    cmd.arg(rootfs);
    for host in ["/dev", "/proc", "/sys"] {
        cmd.arg("-b");
        cmd.arg(host);
    }
    cmd.arg("-b");
    cmd.arg("/dev/urandom:/dev/random");

    let fake_proc_dir = sandbox_dir.join("proc");
    for (name, content) in FAKE_PROC_FILES {
        let real = Path::new("/proc").join(name);
        if std::fs::read(&real).is_ok() {
            continue;
        }
        let fake = fake_proc_dir.join(name);
        if std::fs::create_dir_all(&fake_proc_dir).is_ok() && std::fs::write(&fake, content).is_ok() {
            cmd.arg("-b");
            cmd.arg(format!("{}:/proc/{name}", fake.to_string_lossy()));
        }
    }
}

/// Builds a Command to execute the Minecraft server process
/// Either inside PRoot (on Android) or natively (on Desktop)
pub fn build_server_command(
    sandbox: Option<(&Path, &Path)>,
    server_dir: &Path,
    java_bin: &Path,
    jvm_args: &[String],
) -> Result<Command, String> {
    let mut cmd = if is_android_sandbox() {
        let (sandbox_dir, rootfs) =
            sandbox.ok_or_else(|| "Sandbox rootfs path is required on Android".to_string())?;
        let mut cmd = proot_command(sandbox_dir, Some(rootfs), false, &[(server_dir.to_path_buf(), "/server")], "/server")?;
        // Inside rootfs, invoke java
        cmd.arg(java_bin);
        cmd
    } else {
        // Native desktop execution
        Command::new(java_bin)
    };
    cmd.args(jvm_args);
    cmd.current_dir(server_dir);
    Ok(cmd)
}
