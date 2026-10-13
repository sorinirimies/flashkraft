#![no_main]
use flash::{check_caller, host_without_port, is_loopback_host, token_matches, FlashConfig};
use libfuzzer_sys::fuzz_target;

// Compiled straight from the web crate's source so the fuzz binary does not
// need to link Topcoat (its link-section tricks break some linkers).
#[allow(dead_code)]
#[path = "../../crates/flashkraft-web/src/flash.rs"]
mod flash;

// Header values come straight from the network. Parsing must be total, and
// only genuine loopback hosts may ever pass the caller check.
fuzz_target!(|data: &[u8]| {
    let s = String::from_utf8_lossy(data);
    let (host, token) = s.split_once('\n').unwrap_or((&s, ""));
    let bare = host_without_port(host);
    let _ = is_loopback_host(bare);

    let cfg = FlashConfig { enabled: true, token: "0123456789abcdef0123456789abcdef".into() };
    if check_caller(&cfg, Some(host), Some(token)).is_ok() {
        assert!(is_loopback_host(host_without_port(host)));
        assert!(token_matches(&cfg.token, token));
    }
    // A disabled server must refuse everything.
    let off = FlashConfig { enabled: false, ..cfg };
    assert!(check_caller(&off, Some(host), Some(token)).is_err());
});
