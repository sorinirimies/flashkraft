//! Connector Line Component
//!
//! A calm, static line connecting the chooser cards (image \u2192 drive \u2192
//! flash): muted when the step isn't done yet, a solid themed accent colour
//! once it is. Deliberately *not* animated \u2014 an earlier version redrew a
//! noisy travelling glow on every frame, which read as busy/distracting
//! rather than polished.

use iced::widget::{Space, container};
use iced::{Background, Border, Element, Length, Theme};

use crate::core::Message;

/// A short rounded connector line between two steps.
pub fn view_progress_line(completed: bool, width: f32) -> Element<'static, Message> {
    container(
        container(Space::new().width(Length::Fill).height(Length::Fill))
            .width(width)
            .height(3.0)
            .style(move |theme: &Theme| {
                let p = theme.extended_palette();
                container::Style {
                    background: Some(Background::Color(if completed {
                        p.primary.base.color
                    } else {
                        p.background.strong.color
                    })),
                    border: Border {
                        radius: 999.0.into(),
                        ..Border::default()
                    },
                    ..container::Style::default()
                }
            }),
    )
    .center_y(Length::Fill)
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_without_panicking_either_state() {
        let _ = view_progress_line(true, 36.0);
        let _ = view_progress_line(false, 36.0);
    }
}
