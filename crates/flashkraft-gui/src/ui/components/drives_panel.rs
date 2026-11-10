//! Inline Drives Panel
//!
//! Shown on the main screen below the chooser row: a stats strip plus a live
//! list of detected drives. Without this, the home screen was just the
//! compact chooser row floating in a mostly empty 1300×700 window — this
//! gives it the same at-a-glance richness as the web dashboard's drive
//! table, and lets you pick a single target without opening the overlay.

use iced::widget::{column, container, row, text};
use iced::{Alignment, Element, Length};

use crate::core::message::Message;
use crate::domain::{DriveInfo, ImageInfo};
use crate::ui::screens::select_drive::view_device_row;
use crate::ui::theme as style;

/// A small stat card (label + value), reusing the standard card style.
fn stat<'a>(label: &'static str, value: String) -> Element<'a, Message> {
    container(
        column![
            text(label).size(11).style(style::muted_text),
            text(value).size(20),
        ]
        .spacing(2),
    )
    .padding([10, 16])
    .style(|t| style::card(t, false))
    .into()
}

/// Stats strip + live drive list for the main screen.
pub fn view_drives_panel<'a>(
    drives: &'a [DriveInfo],
    selected: &'a Option<DriveInfo>,
    image: &'a Option<ImageInfo>,
    marked: &'a [DriveInfo],
) -> Element<'a, Message> {
    let usb_count = drives.iter().filter(|d| d.usb_info.is_some()).count();
    let flashable_count = drives
        .iter()
        .filter(|d| !d.disabled && !d.is_system)
        .count();
    let total_gb: f64 = drives.iter().map(|d| d.size_gb).sum();

    let stats = row![
        stat("drives", drives.len().to_string()),
        stat("usb sticks", usb_count.to_string()),
        stat("flashable", flashable_count.to_string()),
        stat("total capacity", format!("{total_gb:.1} GB")),
    ]
    .spacing(style::SPACE_SM)
    // Wraps onto a second line in a narrow window instead of overflowing it.
    .wrap();

    let heading = text("Drives").size(13).style(style::muted_text);

    let list: Element<'_, Message> = if drives.is_empty() {
        container(
            text("No drives detected \u{2014} plug in a USB stick")
                .size(13)
                .style(style::muted_text),
        )
        .padding(30)
        .width(Length::Fill)
        .center_x(Length::Fill)
        .into()
    } else {
        let rows: Vec<Element<'_, Message>> = drives
            .iter()
            .map(|d| view_device_row(d, selected.as_ref(), image.as_ref(), marked))
            .collect();

        // Not a scrollable of its own: the main view scrolls as a whole.
        column(rows).spacing(style::SPACE_SM).into()
    };

    column![stats, heading, list]
        .spacing(style::SPACE_SM)
        .align_x(Alignment::Start)
        .width(Length::Fill)
        .into()
}
