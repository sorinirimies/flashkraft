//! Header Component
//!
//! Left-aligned brand (`⚡ Flash`**Kraft** + version chip) and the theme picker
//! on the right — the same title-bar layout as the TUI and web front-ends.

use iced::widget::{container, row, text, Space};
use iced::{Alignment, Element, Length, Theme};

use crate::core::message::Message;
use crate::core::FlashKraft;
use crate::ui::components::theme_selector;
use crate::ui::theme as style;
use crate::utils::icons;
use iced_fonts::bootstrap;

/// Application header: brand on the left, theme selector on the right.
pub fn view_header(state: &FlashKraft) -> Element<'_, Message> {
    let brand = row![
        icons::icon(bootstrap::lightning_fill(), 30.0),
        text("Flash").size(32).style(|theme: &Theme| text::Style {
            color: Some(theme.palette().primary),
        }),
        text("Kraft").size(32),
        container(text(format!("v{}", env!("CARGO_PKG_VERSION"))).size(12))
            .padding([2, 10])
            .style(style::chip),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    row![
        brand,
        Space::new().width(Length::Fill),
        theme_selector::theme_selector(&state.theme),
    ]
    .align_y(Alignment::Center)
    .into()
}
