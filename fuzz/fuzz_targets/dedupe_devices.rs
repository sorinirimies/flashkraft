#![no_main]
use libfuzzer_sys::fuzz_target;
use std::path::PathBuf;

// Device lists for multi-flash come from the UI; de-duplication must be total
// and idempotent.
fuzz_target!(|data: &[u8]| {
    let devices: Vec<PathBuf> = String::from_utf8_lossy(data).lines().map(PathBuf::from).collect();
    let once = flashkraft_core::multi_flash::dedupe_devices(&devices);
    let twice = flashkraft_core::multi_flash::dedupe_devices(&once);
    assert_eq!(once, twice);
    assert!(once.len() <= devices.len());
});
