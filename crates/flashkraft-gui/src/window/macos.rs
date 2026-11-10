//! macOS-only: the standard **Window** menu and the Dock icon.
//!
//! winit's default menu bar contains just the application menu. Without a Window menu
//! the system has nothing to hang its window commands on: *Minimize*, *Zoom*, *Move &
//! Resize* (the tiling shortcuts such as ⌃⌘←) and *Bring All to Front* all need it, which
//! is why a window from another app resizes with those shortcuts and ours did not.
//! Registering the menu with `setWindowsMenu:` is also what makes macOS add its own
//! "Move & Resize" / "Tile Window" items.
//!
//! Must run on the main thread once the application exists; `install_window_menu` and
//! `set_dock_icon` are called from an iced `window::run` callback, which is exactly that.

use objc2::sel;
use objc2_app_kit::{NSApplication, NSImage, NSMenu, NSMenuItem};
use objc2_foundation::{MainThreadMarker, NSData, ns_string};

/// Add the Window menu to the menu bar. Returns `false` when it could not (not on the
/// main thread, or no menu bar yet); `true` when the menu is there (also if it already was).
#[allow(unsafe_code, reason = "AppKit calls from the main thread")]
pub fn install_window_menu() -> bool {
    let Some(mtm) = MainThreadMarker::new() else {
        return false;
    };
    let app = NSApplication::sharedApplication(mtm);
    // SAFETY (the `unsafe` below): we hold a `MainThreadMarker`, so AppKit is being
    // used on the main thread as it requires; the selectors are standard
    // `NSWindow`/`NSApplication` actions and the titles are valid, non-null strings.
    if app.windowsMenu().is_some() {
        return true;
    }
    let Some(main_menu) = app.mainMenu() else {
        return false;
    };

    let menu = NSMenu::initWithTitle(mtm.alloc(), ns_string!("Window"));
    let item = |title, action, key| unsafe {
        NSMenuItem::initWithTitle_action_keyEquivalent(mtm.alloc(), title, Some(action), key)
    };
    // ⌘M and Zoom (no shortcut: macOS assigns the tiling ones itself).
    menu.addItem(&item(
        ns_string!("Minimize"),
        sel!(performMiniaturize:),
        ns_string!("m"),
    ));
    menu.addItem(&item(
        ns_string!("Zoom"),
        sel!(performZoom:),
        ns_string!(""),
    ));
    menu.addItem(&NSMenuItem::separatorItem(mtm));
    menu.addItem(&item(
        ns_string!("Bring All to Front"),
        sel!(arrangeInFront:),
        ns_string!(""),
    ));

    let bar_item = NSMenuItem::new(mtm);
    bar_item.setSubmenu(Some(&menu));
    main_menu.addItem(&bar_item);
    app.setWindowsMenu(Some(&menu));
    true
}

/// Set the Dock and app-switcher icon from `png`. A `.app` bundle shows its `.icns`
/// before launch already; a bare binary (`cargo install`, `cargo run`) only shows an
/// icon once this ran. Returns `false` when it could not (not on the main thread, or
/// `png` is no image AppKit can read).
#[allow(unsafe_code, reason = "AppKit calls from the main thread")]
pub fn set_dock_icon(png: &[u8]) -> bool {
    let Some(mtm) = MainThreadMarker::new() else {
        return false;
    };
    let Some(image) = NSImage::initWithData(mtm.alloc(), &NSData::with_bytes(png)) else {
        return false;
    };
    // SAFETY: on the main thread (we hold a `MainThreadMarker`), with a valid, non-nil image.
    unsafe { NSApplication::sharedApplication(mtm).setApplicationIconImage(Some(&image)) };
    true
}
