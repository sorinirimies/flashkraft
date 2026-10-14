#![no_main]
use libfuzzer_sys::fuzz_target;

// Compiled from the web crate's source (see web_validate).
#[allow(dead_code)]
#[path = "../../crates/flashkraft-web/src/browse.rs"]
mod browse;

// The path comes from the browser. Listing must never panic, only ever show
// visible folders/images, and stay within the entry cap.
fuzz_target!(|data: &[u8]| {
    let path = String::from_utf8_lossy(data);
    if let Ok(l) = browse::list_dir(&path) {
        assert!(std::path::Path::new(&l.path).is_absolute());
        assert!(l.entries.len() <= browse::MAX_ENTRIES);
        for e in &l.entries {
            assert!(!e.name.starts_with('.'));
            assert!(e.kind == "dir" || browse::is_image_name(&e.name));
        }
    }
});
