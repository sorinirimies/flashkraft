//! Animated Progress Bar with Canvas Effects
//!
//! This module provides a custom animated progress bar using Iced's canvas
//! for smooth, animated gradient and shimmer effects during flash operations.

use iced::widget::canvas::{self, Cache, Frame, Geometry, Path, Stroke};
use iced::{Color, Element, Length, Point, Rectangle, Renderer, Size, Theme};

/// Animated progress bar with shimmer effect
#[derive(Debug)]
pub struct AnimatedProgress {
    /// Progress value (0.0 to 1.0)
    progress: f32,
    /// Animation time for shimmer effect
    animation_time: f32,
    /// Cache for the progress bar rendering
    cache: Cache,
    /// Theme for color palette
    theme: Theme,
    /// Optional color override — when `Some`, replaces the theme primary color.
    /// Used to render the verification bar in green instead of the accent color.
    color_override: Option<Color>,
}

impl AnimatedProgress {
    /// Create a new animated progress bar
    pub fn new() -> Self {
        Self {
            progress: 0.0,
            animation_time: 0.0,
            cache: Cache::new(),
            theme: Theme::Dark,
            color_override: None,
        }
    }

    /// Create a new animated progress bar with a fixed color override.
    pub fn new_with_color(color: Color) -> Self {
        Self {
            progress: 0.0,
            animation_time: 0.0,
            cache: Cache::new(),
            theme: Theme::Dark,
            color_override: Some(color),
        }
    }

    /// Set or clear the color override at runtime.
    pub fn set_color_override(&mut self, color: Option<Color>) {
        self.color_override = color;
        self.cache.clear();
    }

    /// Update the progress value
    pub fn set_progress(&mut self, progress: f32) {
        let new_progress = progress.clamp(0.0, 1.0);
        if (self.progress - new_progress).abs() > 0.001 {
            self.progress = new_progress;
            self.cache.clear();
        }
    }

    /// Update animation time based on transfer speed
    ///
    /// # Arguments
    /// * `speed_mb_s` - Current transfer speed in MB/s to scale animation speed
    pub fn tick(&mut self, speed_mb_s: f32) {
        // Scale animation speed based on transfer rate
        // - At 1 MB/s: 0.15x speed (slow)
        // - At 20 MB/s: 0.5x speed (baseline)
        // - At 100+ MB/s: 1.2x speed (fast, capped)
        let speed_multiplier = (speed_mb_s / 20.0).clamp(0.15, 1.2);
        self.animation_time += 0.016 * speed_multiplier;
        if self.animation_time > 1000.0 {
            self.animation_time = 0.0;
        }
        self.cache.clear();
    }

    /// Set the theme for color rendering
    pub fn set_theme(&mut self, theme: Theme) {
        self.theme = theme;
        self.cache.clear();
    }

    /// Return a clone of this bar's color override (if any).
    pub fn color_override(&self) -> Option<Color> {
        self.color_override
    }

    /// Create the widget view
    pub fn view<'a, Message: 'a>(&'a self) -> Element<'a, Message, Theme, Renderer> {
        iced::widget::canvas(self)
            .width(Length::Fill)
            .height(Length::Fixed(20.0))
            .into()
    }
}

impl Default for AnimatedProgress {
    fn default() -> Self {
        Self::new()
    }
}

impl<Message> canvas::Program<Message, Theme, Renderer> for AnimatedProgress {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: iced::mouse::Cursor,
    ) -> Vec<Geometry<Renderer>> {
        let geometry = self.cache.draw(renderer, bounds.size(), |frame| {
            draw_animated_progress(
                frame,
                bounds.size(),
                self.progress,
                self.animation_time,
                &self.theme,
                self.color_override,
            );
        });

        vec![geometry]
    }
}

