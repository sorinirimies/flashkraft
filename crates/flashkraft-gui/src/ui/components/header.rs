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
pub fn view_header(state: &FlashKraft, m: crate::ui::layout::Metrics) -> Element<'_, Message> {
    let size = m.title_size;
    let mut brand = row![
        icons::icon(bootstrap::lightning_fill(), size - 2.0),
        text("Flash").size(size).style(|theme: &Theme| text::Style {
            color: Some(theme.palette().primary),
        }),
        text("Kraft").size(size),
    ]
    .spacing(8)
    .align_y(Alignment::Center);
    if m.show_extras {
        brand = brand.push(
            container(text(format!("v{}", env!("CARGO_PKG_VERSION"))).size(12))
                .padding([2, 10])
                .style(style::chip),
        );
    }

    row![
        brand,
        Space::new().width(Length::Fill),
        theme_selector::theme_selector(&state.theme, m.stacked),
    ]
    .align_y(Alignment::Center)
    .into()
}
