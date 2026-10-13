//! FlashKraft web — a server-rendered drive & image inspector built with
//! [Topcoat](https://github.com/tokio-rs/topcoat).
//!
//! It shows what the TUI and GUI see — the connected drives, whether a chosen
//! image fits each one, and your flash history — in a browser. It is
//! **read-only by design**: flashing needs raw device access, so it stays in
//! the TUI/GUI; this crate never opens a block device for writing.
//!
//! | module | what |
//! |---|---|
//! | [`app`] | [`ServerOptions`], router construction, [`serve`] |
//! | [`pages`] | document shell, dashboard, history, about, live shard |
//! | [`components`] | cards, tables, filters |
//! | [`api`] | JSON + static routes |
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
pub mod components;
pub mod data;
pub mod pages;
pub mod params;
pub mod style;
pub mod theme;

pub use app::{ServerOptions, router, serve};
