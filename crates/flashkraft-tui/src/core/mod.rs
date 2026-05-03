//! Core module — groups all business logic components.
//!
//! | Submodule        | Responsibility                                          |
//! |------------------|---------------------------------------------------------|
//! | `state`          | Application state machine & channel polling             |
//! | `message`        | Shared data types (screens, events, enums)              |
//! | `update`         | Keyboard event → state-transition mapping               |
//! | `flash_runner`   | Tokio task that drives the privileged flash child       |
//! | `storage`        | Redb-backed preference persistence                     |

// Re-export flashkraft_core items so that `crate::core::commands` etc.
// continue to resolve across the entire crate and in examples via
// `flashkraft_tui::core::*`.
pub use flashkraft_core::commands;
pub use flashkraft_core::domain;
pub use flashkraft_core::flash_helper;
pub use flashkraft_core::utils;

pub mod flash_runner;
pub mod message;
pub mod state;
pub mod storage;
pub mod update;
