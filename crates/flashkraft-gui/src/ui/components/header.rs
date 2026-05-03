//! Header Component
//!
//! This module contains the application header with title and theme selector.

use iced::widget::{column, container, row, text, Space};
use iced::{Alignment, Element, Length, Theme};

use crate::core::message::Message;
use crate::core::FlashKraft;
use crate::ui::components::theme_selector;
use crate::utils::icons;
use iced_fonts::bootstrap;

/// Application header with title and theme selector
pub fn view_header(state: &FlashKraft) -> Element<'_, Message> {
    // Centered title with larger text
    let title_content = container(
        column![container(
            row![
                icons::icon(bootstrap::lightning_fill(), 48.0),
                Space::new().width(20),
                text("FlashKraft").size(56).style(move |theme: &Theme| {
                    let palette = theme.palette();
                    iced::widget::text::Style {
                        color: Some(palette.primary),
                    }
                }),
            ]
            .align_y(Alignment::Center),
        )
        .center_x(Length::Fill),]
        .spacing(0),
    )
    .width(Length::Fill);

    column![
        theme_selector::theme_selector_right(&state.theme),
        title_content,
    ]
    .into()
}
