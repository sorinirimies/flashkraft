#![no_main]
use libfuzzer_sys::fuzz_target;

// The update checker compares versions fetched from crates.io: garbage in
// must mean "not newer", never a panic.
fuzz_target!(|data: &[u8]| {
    let text = String::from_utf8_lossy(data);
    let (a, b) = text.split_once('\n').unwrap_or((&text, ""));
    let _ = flashkraft_core::update_check::is_newer(a, b);
    assert!(!flashkraft_core::update_check::is_newer(a, a));
});
