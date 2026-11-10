//! Header Component
//!
//! Left-aligned brand (`⚡ Flash`**Kraft** + version chip) and, on the right,
//! the theme picker plus Settings/About icon buttons — the same title-bar
//! layout as the TUI and web front-ends, with the extra controls GitKraft
//! users will recognise.

use iced::widget::{Space, button, column, container, row, text, tooltip};
use iced::{Alignment, Element, Length, Theme};

use crate::core::FlashKraft;
use crate::core::message::Message;
use crate::ui::components::theme_selector;
use crate::ui::theme as style;
use iced_fonts::bootstrap;

/// A small icon-only header button with a hover tooltip (Settings, About).
fn icon_button<'a>(
    icon: iced::widget::Text<'a>,
    label: &'static str,
    message: Message,
) -> Element<'a, Message> {
    let btn = button(icon.size(16.0))
        .padding([6, 10])
        .on_press(message)
        .style(style::ghost_button);
    tooltip(btn, text(label).size(12), tooltip::Position::Bottom)
        .style(|t| style::card(t, false))
        .into()
}

/// Application header: brand on the left, Settings/About + theme selector on the
/// right. In a narrow window (`m.stacked`) the theme picker drops to its own row and
/// the version chip is hidden, so nothing overflows.
pub fn view_header(state: &FlashKraft, m: crate::ui::layout::Metrics) -> Element<'_, Message> {
    let size = m.title_size;
    let mut brand = row![
        iced::widget::image(crate::window::brand_image())
            .width(size + 12.0)
            .height(size + 12.0),
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

    let settings_btn = icon_button(bootstrap::gear(), "Settings", Message::ToggleSettings);
    let about_btn = icon_button(bootstrap::info_circle(), "About", Message::ToggleAbout);
    let themes = theme_selector::theme_selector(&state.theme, m.stacked);

    if m.stacked {
        return column![
            row![
                brand,
                Space::new().width(Length::Fill),
                settings_btn,
                about_btn
            ]
            .spacing(style::SPACE_XS)
            .align_y(Alignment::Center),
            themes,
        ]
        .spacing(style::SPACE_SM)
        .into();
    }

    row![
        brand,
        Space::new().width(Length::Fill),
        settings_btn,
        about_btn,
        themes,
    ]
    .spacing(style::SPACE_XS)
    .align_y(Alignment::Center)
    .into()
}
