//! Theme Selector Component
//!
//! This module provides a reusable theme selector component that displays
//! all available themes and handles theme changes with persistence.

use crate::core::message::Message;
use crate::core::storage::all_themes;
use iced::widget::{container, pick_list};
use iced::{Alignment, Element, Length, Theme};

/// Create a theme selector widget
///
/// # Arguments
///
/// * `current_theme` - The currently selected theme
///
/// # Returns
///
/// An Element containing the theme picker
pub fn theme_selector(current_theme: &Theme) -> Element<'static, Message> {
    let themes = all_themes();

    pick_list(themes, Some(current_theme.clone()), Message::ThemeChanged)
        .placeholder("Select theme")
        .text_size(14.0)
        .width(Length::Fixed(200.0))
        .into()
}

/// Create a theme selector widget aligned to the right
///
/// # Arguments
///
/// * `current_theme` - The currently selected theme
///
/// # Returns
///
/// An Element containing the right-aligned theme picker
pub fn theme_selector_right(current_theme: &Theme) -> Element<'static, Message> {
    use iced::widget::{row, Space};

    container(
        row![
            Space::with_width(Length::Fill),
            theme_selector(current_theme),
        ]
        .align_y(Alignment::Start),
    )
    .padding(20)
    .into()
}