/// Draw the animated progress bar with effects
fn draw_animated_progress(
    frame: &mut Frame,
    size: Size,
    progress: f32,
    time: f32,
    theme: &Theme,
    color_override: Option<Color>,
) {
    let width = size.width;
    let height = size.height;

    // Background (dark gray)
    let background = Path::rectangle(Point::ORIGIN, size);
    frame.fill(&background, Color::from_rgb(0.15, 0.15, 0.15));

    if progress > 0.0 {
        let progress_width = width * progress;

        // Main progress bar with gradient effect
        draw_gradient_progress(frame, progress_width, height, time, theme, color_override);

        // Add shimmer/shine effect
        draw_shimmer_effect(frame, progress_width, height, time);

        // Add subtle pulse effect at the edge
        if progress < 1.0 {
            draw_edge_pulse(frame, progress_width, height, time);
        }
    }

    // Border
    let border = Path::rectangle(Point::ORIGIN, size);
    frame.stroke(
        &border,
        Stroke::default()
            .with_color(Color::from_rgb(0.3, 0.3, 0.3))
            .with_width(1.0),
    );
}

/// Draw gradient progress fill
fn draw_gradient_progress(
    frame: &mut Frame,
    width: f32,
    height: f32,
    time: f32,
    theme: &Theme,
    color_override: Option<Color>,
) {
    // Use override color when provided (e.g. green for verification),
    // otherwise fall back to the theme primary.
    let primary = color_override.unwrap_or_else(|| theme.palette().primary);

    // Create color variants from the base color
    let color_start = Color::from_rgb(primary.r, primary.g, primary.b);
    let color_end = Color::from_rgb(
        (primary.r * 0.7 + 0.3).min(1.0),
        (primary.g * 0.9 + 0.1).min(1.0),
        (primary.b * 0.95 + 0.05).min(1.0),
    );
    // Create a multi-segment gradient effect
    let segments = 20;
    let segment_width = width / segments as f32;

    for i in 0..segments {
        let x = i as f32 * segment_width;
        let progress_ratio = i as f32 / segments as f32;

        // Animated color shifting
        let hue_shift = (time * 0.12 + progress_ratio * 2.0).sin() * 0.06;

        // Theme-based gradient with animation
        let base_color = interpolate_color(color_start, color_end, progress_ratio);

        let animated_color = Color {
            r: (base_color.r + hue_shift).clamp(0.0, 1.0),
            g: (base_color.g + hue_shift * 0.5).clamp(0.0, 1.0),
            b: (base_color.b - hue_shift * 0.3).clamp(0.0, 1.0),
            a: 1.0,
        };

        let segment = Path::rectangle(Point::new(x, 0.0), Size::new(segment_width + 1.0, height));
        frame.fill(&segment, animated_color);
    }
}

/// Draw shimmer/shine effect
fn draw_shimmer_effect(frame: &mut Frame, width: f32, height: f32, time: f32) {
    // Moving shine effect
    let shine_position = (time * 0.18).fract();
    let shine_x = width * shine_position;
    let shine_width = width * 0.15;

    // Create shimmer with gradient
    let shimmer_segments = 10;
    for i in 0..shimmer_segments {
        let offset = i as f32 / shimmer_segments as f32;
        let x = shine_x + (offset - 0.5) * shine_width;

        if x >= 0.0 && x <= width {
            // Gaussian-like falloff
            let distance_from_center = ((offset - 0.5) * 2.0).abs();
            let alpha = (1.0 - distance_from_center.powf(2.0)) * 0.3;

            let shimmer = Path::rectangle(
                Point::new(x, 0.0),
                Size::new(shine_width / shimmer_segments as f32, height),
            );

            frame.fill(
                &shimmer,
                Color {
                    r: 1.0,
                    g: 1.0,
                    b: 1.0,
                    a: alpha,
                },
            );
        }
    }
}

/// Draw pulsing effect at the leading edge
fn draw_edge_pulse(frame: &mut Frame, width: f32, height: f32, time: f32) {
    let pulse = (time * 1.8).sin() * 0.5 + 0.5;
    let pulse_width = 4.0 + pulse * 2.0;

    // Glowing edge
    let edge = Path::rectangle(
        Point::new(width - pulse_width / 2.0, 0.0),
        Size::new(pulse_width, height),
    );

    frame.fill(
        &edge,
        Color {
            r: 1.0,
            g: 1.0,
            b: 1.0,
            a: 0.3 + pulse * 0.3,
        },
    );
}

/// Interpolate between two colors
fn interpolate_color(start: Color, end: Color, t: f32) -> Color {
    Color {
        r: start.r + (end.r - start.r) * t,
        g: start.g + (end.g - start.g) * t,
        b: start.b + (end.b - start.b) * t,
        a: start.a + (end.a - start.a) * t,
    }
}
