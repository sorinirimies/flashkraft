//! FlashKraft TUI — application entry point.
//!
//! Privileges are initialized before the Tokio runtime is created, ensuring no
//! application thread starts with effective root privileges. A setuid-root
//! installation retains only the saved identity needed by the core flash
//! pipeline for narrowly scoped raw-device operations.

fn main() -> anyhow::Result<()> {
    flashkraft_core::flash_helper::initialize_privileges().map_err(anyhow::Error::msg)?;

    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?
        .block_on(flashkraft_tui::run())
}
