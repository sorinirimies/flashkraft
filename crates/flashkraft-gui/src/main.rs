//! FlashKraft GUI — application entry point.
//!
//! On Unix, a setuid-root installation starts with a saved root identity but
//! immediately drops its effective UID to the invoking user before Iced or any
//! other UI subsystem is initialized. The core flash pipeline regains root only
//! for narrowly scoped raw-device operations.
//!
//! Unlike the TUI, the GUI never attempts transparent sudo/pkexec re-exec:
//! doing so would replace the whole process image, breaking the native
//! file-picker's access to the desktop D-Bus/portal session. GUI installs
//! must be setuid-root (`just install`, `just install-nix`, or the NixOS
//! module).

fn main() -> iced::Result {
    if let Err(error) = flashkraft_core::flash_helper::initialize_privileges(false) {
        eprintln!("FlashKraft privilege initialization failed: {error}");
        std::process::exit(1);
    }

    flashkraft_gui::run_gui()
}
