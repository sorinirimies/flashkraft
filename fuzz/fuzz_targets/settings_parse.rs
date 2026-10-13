#![no_main]
use libfuzzer_sys::fuzz_target;

// Settings files are user-editable JSON: parsing and merging history must
// never panic, whatever the bytes.
fuzz_target!(|data: &[u8]| {
    if let Ok(text) = std::str::from_utf8(data) {
        let a = flashkraft_core::settings::parse(text);
        let b = flashkraft_core::settings::parse(text);
        let merged = flashkraft_core::settings::merge_history([a.flash_history, b.flash_history]);
        assert!(merged.len() <= flashkraft_core::domain::flash_history::MAX_HISTORY_ENTRIES);
    }
});
