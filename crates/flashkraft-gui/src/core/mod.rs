//! Core Module — Iced Application Logic
//!
//! This module contains the Iced-specific application core following
//! The Elm Architecture pattern.
//!
//! ## What lives here
//!
//! | Module | Responsibility |
//! |--------|----------------|
//! | [`state`] | Application state (`FlashKraft` struct) + Elm methods |
//! | [`message`] | All `Message` variants (user events + async results) |
//! | [`update()`] | Pure state-transition function |
//! | [`flash_runner`] | Iced [`iced::Subscription`] that streams flash progress |
//! | [`storage`] | Persistent theme preference via `redb` |
//! | [`commands`] | Async side effects (file picker dialog) |
//!
//! ## What does NOT live here
//!
//! - The actual flash pipeline (`flash_helper`) —
//!   that is framework-free and lives in `flashkraft-core`.
//! - Drive detection — also in `flashkraft-core::commands::drive_detection`.

pub mod commands;
pub mod flash_runner;
pub mod message;
pub mod state;
pub mod storage;
pub mod update;

// ── Convenience re-exports ────────────────────────────────────────────────────

pub use message::Message;
pub use state::FlashKraft;
pub use update::update;
