//! Update Banner Component
//!
//! A small, dismissible banner shown across the top of the app when a
//! background crates.io check finds a newer release than the one currently
//! running. Auto-hides itself after [`crate::core::state::UPDATE_BANNER_DURATION`]
//! (ticked from `Message::AnimationTick` in `update.rs`) — the user can also
//! dismiss it immediately.

use iced::widget::{button, container, row, text, Space};
use iced::{Alignment, Element, Length, Theme};

use crate::core::message::Message;
use crate::core::state::UpdateBanner;

/// Render the "update available" banner, or `None` if no update is pending.
pub fn view_update_banner(banner: &UpdateBanner) -> Element<'_, Message> {
    let message = text(format!(
        "🚀 FlashKraft {} is available — you're on {}.",
        banner.latest_version,
        env!("CARGO_PKG_VERSION"),
    ))
    .size(15);

    let dismiss = button(text("✕").size(14))
        .on_press(Message::DismissUpdateBanner)
        .padding([2, 8]);

    container(
        row![message, Space::new().width(Length::Fill), dismiss]
            .align_y(Alignment::Center)
            .spacing(12)
            .padding([8, 16]),
    )
    .width(Length::Fill)
    .style(|theme: &Theme| {
        let palette = theme.extended_palette();
        container::Style {
            background: Some(palette.primary.weak.color.into()),
            text_color: Some(palette.primary.weak.text),
            ..container::Style::default()
        }
    })
    .into()
}
