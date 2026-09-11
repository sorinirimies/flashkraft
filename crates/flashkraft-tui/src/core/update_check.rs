//! Update Check Command (TUI)
//!
//! Queries crates.io — not GitHub/Gitea releases — for the latest published
//! `flashkraft` version. crates.io is used deliberately: it is the single
//! canonical source regardless of which git remote (GitHub, self-hosted
//! Gitea, …) a particular build was cut from.
//!
//! Lives in this frontend crate (not `flashkraft-core`) so the `reqwest`
//! HTTP client — and its `ring` TLS backend, which needs a C compiler for
//! the target — never gets pulled into the cross-compile-checked core
//! crate. See `flashkraft_core::update_check` for the shared, dependency-free
//! version-comparison helpers used here.

use serde::Deserialize;
use std::time::Duration;

const CRATES_IO_URL: &str = "https://crates.io/api/v1/crates/flashkraft";

/// crates.io requires a descriptive `User-Agent` on all API requests and
/// will reject generic/default ones.
const USER_AGENT: &str = concat!(
    "flashkraft-tui-update-checker/",
    env!("CARGO_PKG_VERSION"),
    " (https://github.com/sorinirimies/flashkraft)"
);

const REQUEST_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Debug, Deserialize)]
struct CratesIoResponse {
    #[serde(rename = "crate")]
    krate: CrateInfo,
}

#[derive(Debug, Deserialize)]
struct CrateInfo {
    max_stable_version: String,
}

/// Check crates.io for a newer `flashkraft` release than `current_version`.
///
/// Returns `Some(version)` if an update is available, `None` if already up
/// to date *or* the check failed for any reason (network error, timeout,
/// parse failure) — an update check must never be disruptive, so errors are
/// swallowed rather than surfaced to the user.
pub async fn check_for_update(current_version: &str) -> Option<String> {
    let client = reqwest::Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .user_agent(USER_AGENT)
        .build()
        .ok()?;

    let response: CratesIoResponse = client
        .get(CRATES_IO_URL)
        .send()
        .await
        .ok()?
        .error_for_status()
        .ok()?
        .json()
        .await
        .ok()?;

    let latest = response.krate.max_stable_version;
    if flashkraft_core::is_newer(current_version, &latest) {
        Some(latest)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_module_compiles() {
        // Network behaviour is exercised manually / in the version-comparison
        // unit tests in flashkraft_core::update_check — this just ensures
        // the module (and its reqwest wiring) builds.
    }
}
