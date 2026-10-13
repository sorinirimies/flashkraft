//! `flashkraft-web` — local web dashboard.
//!
//! Binds `127.0.0.1:3000` by default; `--host` / `--port` (or `HOST` / `PORT`)
//! override. The page opens in your browser on start; `--no-open` skips that.

use clap::Parser;
use flashkraft_web::{ServerOptions, serve};

#[derive(Debug, Parser)]
#[command(name = "flashkraft-web", version, about)]
struct Args {
    /// Interface to bind (0.0.0.0 exposes it on the network — there is no auth).
    #[arg(long, env = "HOST", default_value = "127.0.0.1")]
    host: String,

    /// Port to listen on.
    #[arg(long, short, env = "PORT", default_value_t = 3000)]
    port: u16,

    /// Don't open the dashboard in a browser on start.
    #[arg(long)]
    no_open: bool,

    /// Show fake drives instead of the real ones (same as FLASHKRAFT_DEMO=1).
    #[arg(long)]
    demo: bool,

    /// Never flash from the browser (list drives and history only).
    #[arg(long)]
    read_only: bool,

    /// Print every theme name and exit.
    #[arg(long)]
    list_themes: bool,
}

fn main() -> std::io::Result<()> {
    let args = Args::parse();
    if args.list_themes {
        for name in flashkraft_core::THEME_NAMES {
            println!("{name}");
        }
        return Ok(());
    }
    if args.demo {
        // SAFETY: single-threaded at this point — before the server starts.
        unsafe { std::env::set_var(flashkraft_core::commands::demo::DEMO_ENV, "1") };
    }
    // Same privilege model as the TUI: drop root now, regain it only inside
    // the flash pipeline. Before the runtime starts any threads.
    flashkraft_core::flash_helper::initialize_privileges(true).map_err(std::io::Error::other)?;
    flashkraft_web::flash::configure(flashkraft_web::flash::flashing_allowed(
        &args.host,
        args.read_only,
    ));
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?
        .block_on(serve(ServerOptions {
            host: args.host,
            port: args.port,
            open_browser: !args.no_open,
        }))
}
