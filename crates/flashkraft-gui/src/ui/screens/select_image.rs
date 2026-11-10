//! Selection Panels Component
//!
//! Compact horizontal "chooser bar": icon badge + two lines of text per
//! card, image → target → flash, connected by a calm static line so the
//! flow reads left to right and lights up as each step completes. This *is*
//! the step indicator — there is no separate row of near-identical cards
//! duplicating it above.

use iced::widget::{button, column, container, row, text};
use iced::{Alignment, Element, Length, Theme};

use crate::core::message::Message;
use crate::domain::{DriveInfo, ImageInfo};
use crate::ui::components::progress_line;
use crate::ui::theme as style;
use iced_fonts::bootstrap;

/// Icon size inside a chooser card's badge.
const ICON_SIZE: f32 = 20.0;
/// Diameter of the round icon badge.
const BADGE_SIZE: f32 = 36.0;
/// Longest connecting line between cards: they link the cards, they are not a rule.
const MAX_LINE: f32 = 40.0;

/// Three buttons section
pub fn view_buttons<'a>(
    selected_image: &'a Option<ImageInfo>,
    selected_target: &'a Option<DriveInfo>,
    marked: &'a [DriveInfo],
    is_ready_to_flash: bool,
    m: crate::ui::layout::Metrics,
) -> Element<'a, Message> {
    let has_image = selected_image.is_some();
    let has_target = selected_target.is_some() || !marked.is_empty();

    let image_button = view_image_section(selected_image, m.card_w);
    let target_button = view_target_section(selected_target, marked, m.card_w);
    let flash_button = view_flash_section(is_ready_to_flash, m.card_w);

    // Narrow window: the three cards stack; otherwise a row joined by short lines.
    let buttons: Element<'a, Message> = if m.stacked {
        column![image_button, target_button, flash_button]
            .spacing(m.gap)
            .align_x(Alignment::Center)
            .into()
    } else {
        let line1 = progress_line::view_progress_line(has_image, m.line_w.min(MAX_LINE));
        let line2 = progress_line::view_progress_line(has_target, m.line_w.min(MAX_LINE));
        row![image_button, line1, target_button, line2, flash_button]
            .spacing(style::SPACE_SM)
            .align_y(Alignment::Center)
            .into()
    };

    let mut content = column![buttons]
        .spacing(style::SPACE_MD)
        .align_x(Alignment::Center);

    // Show cancel button if something is selected
    if selected_image.is_some() || selected_target.is_some() {
        content = content.push(
            button(text("Cancel").size(14))
                .on_press(Message::CancelClicked)
                .padding(10)
                .style(style::ghost_button),
        );
    }

    container(content)
        .width(Length::Fill)
        .center_x(Length::Fill)
        .into()
}

/// A compact icon-badge-left, text-right chooser card.
fn chooser_card<'a>(
    icon: iced::widget::Text<'a, Theme, iced::Renderer>,
    title: String,
    subtitle: String,
    active: bool,
    width: f32,
) -> container::Container<'a, Message> {
    let badge = container(icon.size(ICON_SIZE))
        .width(BADGE_SIZE)
        .height(BADGE_SIZE)
        .center_x(BADGE_SIZE)
        .center_y(BADGE_SIZE)
        .style(move |t: &Theme| style::icon_badge(t, active));

    let text_col = column![
        text(title).size(14),
        text(subtitle).size(11).style(style::muted_text),
    ]
    .spacing(2);

    container(
        row![badge, text_col]
            .spacing(style::SPACE_SM)
            .align_y(Alignment::Center),
    )
    .width(width)
    .padding([10, 12])
    .style(move |t| style::card(t, active))
}

/// Image selection panel
fn view_image_section(image: &Option<ImageInfo>, width: f32) -> Element<'_, Message> {
    let active = image.is_some();
    let card = if let Some(img) = image {
        chooser_card(
            bootstrap::check_circle(),
            img.name.clone(),
            flashkraft_core::fmt_bytes(img.size_bytes()),
            active,
            width,
        )
    } else {
        chooser_card(
            bootstrap::plus_circle(),
            "Flash from file".into(),
            "Click to select".into(),
            active,
            width,
        )
    };

    button(card)
        .on_press(Message::SelectImageClicked)
        .padding(0)
        .style(move |t, s| style::card_button(t, s, active))
        .into()
}

/// Target drive selection panel
fn view_target_section<'a>(
    selected: &'a Option<DriveInfo>,
    marked: &'a [DriveInfo],
    width: f32,
) -> Element<'a, Message> {
    let active = selected.is_some() || !marked.is_empty();
    let total: f64 = marked.iter().map(|d| d.size_gb).sum();

    let card = if marked.len() > 1 {
        chooser_card(
            bootstrap::collection_fill(),
            format!("{} drives", marked.len()),
            format!("{total:.1} GB total"),
            active,
            width,
        )
    } else if let Some(target) = marked.first().or(selected.as_ref()) {
        chooser_card(
            bootstrap::check_circle(),
            target.name.clone(),
            flashkraft_core::fmt_bytes(target.size_bytes()),
            active,
            width,
        )
    } else {
        chooser_card(
            bootstrap::device_hdd(),
            "Select target".into(),
            "Click to choose".into(),
            active,
            width,
        )
    };

    button(card)
        .on_press(Message::OpenDeviceSelection)
        .padding(0)
        .style(move |t, s| style::card_button(t, s, active))
        .into()
}

/// Flash button panel
fn view_flash_section(is_ready: bool, width: f32) -> Element<'static, Message> {
    let card = chooser_card(
        bootstrap::lightning_fill(),
        "Flash!".into(),
        if is_ready {
            "Ready to flash".into()
        } else {
            "Select image & target".into()
        },
        false,
        width,
    );

    let btn = button(card).padding(0).style(move |t, s| {
        if is_ready {
            style::primary_button(t, s)
        } else {
            style::card_button(t, s, false)
        }
    });

    if is_ready {
        btn.on_press(Message::FlashClicked).into()
    } else {
        btn.into()
    }
}
