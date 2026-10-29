//! App theme catalogue.
//!
//! `tui-nuances` is the single source of truth for colours: the UI reads
//! [`tui_nuances::Theme`] fields directly. This module only maps the
//! `flashkraft_core::THEME_NAMES` indices (shared with the GUI) to those themes.

use flashkraft_core::{THEME_COUNT, THEME_NAMES};
use tui_nuances::Theme;

/// The `tui-nuances` theme matching core preset `index` (same name).
pub(crate) fn nuances_theme(index: usize) -> Theme {
    THEME_NAMES
        .get(index)
        .and_then(|n| resolve(n))
        .unwrap_or_default()
}

/// Look up a core preset name; `Gruvbox Dark` and `Atom One Dark` are `gruvbox` and `one-dark` in `tui-nuances`.
fn resolve(name: &str) -> Option<Theme> {
    match name {
        "Gruvbox Dark" => Theme::by_name("gruvbox"),
        "Atom One Dark" => Theme::by_name("one-dark"),
        other => Theme::by_name(other),
    }
}

/// One `(name, theme)` entry per `flashkraft_core::THEME_NAMES`, in order.
pub fn all_app_themes() -> Vec<(String, Theme)> {
    (0..THEME_COUNT)
        .map(|i| (THEME_NAMES[i].to_string(), nuances_theme(i)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalogue_matches_core_names() {
        let themes = all_app_themes();
        assert_eq!(themes.len(), THEME_COUNT);
        for (i, (name, _)) in themes.iter().enumerate() {
            assert_eq!(name, THEME_NAMES[i]);
        }
    }

    #[test]
    fn every_core_name_resolves_to_a_nuances_theme() {
        for name in THEME_NAMES {
            assert!(resolve(name).is_some(), "no tui-nuances theme for {name}");
        }
    }

    #[test]
    fn cyberpunk_is_present() {
        assert!(all_app_themes().iter().any(|(n, _)| n == "Cyberpunk"));
    }

    #[test]
    fn out_of_range_index_falls_back_to_default() {
        assert_eq!(nuances_theme(usize::MAX).name, Theme::default().name);
    }
}
