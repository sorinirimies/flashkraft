//! Responsive helpers: terminal size classes and adaptive splits.
//!
//! Breakpoints (columns): `< 70` compact, `< 100` medium, otherwise wide.
//! Side-by-side panes stack vertically below [`STACK_BELOW`] columns.

use ratatui::layout::{Constraint, Direction, Layout, Rect};

/// Below this width the UI uses its most compact form.
pub const COMPACT_BELOW: u16 = 70;
/// Below this width secondary detail is trimmed.
pub const MEDIUM_BELOW: u16 = 100;
/// Below this width two-pane layouts stack vertically.
pub const STACK_BELOW: u16 = 80;
/// Smallest terminal we try to render real content in.
pub const MIN_WIDTH: u16 = 30;
pub const MIN_HEIGHT: u16 = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SizeClass {
    Compact,
    Medium,
    Wide,
}

pub fn size_class(width: u16) -> SizeClass {
    if width < COMPACT_BELOW {
        SizeClass::Compact
    } else if width < MEDIUM_BELOW {
        SizeClass::Medium
    } else {
        SizeClass::Wide
    }
}

/// True when the terminal is too small to draw real content.
pub fn too_small(area: Rect) -> bool {
    area.width < MIN_WIDTH || area.height < MIN_HEIGHT
}

/// Split `area` into two panes with percentage weights `a`/`b`.
/// Side by side when wide enough, stacked (same weights) otherwise.
pub fn split_two(area: Rect, a: u16, b: u16) -> [Rect; 2] {
    let dir = if area.width < STACK_BELOW {
        Direction::Vertical
    } else {
        Direction::Horizontal
    };
    let v = Layout::default()
        .direction(dir)
        .constraints([Constraint::Percentage(a), Constraint::Percentage(b)])
        .split(area);
    [v[0], v[1]]
}

/// How many of the `widths` (each already including separators) fit in `avail`.
pub fn fit_count(widths: &[usize], avail: usize) -> usize {
    let mut used = 0;
    for (i, w) in widths.iter().enumerate() {
        if used + w > avail {
            return i;
        }
        used += w;
    }
    widths.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classes() {
        assert_eq!(size_class(0), SizeClass::Compact);
        assert_eq!(size_class(69), SizeClass::Compact);
        assert_eq!(size_class(70), SizeClass::Medium);
        assert_eq!(size_class(99), SizeClass::Medium);
        assert_eq!(size_class(100), SizeClass::Wide);
    }

    #[test]
    fn split_stacks_when_narrow() {
        let [a, b] = split_two(Rect::new(0, 0, 79, 20), 60, 40);
        assert_eq!(a.x, b.x);
        assert!(b.y > a.y);
        let [a, b] = split_two(Rect::new(0, 0, 80, 20), 60, 40);
        assert_eq!(a.y, b.y);
        assert!(b.x > a.x);
    }

    #[test]
    fn split_zero_area_ok() {
        let _ = split_two(Rect::new(0, 0, 0, 0), 50, 50);
    }

    #[test]
    fn fit() {
        assert_eq!(fit_count(&[5, 5, 5], 10), 2);
        assert_eq!(fit_count(&[5, 5, 5], 100), 3);
        assert_eq!(fit_count(&[5], 0), 0);
        assert_eq!(fit_count(&[], 3), 0);
    }

    #[test]
    fn too_small_thresholds() {
        assert!(too_small(Rect::new(0, 0, 29, 20)));
        assert!(too_small(Rect::new(0, 0, 80, 9)));
        assert!(!too_small(Rect::new(0, 0, 30, 10)));
    }
}
