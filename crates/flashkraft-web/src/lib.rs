//! FlashKraft web — a server-rendered drive & image inspector built with
//! [Topcoat](https://github.com/tokio-rs/topcoat).
//!
//! It shows what the TUI and GUI see — the connected drives, whether a chosen
//! image fits each one, and your flash history — in a browser — and can
//! **flash one or several drives at once**. Because a page that can erase disks
//! must be hard to abuse, every flash passes the guards in [`flash`]
//! (loopback only, `Host` check, per-session token, server-side drive
//! re-validation, explicit confirmation). `--read-only` disables flashing.
//!
//! | module | what |
//! |---|---|
//! | [`app`] | [`ServerOptions`], router construction, [`serve`] |
//! | [`pages`] | document shell, dashboard, history, about, live shard |
//! | [`components`] | cards, tables, filters |
//! | [`api`] | JSON + static routes, flash endpoints |
//! | [`flash`] | flash job manager and every request guard |
//! | [`data`] | shaping drives / image / history into a [`data::Snapshot`] |
//! | [`params`] | query-string values |
//! | [`style`] | the stylesheet |
//! | [`theme`] | the core's 43 colour themes as CSS custom properties |
//!
//! **Live mode** (asset bundle present): the image field and theme are
//! signals and the drive list refreshes itself over a WebSocket (new USB
//! sticks appear without a reload). **Static mode**: GET-form + `<meta refresh>`.

// Topcoat's shard API takes one plain argument per signal.
#![allow(clippy::too_many_arguments)]

pub mod api;
pub mod app;
pub mod browse;
pub mod components;
pub mod data;
pub mod flash;
pub mod pages;
pub mod params;
pub mod style;
pub mod theme;

pub use app::{ServerOptions, router, serve};
