//! Step Indicators Component
//!
//! This module contains the step indicator UI that shows the three main steps
//! (Select Image, Select Target, Flash) with connecting animated progress lines.

use iced::widget::{column, container, row, text};
use iced::{Alignment, Element, Length};

use crate::core::message::Message;
use crate::core::FlashKraft;
use crate::ui::components::progress_line;
use crate::utils::icons;
use iced_fonts::bootstrap;

/// Build a step-indicator column: icon + label, centred in 220px.
macro_rules! step_indicator {
    ($icon_widget:expr, $label:expr, $active:expr, $w:expr) => {
        container(
            column![icons::icon($icon_widget, 28.0), text($label).size(13)]
                .spacing(8)
                .align_x(Alignment::Center),
        )
        .width($w)
        .padding([10, 0])
        .center_x($w)
        .style(move |theme: &iced::Theme| crate::ui::theme::card(theme, $active))
    };
}

/// Step indicators with connecting lines (cards light up as each step completes)
pub fn view_step_indicators(
    state: &FlashKraft,
    m: crate::ui::layout::Metrics,
) -> Element<'_, Message> {
    let has_image = state.selected_image.is_some();
    let has_target = state.selected_target.is_some();

    // Create step indicators - width matches button content width (220px)
    let step1 = step_indicator!(bootstrap::image(), "1 · Select Image", has_image, m.card_w);
    let step2 = step_indicator!(
        bootstrap::device_hdd(),
        "2 · Select Target",
        has_target,
        m.card_w
    );
    let step3 = step_indicator!(
        bootstrap::lightning_fill(),
        "3 · Flash!",
        has_image && has_target,
        m.card_w
    );

    // Create connecting lines - animated with theme colors
    if m.stacked {
        return container(
            column![step1, step2, step3]
                .spacing(8)
                .align_x(Alignment::Center),
        )
        .width(Length::Fill)
        .center_x(Length::Fill)
        .into();
    }

    let line1 =
        progress_line::view_progress_line(has_image, m.line_w, state.animation_time, &state.theme);
    let line2 =
        progress_line::view_progress_line(has_target, m.line_w, state.animation_time, &state.theme);

    container(row![step1, line1, step2, line2, step3].align_y(Alignment::Center))
        .width(Length::Fill)
        .center_x(Length::Fill)
        .into()
}
