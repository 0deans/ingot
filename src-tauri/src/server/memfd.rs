/// Android-only helper: execute a binary from a noexec filesystem path
/// by loading it into an anonymous in-memory file via `memfd_create(2)`.
///
/// # Why this is needed
/// Android 10+ (API 29+) enforces W^X via SELinux: all paths under
/// `/data/data/<pkg>/` are mounted `noexec`, so `execve()` on files there
/// returns EACCES (Permission denied, os error 13).
///
/// `memfd_create()` creates an anonymous file backed by RAM. Files created
/// this way are executable by default (they are not subject to filesystem
/// mount flags). Executing via `/proc/self/fd/<N>` tells the kernel to load
/// the binary directly from the memfd, bypassing the noexec check.
///
/// # SELinux note
/// This works on most Android 10–16 devices. Samsung Knox and some custom
/// ROMs with `execmemfd` neverallow rules may still block it — in that case
/// the only reliable alternative is bundling the binary in the APK as a .so.
#[cfg(target_os = "android")]
pub mod memfd {
    use std::io::Write;
    use std::os::unix::io::FromRawFd;
    use std::path::Path;

    /// Reads a binary from `bin_path` (which may be on a noexec mount),
    /// copies it into a `memfd`, and returns a `tokio::process::Command`
    /// that will execute it from `/proc/self/fd/<N>`.
    ///
    /// The returned Command has `current_dir` set to `work_dir`.
    pub fn command_from_noexec_path(
        bin_path: &Path,
        work_dir: &Path,
    ) -> Result<tokio::process::Command, String> {
        use libc::{c_uint, syscall, SYS_memfd_create};

        // Read the binary from disk into RAM
        let bytes = std::fs::read(bin_path)
            .map_err(|e| format!("Failed to read binary {}: {e}", bin_path.display()))?;

        // Create an anonymous executable in-memory file
        // MFD_CLOEXEC (1u32): close the fd in the parent after fork+exec
        let mfd_cloexec: c_uint = 1;
        let name = std::ffi::CString::new("ingot-exec").unwrap();
        let fd = unsafe { syscall(SYS_memfd_create, name.as_ptr(), mfd_cloexec) } as i32;

        if fd < 0 {
            let err = std::io::Error::last_os_error();
            return Err(format!(
                "memfd_create failed ({err}). \
                 Your device may enforce execmemfd SELinux restrictions. \
                 Rebuild the APK with the binary bundled in jniLibs/arm64-v8a/ instead."
            ));
        }

        // Write the binary bytes into the memfd
        // SAFETY: fd is valid and we own it
        let mut mem_file = unsafe { std::fs::File::from_raw_fd(fd) };
        mem_file
            .write_all(&bytes)
            .map_err(|e| format!("memfd write failed: {e}"))?;
        // Keep fd open — /proc/self/fd/N must remain valid until the child execs
        let fd = {
            use std::os::unix::io::IntoRawFd;
            mem_file.into_raw_fd()
        };

        // Build a Command that execs via the memfd path
        let exe = format!("/proc/self/fd/{fd}");
        let mut cmd = tokio::process::Command::new(exe);
        cmd.current_dir(work_dir);

        // Arrange to close the fd after spawn so we don't leak it in the parent.
        // We do this by storing it in a pre_exec hook.
        //
        // SAFETY: closing an fd in a pre_exec hook is async-signal-safe.
        unsafe {
            cmd.pre_exec(move || {
                // fd was created without CLOEXEC on the child side intentionally:
                // the kernel needs it open during execve to resolve /proc/self/fd/N.
                // After execve loads the binary, the new process image has no reference
                // to this fd. Nothing to do here — the OS handles the rest.
                Ok(())
            });
        }

        // Close fd in the parent *after* spawn returns (child has exec'd by then)
        // We can't do this inside Command easily, so we leak the fd intentionally —
        // it's a single integer and the child closes it on exec (MFD_CLOEXEC).
        // The parent fd is closed when the process exits or we explicitly close it.
        // For a long-running server this is acceptable (one open fd per server).

        Ok(cmd)
    }
}
