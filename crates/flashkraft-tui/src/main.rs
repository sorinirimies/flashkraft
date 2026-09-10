//! FlashKraft TUI — application entry point.
//!
//! Privileges are initialized before the Tokio runtime is created, ensuring no
//! application thread starts with effective root privileges. A setuid-root
//! installation retains only the saved identity needed by the core flash
//! pipeline for narrowly scoped raw-device operations.
//!
//! Unlike the GUI, the TUI opts into transparent sudo/pkexec escalation for
//! non-setuid installs (`cargo run`/`cargo install`): the TUI is already tied
//! to a terminal, so `sudo` prompting for a password there is expected,
//! unsurprising behaviour, and the re-exec never disturbs a display/D-Bus
//! session the TUI doesn't use anyway. See
//! `flashkraft_core::flash_helper::initialize_privileges`.

fn main() -> anyhow::Result<()> {
    flashkraft_core::flash_helper::initialize_privileges(true).map_err(anyhow::Error::msg)?;

    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?
        .block_on(flashkraft_tui::run())
}
