//! Responsive layout metrics.
//!
//! Pure functions mapping the available width (logical px) to the sizes the
//! views use, so the behaviour is unit-testable without a window.
//!
//! * `< 720`  → compact: panels stack vertically, version chip hidden.
//! * `< 1100` → medium: side by side, connector lines shrink to fit.
//! * else     → wide: full-size cards and connector lines.

/// Smallest window size the app allows (logical px).
pub const MIN_WINDOW: (f32, f32) = (360.0, 480.0);
/// Initial window size.
pub const DEFAULT_WINDOW: (f32, f32) = (1100.0, 700.0);

pub const COMPACT_BELOW: f32 = 720.0;
pub const MEDIUM_BELOW: f32 = 1100.0;

const OUTER_PADDING: f32 = 20.0;
const MAX_CARD: f32 = 220.0;
const MAX_LINE: f32 = 200.0;
const MIN_LINE: f32 = 24.0;
const MIN_CARD: f32 = 120.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Metrics {
    /// Panels/steps stacked vertically instead of in a row.
    pub stacked: bool,
    /// Width of each step / selection card.
    pub card_w: f32,
    /// Width of connector lines between step cards (row mode only).
    pub line_w: f32,
    /// Gap between the three selection panels (row mode).
    pub gap: f32,
    /// Show secondary chrome (version chip, etc).
    pub show_extras: bool,
    /// Page padding.
    pub padding: f32,
    /// Title font size.
    pub title_size: f32,
}

impl Metrics {
    pub fn for_width(width: f32) -> Self {
        let width = if width.is_finite() {
            width.max(0.0)
        } else {
            0.0
        };
        let avail = (width - 2.0 * OUTER_PADDING).max(0.0);
        if width < COMPACT_BELOW {
            return Self {
                stacked: true,
                card_w: avail.clamp(MIN_CARD.min(avail), 320.0),
                line_w: 0.0,
                gap: 12.0,
                show_extras: false,
                padding: if width < 480.0 { 10.0 } else { 16.0 },
                title_size: if width < 480.0 { 24.0 } else { 28.0 },
            };
        }
        // Row: 3 cards + 2 lines must fit in `avail`.
        let card_w = ((avail - 2.0 * MIN_LINE) / 3.0).clamp(MIN_CARD, MAX_CARD);
        let line_w = ((avail - 3.0 * card_w) / 2.0).clamp(MIN_LINE, MAX_LINE);
        Self {
            stacked: false,
            card_w,
            line_w,
            gap: if width < MEDIUM_BELOW { 16.0 } else { 40.0 },
            show_extras: true,
            padding: OUTER_PADDING,
            title_size: 32.0,
        }
    }

    /// Total width of the three-card row including connectors.
    pub fn row_width(&self) -> f32 {
        3.0 * self.card_w + 2.0 * self.line_w
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compact_below_breakpoint() {
        assert!(Metrics::for_width(719.0).stacked);
        assert!(!Metrics::for_width(720.0).stacked);
    }

    #[test]
    fn wide_uses_full_sizes() {
        let m = Metrics::for_width(1400.0);
        assert_eq!(m.card_w, 220.0);
        assert_eq!(m.line_w, 200.0);
        assert_eq!(m.gap, 40.0);
        assert!(m.show_extras);
    }

    #[test]
    fn row_always_fits_in_available_width() {
        let mut w = COMPACT_BELOW;
        while w < 3000.0 {
            let m = Metrics::for_width(w);
            assert!(
                m.row_width() <= w - 2.0 * OUTER_PADDING + 0.01,
                "w={w} row={}",
                m.row_width()
            );
            assert!(m.card_w >= MIN_CARD && m.card_w <= MAX_CARD);
            assert!(m.line_w >= MIN_LINE && m.line_w <= MAX_LINE);
            w += 7.0;
        }
    }

    #[test]
    fn stacked_card_fits_window() {
        let mut w = MIN_WINDOW.0;
        while w < COMPACT_BELOW {
            let m = Metrics::for_width(w);
            assert!(m.card_w <= w - 2.0 * OUTER_PADDING + 0.01, "w={w}");
            assert!(m.card_w > 0.0);
            w += 3.0;
        }
    }

    #[test]
    fn sizes_are_monotonic_non_decreasing() {
        let mut prev = Metrics::for_width(720.0);
        let mut w = 721.0;
        while w < 2000.0 {
            let m = Metrics::for_width(w);
            assert!(m.card_w >= prev.card_w - 0.01);
            assert!(m.line_w >= prev.line_w - 0.01 || m.card_w > prev.card_w);
            prev = m;
            w += 1.0;
        }
    }

    #[test]
    fn degenerate_widths_do_not_panic() {
        for w in [0.0, -5.0, f32::NAN, f32::INFINITY, 1.0, 1e9] {
            let m = Metrics::for_width(w);
            assert!(m.card_w.is_finite() && m.card_w >= 0.0);
            assert!(m.line_w.is_finite());
        }
    }

    #[test]
    fn minimum_window_is_compact() {
        assert!(Metrics::for_width(MIN_WINDOW.0).stacked);
        assert!(DEFAULT_WINDOW.0 >= MEDIUM_BELOW);
    }
}
