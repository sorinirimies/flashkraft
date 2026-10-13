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

    /// Print every theme name and exit.
    #[arg(long)]
    list_themes: bool,
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
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
    serve(ServerOptions {
        host: args.host,
        port: args.port,
        open_browser: !args.no_open,
    })
    .await
}
