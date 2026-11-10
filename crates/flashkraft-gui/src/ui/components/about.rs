//! About dialog \u2014 app name, version, author and project info, shown as a
//! centred modal over a dimmed backdrop.

use iced::widget::{Space, button, column, container, mouse_area, row, text};
use iced::{Alignment, Element, Length};

use crate::core::message::Message;
use crate::ui::theme as style;

const AUTHOR_NAME: &str = "Sorin Irimies";
const AUTHOR_URL: &str = "https://github.com/sorinirimies";
const REPO_URL: &str = env!("CARGO_PKG_REPOSITORY");
const LICENSE: &str = env!("CARGO_PKG_LICENSE");
const DESCRIPTION: &str = env!("CARGO_PKG_DESCRIPTION");

/// Render the About dialog as a centred modal over a dimmed backdrop.
pub fn view_about() -> Element<'static, Message> {
    let link = |label: &'static str, url: &'static str| {
        button(text(label).size(13).style(|t: &iced::Theme| text::Style {
            color: Some(t.palette().primary),
        }))
        .padding(0)
        .style(style::ghost_button)
        .on_press(Message::OpenUrl(url.to_string()))
    };

    let info_row = |key: &'static str, value: Element<'static, Message>| {
        row![
            container(text(key).size(13).style(style::muted_text)).width(80),
            value
        ]
        .align_y(Alignment::Center)
    };

    let close_btn = button(text("Close").size(13))
        .padding([6, 18])
        .style(style::ghost_button)
        .on_press(Message::ToggleAbout);

    let content = column![
        row![
            iced::widget::image(crate::window::brand_image())
                .width(56)
                .height(56),
            text("FlashKraft").size(24),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
        text(format!("v{}", env!("CARGO_PKG_VERSION")))
            .size(13)
            .style(style::muted_text),
        Space::new().height(10),
        text(DESCRIPTION).size(13),
        Space::new().height(14),
        info_row("Author", text(AUTHOR_NAME).size(13).into()),
        info_row("GitHub", link("@sorinirimies", AUTHOR_URL).into()),
        info_row("Project", link(REPO_URL, REPO_URL).into()),
        info_row("License", text(LICENSE).size(13).into()),
        Space::new().height(16),
        container(close_btn)
            .width(Length::Fill)
            .align_x(Alignment::End),
    ]
    .spacing(6)
    .width(Length::Fill);

    let panel = container(content)
        .width(Length::Fill)
        .max_width(420)
        .padding(24)
        .style(|t| style::card(t, false));

    let backdrop = mouse_area(
        container(Space::new().width(Length::Fill).height(Length::Fill)).style(style::backdrop),
    )
    .on_press(Message::ToggleAbout);

    // Swallow clicks on the panel itself so they don't dismiss the dialog.
    let panel = mouse_area(panel).on_press(Message::Noop);

    let centered = container(panel)
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill);

    iced::widget::stack![backdrop, centered].into()
}
