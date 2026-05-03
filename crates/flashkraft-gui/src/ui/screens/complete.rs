//! Flash Complete Screen
//!
//! Shown when the flash (and optional verification) pipeline finishes
//! successfully.

use iced::widget::{button, column, text, Space};
use iced::{Alignment, Element};

use crate::core::message::Message;
use crate::core::FlashKraft;
use crate::utils::icons;
use iced_fonts::bootstrap;

/// Flash complete view
pub fn view_complete(state: &FlashKraft) -> Element<'_, Message> {
    let complete_content = column![
        icons::icon(bootstrap::check_circle_fill(), 80.0),
        text("Flash Complete!").size(32),
        Space::new().height(20),
        text("Your device is ready to use").size(16),
        Space::new().height(20),
        button(text("Flash Another").size(14))
            .on_press(Message::ResetClicked)
            .padding(10),
    ]
    .spacing(10)
    .align_x(Alignment::Center)
    .padding(40);

    super::status_page(state, complete_content.into())
}
