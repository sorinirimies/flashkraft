//! Settings panel \u2014 currently just UI scale (zoom), shown as a centred
//! modal over a dimmed backdrop, the same pattern as the About dialog.

use iced::widget::{Space, button, column, container, mouse_area, row, text};
use iced::{Alignment, Element, Length};

use crate::core::FlashKraft;
use crate::core::message::Message;
use crate::ui::theme as style;
use crate::utils::icons;
use iced_fonts::bootstrap;

/// Render the Settings panel.
pub fn view_settings(state: &FlashKraft) -> Element<'_, Message> {
    let close_btn = button(text("Close").size(13))
        .padding([6, 18])
        .style(style::ghost_button)
        .on_press(Message::ToggleSettings);

    let step_btn = |icon: iced::widget::Text<'static>, msg: Message| {
        button(icon.size(16.0))
            .padding([6, 12])
            .style(style::ghost_button)
            .on_press(msg)
    };

    let scale_pct = (state.ui_scale * 100.0).round() as i32;

    let zoom_row = row![
        step_btn(bootstrap::dash(), Message::ZoomOut),
        container(text(format!("{scale_pct}%")).size(16))
            .width(64)
            .center_x(64),
        step_btn(bootstrap::plus(), Message::ZoomIn),
        Space::new().width(style::SPACE_SM),
        button(text("Reset").size(13))
            .padding([6, 14])
            .style(style::ghost_button)
            .on_press(Message::ZoomReset),
    ]
    .spacing(style::SPACE_SM)
    .align_y(Alignment::Center);

    let content = column![
        row![
            icons::icon(bootstrap::gear_fill(), 22.0),
            text("Settings").size(20),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
        Space::new().height(16),
        text("UI scale").size(13).style(style::muted_text),
        text("Ctrl/Cmd + / Ctrl/Cmd - also work, anywhere in the window.")
            .size(11)
            .style(style::muted_text),
        Space::new().height(6),
        zoom_row,
        Space::new().height(20),
        container(close_btn)
            .width(Length::Fill)
            .align_x(Alignment::End),
    ]
    .spacing(4)
    .width(Length::Fill);

    let panel = container(content)
        .width(Length::Fill)
        .max_width(360)
        .padding(24)
        .style(|t| style::card(t, false));

    let backdrop = mouse_area(
        container(Space::new().width(Length::Fill).height(Length::Fill)).style(style::backdrop),
    )
    .on_press(Message::ToggleSettings);

    // Swallow clicks on the panel itself so they don't dismiss it.
    let panel = mouse_area(panel).on_press(Message::Noop);

    let centered = container(panel)
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill);

    iced::widget::stack![backdrop, centered].into()
}
