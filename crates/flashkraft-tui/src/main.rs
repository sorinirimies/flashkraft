//! FlashKraft TUI — binary entry point
//!
//! This file is intentionally thin.  All application logic lives in
//! `lib.rs` and the `core/` + `ui/` submodules so that examples and integration
//! tests can import from `flashkraft_tui::` without also pulling in the
//! `main` function.
//!
//! ## Privilege model
//!
//! FlashKraft needs root access only for the brief moment it opens a raw
//! block device for writing.  It supports two escalation paths, tried in order:
//!
//! ### 1. setuid-root (preferred — zero runtime dependency)
//!
//! ```text
//! sudo chown root:root /usr/bin/flashkraft-tui
//! sudo chmod u+s       /usr/bin/flashkraft-tui
//! ```
//!
//! The binary then carries the setuid bit permanently.  At startup
//! `nix::unistd::getuid()` captures the **real** (unprivileged) UID before
//! any `seteuid` call; the flash pipeline drops back to that UID immediately
//! after opening the block device file descriptor.
//!
//! ### 2. pkexec / sudo re-exec (automatic fallback)
//!
//! If the binary is **not** setuid-root (`getuid() == geteuid()` and
//! `geteuid() != 0`), `main()` attempts to re-exec itself via:
//!
//! 1. `pkexec /path/to/self <args>` — integrates with the desktop polkit agent
//! 2. `sudo -E /path/to/self <args>` — terminal fallback (prompts in-place)
//!
//! The re-exec replaces the current process (`exec`), so if it succeeds the
//! rest of `main()` never runs in the original process.  The new privileged
//! process restarts from the top of `main()`, finds `geteuid() == 0`, skips
//! re-exec, and proceeds normally.
//!
//! If both escalation helpers fail (neither `pkexec` nor `sudo` is present,
//! or the user dismisses the polkit dialog / Ctrl-C's the sudo prompt), the
//! app starts unprivileged and shows a clear error when the user tries to flash.

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // ── Unix privilege bootstrap ─────────────────────────────────────────────
    //
    // Everything inside this block compiles away on Windows / other platforms.
    #[cfg(unix)]
    {
        let real_uid = nix::unistd::getuid();
        let effective_uid = nix::unistd::geteuid();

        if effective_uid.is_root() {
            // Already running as root (setuid-root or re-exec succeeded).
            // Record the real (unprivileged) UID so the flash pipeline can
            // drop back to it after opening the block device.
            flashkraft_core::flash_helper::set_real_uid(real_uid.as_raw());
        } else {
            // Not root yet — attempt a transparent re-exec through pkexec or
            // sudo so the user gets a polkit/password prompt rather than a
            // hard error mid-flash.
            //
            // `try_reexec_as_root()` calls `execvp`, which replaces this
            // process image entirely on success, so the lines after the call
            // are only reached when escalation is unavailable or the user
            // cancelled the prompt.  In that case we fall through and start
            // the TUI unprivileged; a clear error will be shown when the user
            // attempts to flash a drive.
            // Skip privilege escalation when running under `cargo test` —
            // sudo/pkexec would block the test runner waiting for a password.
            #[cfg(not(test))]
            try_reexec_as_root();

            // Still here → escalation not available / declined.
            // Store the real UID anyway (real == effective in this branch).
            flashkraft_core::flash_helper::set_real_uid(real_uid.as_raw());
        }
    }

    // ── Normal TUI startup ────────────────────────────────────────────────────
    flashkraft_tui::run().await
}

// ---------------------------------------------------------------------------
// Privilege re-exec helper (Unix only)
// ---------------------------------------------------------------------------

/// Attempt to re-exec the current binary with elevated privileges.
///
/// Tries, in order:
/// 1. `pkexec <self> [args…]` — integrates with the desktop polkit agent
///    (GNOME, KDE, XFCE, …).  Produces a graphical password dialog.
/// 2. `sudo -E <self> [args…]` — terminal/tty fallback; prompts in the
///    current terminal for a password.
///
/// Both calls use `execvp`, which **replaces** the current process on success.
/// If this function returns, neither helper is available or the user declined.
///
/// `FLASHKRAFT_ESCALATED=1` is added to the environment of the child so that
/// if somehow we end up in a re-exec loop (escalation tool present but keeps
/// failing) we stop after one attempt.
#[cfg(all(unix, not(test)))]
fn try_reexec_as_root() {
    use std::ffi::CString;

    // Guard against infinite re-exec loops.
    if std::env::var("FLASHKRAFT_ESCALATED").as_deref() == Ok("1") {
        return;
    }

    let self_exe = match std::fs::read_link("/proc/self/exe").or_else(|_| {
        // Fallback: use the path reported by the OS
        std::env::current_exe()
    }) {
        Ok(p) => p,
        Err(_) => return,
    };

    let self_exe_str = match self_exe.to_str() {
        Some(s) => s.to_owned(),
        None => return,
    };

    // Collect original argv (skip argv[0] — we supply self_exe instead).
    let extra_args: Vec<String> = std::env::args().skip(1).collect();

    // Set the escalation guard so the re-exec'd process skips this block.
    // `sudo -E` preserves the environment; pkexec preserves a safe subset.
    std::env::set_var("FLASHKRAFT_ESCALATED", "1");

    // ── Try pkexec ────────────────────────────────────────────────────────────
    if which_exists("pkexec") {
        let mut argv: Vec<CString> = Vec::new();
        argv.push(c_str("pkexec"));
        argv.push(c_str(&self_exe_str));
        for arg in &extra_args {
            argv.push(c_str(arg));
        }
        // execvp replaces the process; only returns on error.
        let _ = nix::unistd::execvp(&c_str("pkexec"), &argv);
    }

    // ── Try sudo ──────────────────────────────────────────────────────────────
    if which_exists("sudo") {
        let mut argv: Vec<CString> = Vec::new();
        argv.push(c_str("sudo"));
        argv.push(c_str("-E")); // preserve environment (DISPLAY, WAYLAND_DISPLAY, TERM, …)
        argv.push(c_str(&self_exe_str));
        for arg in &extra_args {
            argv.push(c_str(arg));
        }
        let _ = nix::unistd::execvp(&c_str("sudo"), &argv);
    }

    // Neither helper available — unset the guard and fall through unprivileged.
    std::env::remove_var("FLASHKRAFT_ESCALATED");
}

/// Return `true` if `name` resolves to an executable file via `PATH`.
#[cfg(all(unix, not(test)))]
fn which_exists(name: &str) -> bool {
    if let Ok(path_var) = std::env::var("PATH") {
        for dir in path_var.split(':') {
            let candidate = std::path::Path::new(dir).join(name);
            if candidate.is_file() {
                use std::os::unix::fs::PermissionsExt;
                if let Ok(meta) = std::fs::metadata(&candidate) {
                    if meta.permissions().mode() & 0o111 != 0 {
                        return true;
                    }
                }
            }
        }
    }
    false
}

/// Build a `CString` from a `&str`, replacing any embedded NUL bytes with `?`.
#[cfg(all(unix, not(test)))]
fn c_str(s: &str) -> std::ffi::CString {
    let sanitised: Vec<u8> = s.bytes().map(|b| if b == 0 { b'?' } else { b }).collect();
    std::ffi::CString::new(sanitised).unwrap_or_else(|_| std::ffi::CString::new("?").unwrap())
}
