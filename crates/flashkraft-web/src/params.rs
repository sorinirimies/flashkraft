//! Query-string handling: `?image=~/Downloads/debian.iso&theme=Nord`.

use topcoat::router::query_params;

/// Raw, optional query-string values. Anything unknown is ignored instead of
/// failing the request, so a stale bookmark never 400s.
#[query_params(error = redirect("?"))]
pub struct PageParams {
    /// Path of the image to check against every drive.
    pub image: Option<String>,
    /// Colour theme name (see `/api/themes`).
    pub theme: Option<String>,
}

/// `?path=` for the file browser.
#[query_params(error = redirect("?"))]
pub struct BrowseParams {
    pub path: Option<String>,
}

impl PageParams {
    /// The image path, `None` when empty or absent.
    pub fn image_path(&self) -> Option<String> {
        non_empty(self.image.as_deref())
    }
}

/// `Some(trimmed)` unless empty/whitespace.
pub fn non_empty(s: Option<&str>) -> Option<String> {
    s.map(str::trim)
        .filter(|v| !v.is_empty())
        .map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn non_empty_trims_and_drops_blanks() {
        assert_eq!(non_empty(Some("  a.iso ")).as_deref(), Some("a.iso"));
        assert_eq!(non_empty(Some("   ")), None);
        assert_eq!(non_empty(None), None);
    }
}
