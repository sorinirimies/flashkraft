//! Screen Modules
//!
//! Each submodule corresponds to a distinct screen / view of the application.

pub mod complete;
pub mod error;
pub mod flashing;
pub mod select_drive;
pub mod select_image;

use iced::widget::{column, container};
use iced::{Element, Length};

use crate::core::message::Message;
use crate::core::FlashKraft;
use crate::ui::components::theme_selector;

/// Wrap inner content in the standard status-page layout:
/// theme selector on top, centred content below.
pub(crate) fn status_page<'a>(
    state: &'a FlashKraft,
    inner: Element<'a, Message>,
) -> Element<'a, Message> {
    let content = column![
        theme_selector::theme_selector_right(&state.theme),
        container(inner)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill),
    ];
    container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}
