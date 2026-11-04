//! `demo::enable` switches demo mode on for the whole process. It lives in its
//! own test binary: the flag cannot be reset, so it must not leak into the
//! unit tests that expect real drive detection.

use flashkraft_core::commands::demo;

#[test]
fn enable_turns_demo_mode_on_without_the_environment_variable() {
    assert!(
        std::env::var_os(demo::DEMO_ENV).is_none(),
        "run without {}",
        demo::DEMO_ENV
    );
    assert!(!demo::is_demo());
    demo::enable();
    assert!(demo::is_demo());
    // The fake drives come back instead of the hardware.
    assert_eq!(
        flashkraft_core::commands::load_drives_sync(),
        demo::demo_drives()
    );
}
