//! FlashKraft GUI — application entry point.
//!
//! On Unix, a setuid-root installation starts with a saved root identity but
//! immediately drops its effective UID to the invoking user before Iced or any
//! other UI subsystem is initialized. If the binary is not setuid-root (e.g.
//! installed via plain `cargo install`), Linux falls back to a transparent
//! `sudo`/`pkexec` re-exec followed by the same privilege drop. See
//! `flashkraft_core::flash_helper::initialize_privileges` for details. The
//! core flash pipeline regains root only for narrowly scoped raw-device
//! operations.

fn main() -> iced::Result {
    if let Err(error) = flashkraft_core::flash_helper::initialize_privileges() {
        eprintln!("FlashKraft privilege initialization failed: {error}");
        std::process::exit(1);
    }

    flashkraft_gui::run_gui()
}
