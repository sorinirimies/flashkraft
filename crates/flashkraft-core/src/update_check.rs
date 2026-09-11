//! Update Checker — comparison helpers
//!
//! Pure, dependency-free helpers shared by the GUI and TUI update checkers.
//! Deliberately does **not** perform the network request itself: pulling in
//! an HTTP client (`reqwest`) here would drag its TLS backend (`ring`, which
//! needs a C compiler for the target) into `flashkraft-core`, breaking the
//! Windows/macOS cross-compile `cargo check` gates in CI. The actual
//! crates.io fetch lives in each frontend crate instead (`flashkraft-gui`
//! and `flashkraft-tui`), which are never cross-compiled.

/// Minimum time between two checks, so the app doesn't hit crates.io on
/// every single launch. Callers persist `last_checked_unix` themselves (in
/// GUI/TUI settings) and pass it to [`should_check`].
pub const CHECK_INTERVAL_SECS: u64 = 24 * 60 * 60; // once per day

/// Parse a `MAJOR.MINOR.PATCH` version string into a comparable tuple.
///
/// Ignores any pre-release/build metadata suffix (e.g. `1.2.3-beta` is
/// treated as `1.2.3`) — good enough for "is there a newer stable release"
/// comparisons without pulling in the `semver` crate.
fn parse_version(s: &str) -> Option<(u64, u64, u64)> {
    let core = s.split(['-', '+']).next().unwrap_or(s);
    let mut parts = core.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    let patch = parts.next()?.parse().ok()?;
    Some((major, minor, patch))
}

/// Return `true` if `latest` is strictly newer than `current`.
///
/// Unparseable versions are treated as "not newer" (fail safe — never nag
/// the user based on a malformed comparison).
pub fn is_newer(current: &str, latest: &str) -> bool {
    match (parse_version(current), parse_version(latest)) {
        (Some(c), Some(l)) => l > c,
        _ => false,
    }
}

/// Return `true` when enough time has passed since `last_checked_unix` to
/// perform another check (or no check has ever been recorded).
pub fn should_check(last_checked_unix: u64, now_unix: u64) -> bool {
    now_unix.saturating_sub(last_checked_unix) >= CHECK_INTERVAL_SECS
}

/// Current wall-clock time as a Unix timestamp (seconds). Falls back to `0`
/// if the system clock is before the epoch (never happens in practice).
pub fn now_unix() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_plain_versions() {
        assert_eq!(parse_version("1.5.1"), Some((1, 5, 1)));
        assert_eq!(parse_version("0.0.1"), Some((0, 0, 1)));
    }

    #[test]
    fn parses_versions_with_prerelease_suffix() {
        assert_eq!(parse_version("1.5.1-beta.1"), Some((1, 5, 1)));
        assert_eq!(parse_version("2.0.0+build.5"), Some((2, 0, 0)));
    }

    #[test]
    fn rejects_malformed_versions() {
        assert_eq!(parse_version("not-a-version"), None);
        assert_eq!(parse_version("1.5"), None);
        assert_eq!(parse_version(""), None);
    }

    #[test]
    fn detects_newer_patch_minor_major() {
        assert!(is_newer("1.5.1", "1.5.2"));
        assert!(is_newer("1.5.1", "1.6.0"));
        assert!(is_newer("1.5.1", "2.0.0"));
    }

    #[test]
    fn does_not_flag_equal_or_older_as_newer() {
        assert!(!is_newer("1.5.1", "1.5.1"));
        assert!(!is_newer("1.5.1", "1.5.0"));
        assert!(!is_newer("2.0.0", "1.9.9"));
    }

    #[test]
    fn malformed_versions_are_never_newer() {
        assert!(!is_newer("1.5.1", "not-a-version"));
        assert!(!is_newer("garbage", "1.5.1"));
    }

    #[test]
    fn should_check_respects_interval() {
        let now = 1_000_000_u64;
        assert!(should_check(0, now));
        assert!(!should_check(now - 10, now));
        assert!(should_check(now - CHECK_INTERVAL_SECS, now));
        assert!(should_check(now - CHECK_INTERVAL_SECS - 1, now));
    }
}
