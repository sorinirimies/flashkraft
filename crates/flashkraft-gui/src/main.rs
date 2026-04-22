//! FlashKraft GUI — application entry point
//!
//! ## Privilege model
//!
//! FlashKraft needs root access only for the brief moment it opens a raw
//! block device for writing.  It supports two escalation paths, tried in order:
//!
//! ### 1. setuid-root (preferred — zero runtime dependency)
//!
//! ```text
//! sudo chown root:root /usr/bin/flashkraft
//! sudo chmod u+s       /usr/bin/flashkraft
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
//! 2. `sudo -E /path/to/self <args>` — terminal fallback
//!
//! The re-exec replaces the current process (`exec`), so if it succeeds the
//! rest of `main()` never runs in the original process.  The new privileged
//! process restarts from the top of `main()`, finds `geteuid() == 0`, skips
//! re-exec, and proceeds normally.
//!
//! If both escalation helpers fail (neither `pkexec` nor `sudo` is present,
//! or the user dismisses the polkit dialog), the app starts unprivileged and
//! shows a clear error message with the manual setuid instructions when the
//! user tries to flash.

use flashkraft_gui::{FlashKraft, Message};
use iced::{Settings, Task};

fn main() -> iced::Result {
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
            // `try_reexec_as_root()` calls `execv`/`execvp`, which replaces
            // this process image entirely on success, so the lines after the
            // call are only reached when escalation is unavailable or the user
            // cancelled the prompt.  In that case we fall through and start
            // the GUI unprivileged; a clear error will be shown when the user
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

    // ── Start the Iced GUI ───────────────────────────────────────────────────
    iced::application(
        || {
            // Initialise application state.
            let initial_state = FlashKraft::new();

            // Kick off drive detection immediately on startup.
            let initial_command = Task::perform(
                flashkraft_core::commands::load_drives(),
                Message::DrivesRefreshed,
            );

            (initial_state, initial_command)
        },
        FlashKraft::update,
        FlashKraft::view,
    )
    .title("FlashKraft - OS Image Writer")
    .subscription(FlashKraft::subscription)
    .theme(|state: &FlashKraft| state.theme.clone())
    .settings(Settings {
        fonts: vec![iced_fonts::BOOTSTRAP_FONT_BYTES.into()],
        ..Default::default()
    })
    .window(iced::window::Settings {
        size: iced::Size::new(1300.0, 700.0),
        resizable: false,
        decorations: true,
        ..Default::default()
    })
    .run()
}

// ---------------------------------------------------------------------------
// Privilege re-exec helper (Unix only)
// ---------------------------------------------------------------------------

/// Attempt to re-exec the current binary with elevated privileges.
///
/// Tries, in order:
/// 1. `sudo -E <self> [args…]` — preserves the full environment (including
///    `DISPLAY`, `WAYLAND_DISPLAY`, `XDG_RUNTIME_DIR`) so the GUI can connect
///    to the display server after re-exec.
/// 2. `pkexec <self> [args…]` — polkit graphical agent fallback.  **Note:**
///    pkexec intentionally strips display-related env vars for security; when
///    it is used the re-execed process will receive `DISPLAY`/`WAYLAND_DISPLAY`
///    only if a polkit `allow_gui` action is configured, otherwise the GUI will
///    start without a display and show a clear error.
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
        // Fallback: use argv[0] resolved through PATH
        std::env::current_exe()
    }) {
        Ok(p) => p,
        Err(_) => return,
    };

    let self_exe_str = match self_exe.to_str() {
        Some(s) => s.to_owned(),
        None => return,
    };

    // Collect original argv (skip argv[0] — we replace it with self_exe).
    let extra_args: Vec<String> = std::env::args().skip(1).collect();

    // Set the escalation guard so the re-execed process does not loop.
    std::env::set_var("FLASHKRAFT_ESCALATED", "1");

    // ── Try sudo first ────────────────────────────────────────────────────────
    // `sudo -E` preserves the caller's full environment, which means
    // DISPLAY, WAYLAND_DISPLAY, XDG_RUNTIME_DIR, etc. all survive the exec.
    // This is essential for the GUI to connect to the display server.
    if which_exists("sudo") {
        let mut argv: Vec<CString> = Vec::new();
        argv.push(c_str("sudo"));
        argv.push(c_str("-E")); // preserve full environment
        argv.push(c_str(&self_exe_str));
        for arg in &extra_args {
            argv.push(c_str(arg));
        }
        // execvp replaces the process; only returns on error.
        let _ = nix::unistd::execvp(&c_str("sudo"), &argv);
    }

    // ── Fall back to pkexec ───────────────────────────────────────────────────
    // pkexec provides a graphical polkit dialog but intentionally strips display
    // env vars. It will work on systems where a polkit `allow_gui` action is
    // configured for flashkraft, or when running under X11 with XAUTHORITY set
    // via the polkit action annotation.
    if which_exists("pkexec") {
        let mut argv: Vec<CString> = Vec::new();
        argv.push(c_str("pkexec"));
        argv.push(c_str(&self_exe_str));
        for arg in &extra_args {
            argv.push(c_str(arg));
        }
        let _ = nix::unistd::execvp(&c_str("pkexec"), &argv);
    }

    // Neither helper available — unset the guard and fall through unprivileged.
    std::env::remove_var("FLASHKRAFT_ESCALATED");
}

/// Return `true` if `name` resolves to an executable via `PATH`.
#[cfg(all(unix, not(test)))]
fn which_exists(name: &str) -> bool {
    if let Ok(path_var) = std::env::var("PATH") {
        for dir in path_var.split(':') {
            let candidate = std::path::Path::new(dir).join(name);
            if candidate.is_file() {
                // Check executable bit
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

/// Convenience: build a `CString`, replacing interior NULs with `?`.
#[cfg(all(unix, not(test)))]
fn c_str(s: &str) -> std::ffi::CString {
    // CString::new only fails on embedded NUL bytes — sanitise defensively.
    let sanitised: Vec<u8> = s.bytes().map(|b| if b == 0 { b'?' } else { b }).collect();
    std::ffi::CString::new(sanitised).unwrap_or_else(|_| std::ffi::CString::new("?").unwrap())
}
