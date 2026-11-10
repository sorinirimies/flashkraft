//! Flash Complete Screen
//!
//! Shown when the flash (and optional verification) pipeline finishes
//! successfully.

use iced::widget::{Space, button, column, container, text};
use iced::{Alignment, Element};

use crate::core::FlashKraft;
use crate::core::message::Message;
use crate::ui::theme as style;
use iced_fonts::bootstrap;

/// Flash complete view
pub fn view_complete(state: &FlashKraft) -> Element<'_, Message> {
    let hero = container(bootstrap::check_circle_fill().size(40.0))
        .width(96.0)
        .height(96.0)
        .center_x(96.0)
        .center_y(96.0)
        .style(move |t: &iced::Theme| style::badge(t, style::Tone::Success));

    let complete_content = column![
        hero,
        text("Flash Complete!").size(32),
        Space::new().height(20),
        text(match &state.batch {
            Some(b) if b.targets.len() > 1 =>
                format!("All {} devices are ready to use", b.targets.len()),
            _ => "Your device is ready to use".to_string(),
        })
        .size(16)
        .style(style::muted_text),
        Space::new().height(20),
        button(text("Flash Another").size(14))
            .on_press(Message::ResetClicked)
            .padding([10, 20])
            .style(style::primary_button),
    ]
    .spacing(10)
    .align_x(Alignment::Center)
    .padding(40);

    super::status_page(
        state,
        iced::widget::scrollable(complete_content.width(iced::Length::Fill))
            .width(iced::Length::Fill)
            .into(),
    )
}
