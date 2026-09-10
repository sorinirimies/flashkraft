//! FlashKraft GUI — application entry point.
//!
//! On Unix, a setuid-root installation starts with a saved root identity but
//! immediately drops its effective UID to the invoking user before Iced or any
//! other UI subsystem is initialized. If the binary is not setuid-root (e.g.
//! installed via plain `cargo install`), the whole app runs fully unprivileged
//! — escalation is deferred until the user presses "Flash" (see
//! `Message::FlashClicked` in `core/update.rs` and
//! `flashkraft_core::flash_helper::escalate_for_flash`), so native file
//! dialogs and drive detection always run as a normal user process.
//!
//! A relaunch triggered by `escalate_for_flash` passes
//! `--internal-resume-flash <image_path> <device_path>` so this same binary
//! can skip the picker screens and resume straight into flashing once it
//! lands back here with root available.

fn main() -> iced::Result {
    let mut args = std::env::args().skip(1);
    let resume_flash = if args.next().as_deref() == Some("--internal-resume-flash") {
        match (args.next(), args.next()) {
            (Some(image_path), Some(device_path)) => Some((image_path, device_path)),
            _ => None,
        }
    } else {
        None
    };

    if let Err(error) = flashkraft_core::flash_helper::initialize_privileges() {
        eprintln!("FlashKraft privilege initialization failed: {error}");
        std::process::exit(1);
    }

    match resume_flash {
        Some((image_path, device_path)) => {
            flashkraft_gui::run_gui_resume_flash(image_path, device_path)
        }
        None => flashkraft_gui::run_gui(),
    }
}
