//! UI Module — View Logic & Component Organisation
//!
//! This module contains the main view function that renders the UI
//! based on the application state, plus all reusable components and
//! per-screen views.
//!
//! ## Sub-modules
//!
//! | Module | What lives here |
//! |--------|-----------------|
//! | [`components`] | Reusable widgets (header, progress bars, theme picker, …) |
//! | [`screens`] | One file per screen (select image, select drive, flashing, …) |
//! | [`theme`] | Theme helpers and constants |

pub mod components;
pub mod screens;
pub mod theme;

use iced::widget::{column, container};
use iced::{Element, Length};

use crate::core::{FlashKraft, Message};

// ============================================================================
// Main View Entry Point
// ============================================================================

/// Main view function - decides what to show based on state
///
/// This is the entry point for rendering the UI. It examines the
/// current state and delegates to the appropriate view function.
///
/// # Arguments
///
/// * `state` - The current application state
///
/// # Returns
///
/// An Element describing the UI to render
pub fn view(state: &FlashKraft) -> Element<'_, Message> {
    let content = if state.is_flash_complete() {
        // Flash completed successfully
        screens::complete::view_complete(state)
    } else if state.is_flashing() {
        // Currently flashing
        screens::flashing::view_flashing(state)
    } else if state.has_error() {
        // Error occurred
        let error = state.error_message.as_deref().unwrap_or("Unknown error");
        screens::error::view_error(state, error)
    } else {
        // Normal main view
        view_main(state)
    };

    container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .into()
}

// ============================================================================
// Main Application View
// ============================================================================

/// Main application view (normal state)
fn view_main(state: &FlashKraft) -> Element<'_, Message> {
    // If device selection is open, show it as an overlay
    if state.device_selection_open {
        return screens::select_drive::view_device_selector(
            &state.available_drives,
            &state.selected_target,
            &state.selected_image,
        );
    }

    let header = components::header::view_header(state);
    let step_indicators = components::step_indicators::view_step_indicators(state);
    let buttons = screens::select_image::view_buttons(
        &state.selected_image,
        &state.selected_target,
        state.is_ready_to_flash(),
    );

    column![header, step_indicators, buttons]
        .spacing(30)
        .padding(20)
        .width(Length::Fill)
        .into()
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_view_renders() {
        let state = FlashKraft::new();
        let _view = view(&state);
        // If this compiles and runs, the view renders successfully
    }
}
