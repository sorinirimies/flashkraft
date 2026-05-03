//! Progress Line Component
//!
//! This module contains the animated progress line that connects step indicators.

use iced::widget::canvas::{Frame, Path, Stroke};
use iced::widget::{canvas, container};
use iced::{Color, Element, Length, Theme};
use std::f32::consts::TAU;

use crate::core::Message;

/// Draw a progress line between steps with animation
pub fn view_progress_line(
    completed: bool,
    width: f32,
    time: f32,
    theme: &Theme,
) -> Element<'static, Message> {
    container(
        canvas(ProgressLine {
            completed,
            width,
            time,
            theme: theme.clone(),
        })
        .width(width)
        .height(10.0), // Height for glow effect
    )
    .center_y(Length::Fill)
    .into()
}

struct ProgressLine {
    completed: bool,
    width: f32,
    time: f32,
    theme: Theme,
}

impl ProgressLine {
    // Simplex-like noise function for organic movement
    fn noise(&self, x: f32, time: f32) -> f32 {
        let freq1 = 2.0;
        let freq2 = 3.7;
        let freq3 = 5.1;

        let n1 = ((x * freq1 + time * 2.0).sin() * 0.5 + 0.5) * 0.4;
        let n2 = ((x * freq2 - time * 1.5).sin() * 0.5 + 0.5) * 0.3;
        let n3 = ((x * freq3 + time * 2.5).cos() * 0.5 + 0.5) * 0.3;

        n1 + n2 + n3
    }
}

impl canvas::Program<Message> for ProgressLine {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &iced::Renderer,
        _theme: &Theme,
        bounds: iced::Rectangle,
        _cursor: iced::mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let center_y = bounds.height / 2.0;

        if self.completed {
            // Animated glowing effect for completed lines

            // Use theme colors for the glow effect
            let palette = self.theme.palette();
            let primary = palette.primary;

            // Base colors derived from theme
            let color1 = Color::from_rgb(primary.r, primary.g, primary.b); // Primary theme color
            let color2 = Color::from_rgb(
                (primary.r * 0.5 + 0.5).min(1.0),
                (primary.g * 0.8 + 0.2).min(1.0),
                (primary.b * 0.9 + 0.1).min(1.0),
            ); // Accent variant
            let color3 = Color::from_rgb(
                (primary.r * 1.2).min(1.0),
                (primary.g * 1.2).min(1.0),
                (primary.b * 1.2).min(1.0),
            ); // Highlight (brighter)

            // Draw glow layers using rounded rectangles for proper rounded ends
            for glow_layer in 0..5 {
                let glow_intensity = 1.0 - (glow_layer as f32 * 0.18);
                let glow_width = 2.0 + (glow_layer as f32 * 1.2);

                // Draw segments along the line for animated effect
                let segments = 50;
                for i in 0..segments {
                    let t = i as f32 / segments as f32;
                    let x = t * self.width;
                    let next_t = (i + 1) as f32 / segments as f32;
                    let next_x = next_t * self.width;

                    // Calculate noise-based intensity
                    let noise_val = self.noise(t * 3.0, self.time);

                    // Traveling light effect (left to right)
                    // Subtract position from time for left-to-right movement
                    let travel = (self.time * 1.2 - t * TAU).sin() * 0.5 + 0.5;
                    let travel_intensity = (travel * travel) * 0.7 + 0.3;

                    // Mix colors based on position and time (left to right)
                    let color_mix = (self.time * 0.3 - t * 2.0).sin() * 0.5 + 0.5;
                    let r = color1.r * (1.0 - color_mix) + color2.r * color_mix;
                    let g = color1.g * (1.0 - color_mix) + color2.g * color_mix;
                    let b = color1.b * (1.0 - color_mix) + color2.b * color_mix;

                    // Add highlight
                    let highlight = if travel > 0.7 {
                        (travel - 0.7) * 3.0
                    } else {
                        0.0
                    };
                    let final_r = (r + color3.r * highlight).min(1.0);
                    let final_g = (g + color3.g * highlight).min(1.0);
                    let final_b = (b + color3.b * highlight).min(1.0);

                    // Calculate final alpha with all effects
                    let base_alpha = glow_intensity * noise_val * travel_intensity;

                    let line_segment = Path::line(
                        iced::Point::new(x, center_y),
                        iced::Point::new(next_x, center_y),
                    );

                    frame.stroke(
                        &line_segment,
                        Stroke::default()
                            .with_color(Color::from_rgba(final_r, final_g, final_b, base_alpha))
                            .with_width(glow_width)
                            .with_line_cap(iced::widget::canvas::LineCap::Round),
                    );
                }
            }

            // Draw core bright line using rounded rectangle for perfectly rounded ends
            let palette = self.theme.palette();
            let primary = palette.primary;
            let core_color = Color::from_rgb(
                (primary.r * 1.1).min(1.0),
                (primary.g * 1.1).min(1.0),
                (primary.b * 1.1).min(1.0),
            );

            let line_height = 1.0;
            let rounded_rect = Path::rounded_rectangle(
                iced::Point::new(0.0, center_y - line_height / 2.0),
                iced::Size::new(self.width, line_height),
                iced::border::Radius::from(line_height / 2.0), // border radius = half height for fully rounded ends
            );
            frame.fill(&rounded_rect, core_color);
        } else {
            // Simple gray line with rounded ends for incomplete state
            let gray_color = Color::from_rgb(0.4, 0.4, 0.4);
            let line_height = 1.0;

            let rounded_rect = Path::rounded_rectangle(
                iced::Point::new(0.0, center_y - line_height / 2.0),
                iced::Size::new(self.width, line_height),
                iced::border::Radius::from(line_height / 2.0), // border radius = half height for fully rounded ends
            );
            frame.fill(&rounded_rect, gray_color);
        }

        vec![frame.into_geometry()]
    }
}
