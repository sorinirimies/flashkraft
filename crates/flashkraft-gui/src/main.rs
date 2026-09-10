//! FlashKraft GUI — application entry point.
//!
//! On Unix, a setuid-root installation starts with a saved root identity but
//! immediately drops its effective UID to the invoking user before Iced or any
//! other UI subsystem is initialized. The core flash pipeline regains root only
//! for narrowly scoped raw-device operations.
//!
//! Unlike the TUI, the GUI never attempts transparent sudo/pkexec re-exec of
//! *itself*: doing so would replace the whole process image, breaking the
//! native file-picker's access to the desktop D-Bus/portal session. GUI
//! installs should still be setuid-root (`just install`, `just install-nix`,
//! or the NixOS module) for the fast, prompt-free path.
//!
//! For unprivileged builds (e.g. `cargo run`), the flash pipeline instead
//! escalates only the raw-device write by spawning a short-lived, headless
//! privileged **child** process via `pkexec`/`sudo` — the GUI process itself
//! keeps running throughout. See [`flashkraft_gui::privileged_helper`] for
//! the full design.

fn main() -> iced::Result {
    // Hidden, non-interactive worker mode: spawned via `pkexec`/`sudo` by an
    // unprivileged GUI instance (e.g. `cargo run`) to perform the raw-device
    // flash when the binary isn't installed setuid-root. Never initializes
    // Iced, D-Bus, or any UI subsystem — see `privileged_helper` for details.
    if let Some(code) = flashkraft_gui::privileged_helper::run_if_invoked() {
        std::process::exit(code);
    }

    if let Err(error) = flashkraft_core::flash_helper::initialize_privileges(false) {
        eprintln!("FlashKraft privilege initialization failed: {error}");
        std::process::exit(1);
    }

    flashkraft_gui::run_gui()
}
