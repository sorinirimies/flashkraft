//! TUI Event Handling
//!
//! Maps crossterm [`KeyEvent`]s to [`App`] state transitions.
//! Every key press is routed through [`handle_key`], which delegates
//! to a screen-specific handler keeping each case small and focused.
//!
//! ## Testing
//!
//! Every screen handler has unit tests at the bottom of this file.
//! Tests use `KeyEvent::new` to synthesise key presses without a real TTY.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::message::{AppScreen, ClipOp, FileOpMode, InputMode};
use super::state::App;
use tui_file_explorer::ExplorerOutcome;

/// Try `$call`; on error set the error message and transition to the Error screen.
macro_rules! try_or_error_screen {
    ($app:expr, $call:expr) => {
        match $call {
            Ok(()) => {}
            Err(msg) => {
                $app.error_message = msg;
                $app.screen = AppScreen::Error;
            }
        }
    };
}

/// Process a single key event and mutate `app` accordingly.
///
/// Returns `true` if the event was consumed (no further handling needed).
pub fn handle_key(app: &mut App, key: KeyEvent) -> bool {
    // ── Global shortcuts (work on every screen) ──────────────────────────────
    match key.code {
        // Ctrl-C / Ctrl-Q → quit (or cancel flash if one is running)
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            if app.screen == AppScreen::Flashing {
                app.cancel_flash();
            } else {
                app.should_quit = true;
            }
            return true;
        }
        KeyCode::Char('q') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.should_quit = true;
            return true;
        }
        // Ctrl+T cycles the global app theme everywhere — including when the
        // user is actively typing in the image-path text field.
        KeyCode::Char('t') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.next_explorer_theme();
            return true;
        }
        // T (uppercase / Shift+T) toggles the global theme panel on every screen.
        KeyCode::Char('T') if key.modifiers.is_empty() || key.modifiers == KeyModifiers::SHIFT => {
            if app.show_app_theme_panel {
                app.close_app_theme_panel();
            } else {
                app.open_app_theme_panel();
            }
            return true;
        }
        // Bare 't' cycles theme on every screen except when the user is
        // actively typing in the image-path text field (SelectImage +
        // Editing mode) — there 't' should be inserted as a character instead.
        KeyCode::Char('t') if key.modifiers.is_empty() => {
            let in_text_edit =
                app.screen == AppScreen::SelectImage && app.input_mode == InputMode::Editing;
            if !in_text_edit {
                app.next_explorer_theme();
                return true;
            }
        }
        _ => {}
    }

    // ── Global theme panel navigation (intercept before screen dispatch) ─────
    if app.show_app_theme_panel {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                app.theme_panel_up();
                return true;
            }
            KeyCode::Down | KeyCode::Char('j') => {
                app.theme_panel_down();
                return true;
            }
            KeyCode::Enter => {
                app.theme_panel_confirm();
                return true;
            }
            KeyCode::Esc => {
                app.close_app_theme_panel();
                return true;
            }
            _ => {}
        }
    }

    // ── Screen-specific handling ─────────────────────────────────────────────
    match app.screen {
        AppScreen::SelectImage => handle_select_image(app, key),
        AppScreen::BrowseImage => handle_browse_image(app, key),
        AppScreen::SelectDrive => handle_select_drive(app, key),
        AppScreen::DriveInfo => handle_drive_info(app, key),
        AppScreen::ConfirmFlash => handle_confirm_flash(app, key),
        AppScreen::Flashing => handle_flashing(app, key),
        AppScreen::Complete => handle_complete(app, key),
        AppScreen::Error => handle_error(app, key),
    }
}

// ---------------------------------------------------------------------------
// Screen: SelectImage
// ---------------------------------------------------------------------------

fn handle_select_image(app: &mut App, key: KeyEvent) -> bool {
    match app.input_mode {
        InputMode::Editing => match key.code {
            // Open the interactive file browser.
            KeyCode::Tab => {
                app.open_file_explorer();
                true
            }
            KeyCode::Char('f') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                app.open_file_explorer();
                true
            }

            // Confirm path.
            KeyCode::Enter => {
                match app.confirm_image() {
                    Ok(()) => {
                        // Kick off async drive detection.
                        app.start_drive_detection();
                    }
                    Err(msg) => {
                        app.error_message = msg;
                        app.screen = AppScreen::Error;
                    }
                }
                true
            }

            // Text editing.
            KeyCode::Char(c) => {
                app.image_insert(c);
                true
            }
            KeyCode::Backspace => {
                app.image_backspace();
                true
            }
            KeyCode::Delete => {
                // Delete character under cursor (shift cursor left first then
                // delete — simplest approach compatible with our helpers).
                app.image_cursor_right();
                app.image_backspace();
                true
            }
            KeyCode::Left => {
                app.image_cursor_left();
                true
            }
            KeyCode::Right => {
                app.image_cursor_right();
                true
            }
            KeyCode::Home => {
                app.image_cursor = 0;
                true
            }
            KeyCode::End => {
                app.image_cursor = app.image_input.chars().count();
                true
            }

            // Escape exits the app from the first screen.
            KeyCode::Esc => {
                app.should_quit = true;
                true
            }

            _ => false,
        },

        InputMode::Normal => match key.code {
            KeyCode::Enter | KeyCode::Char('i') => {
                app.input_mode = InputMode::Editing;
                true
            }
            // Open the interactive file browser from normal mode too.
            KeyCode::Tab => {
                app.open_file_explorer();
                true
            }
            KeyCode::Esc | KeyCode::Char('q') => {
                app.should_quit = true;
                true
            }
            _ => false,
        },
    }
}

// ---------------------------------------------------------------------------
// Screen: BrowseImage
// ---------------------------------------------------------------------------

fn handle_browse_image(app: &mut App, key: KeyEvent) -> bool {
    // Clear stale file-op status on any new keypress.
    app.file_op_status.clear();

    // ── Confirmation modals take priority ─────────────────────────────────
    match std::mem::replace(&mut app.file_op_mode, FileOpMode::Normal) {
        FileOpMode::ConfirmDelete(path) => {
            match key.code {
                KeyCode::Char('y') | KeyCode::Char('Y') => app.explorer_do_delete(path),
                KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                    app.file_op_status = "Delete cancelled.".to_string();
                }
                _ => {
                    app.file_op_mode = FileOpMode::ConfirmDelete(path);
                }
            }
            return true;
        }
        FileOpMode::ConfirmOverwrite { src, dst, op } => {
            match key.code {
                KeyCode::Char('y') | KeyCode::Char('Y') => {
                    app.explorer_do_paste(&src.clone(), &dst.clone(), op);
                }
                KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                    app.file_op_status = "Paste cancelled.".to_string();
                }
                _ => {
                    app.file_op_mode = FileOpMode::ConfirmOverwrite { src, dst, op };
                }
            }
            return true;
        }
        FileOpMode::Normal => {}
    }

    let searching = app.file_explorer.is_searching();

    // ── Theme controls (only outside search) ──────────────────────────────
    if !searching {
        match key.code {
            KeyCode::Char('t') if key.modifiers.is_empty() => {
                app.next_explorer_theme();
                return true;
            }
            KeyCode::Char('[') => {
                app.prev_explorer_theme();
                return true;
            }

            // ── File operations (only outside search) ─────────────────────
            KeyCode::Char('y') if key.modifiers.is_empty() => {
                app.explorer_yank(ClipOp::Copy);
                return true;
            }
            KeyCode::Char('x') if key.modifiers.is_empty() => {
                app.explorer_yank(ClipOp::Cut);
                return true;
            }
            KeyCode::Char('p') if key.modifiers.is_empty() => {
                app.explorer_initiate_paste();
                return true;
            }
            KeyCode::Char('d') if key.modifiers.is_empty() => {
                app.explorer_initiate_delete();
                return true;
            }
            _ => {}
        }
    }

    // ── Forward remaining keys to the explorer widget ─────────────────────
    let outcome = app.file_explorer.handle_key(key);
    match outcome {
        ExplorerOutcome::Selected(path) => {
            app.apply_explorer_selection(path);
            true
        }
        ExplorerOutcome::Dismissed => {
            app.screen = AppScreen::SelectImage;
            app.input_mode = InputMode::Editing;
            true
        }
        ExplorerOutcome::Pending => true,
        ExplorerOutcome::Unhandled => false,
        ExplorerOutcome::MkdirCreated(_)
        | ExplorerOutcome::TouchCreated(_)
        | ExplorerOutcome::RenameCompleted(_) => true,
    }
}

// ---------------------------------------------------------------------------
// Screen: SelectDrive
// ---------------------------------------------------------------------------

fn handle_select_drive(app: &mut App, key: KeyEvent) -> bool {
    match key.code {
        // Navigate list.
        KeyCode::Up | KeyCode::Char('k') => {
            app.drive_up();
            true
        }
        KeyCode::Down | KeyCode::Char('j') => {
            app.drive_down();
            true
        }

        // Confirm selection.
        KeyCode::Enter | KeyCode::Char(' ') => {
            if app.available_drives.is_empty() {
                return true; // nothing to select
            }
            try_or_error_screen!(app, app.confirm_drive());
            true
        }

        // Refresh drive list.
        KeyCode::Char('r') | KeyCode::F(5) => {
            app.start_drive_detection();
            true
        }

        // Go back to image selection.
        KeyCode::Backspace | KeyCode::Esc | KeyCode::Char('b') => {
            app.go_back();
            true
        }

        _ => false,
    }
}

// ---------------------------------------------------------------------------
// Screen: DriveInfo
// ---------------------------------------------------------------------------

fn handle_drive_info(app: &mut App, key: KeyEvent) -> bool {
    match key.code {
        // Advance to confirmation.
        KeyCode::Enter | KeyCode::Char('f') => {
            app.advance_to_confirm();
            true
        }

        // Go back to drive selection.
        KeyCode::Backspace | KeyCode::Esc | KeyCode::Char('b') => {
            app.go_back();
            true
        }

        _ => false,
    }
}

// ---------------------------------------------------------------------------
// Screen: ConfirmFlash
// ---------------------------------------------------------------------------

fn handle_confirm_flash(app: &mut App, key: KeyEvent) -> bool {
    match key.code {
        // Confirm and start flash.
        KeyCode::Enter | KeyCode::Char('y') | KeyCode::Char('Y') => {
            try_or_error_screen!(app, app.begin_flash());
            true
        }

        // Cancel / go back.
        KeyCode::Esc | KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Char('b') => {
            app.go_back();
            true
        }

        _ => false,
    }
}

// ---------------------------------------------------------------------------
// Screen: Flashing
// ---------------------------------------------------------------------------

fn handle_flashing(app: &mut App, key: KeyEvent) -> bool {
    match key.code {
        // Cancel the running flash operation.
        KeyCode::Char('c') | KeyCode::Esc => {
            app.cancel_flash();
            true
        }
        _ => false,
    }
}

// ---------------------------------------------------------------------------
// Screen: Complete
// ---------------------------------------------------------------------------

fn handle_complete(app: &mut App, key: KeyEvent) -> bool {
    match key.code {
        // Scroll USB contents.
        KeyCode::Up | KeyCode::Char('k') => {
            app.contents_up();
            true
        }
        KeyCode::Down | KeyCode::Char('j') => {
            app.contents_down();
            true
        }
        KeyCode::PageUp => {
            for _ in 0..10 {
                app.contents_up();
            }
            true
        }
        KeyCode::PageDown => {
            for _ in 0..10 {
                app.contents_down();
            }
            true
        }

        // Flash another image — full reset.
        KeyCode::Char('r') | KeyCode::Char('R') => {
            app.reset();
            true
        }

        // Quit.
        KeyCode::Char('q') | KeyCode::Esc => {
            app.should_quit = true;
            true
        }

        _ => false,
    }
}

// ---------------------------------------------------------------------------
// Screen: Error
// ---------------------------------------------------------------------------

fn handle_error(app: &mut App, key: KeyEvent) -> bool {
    match key.code {
        // Retry — full reset.
        KeyCode::Char('r') | KeyCode::Enter => {
            app.reset();
            true
        }

        // Quit.
        KeyCode::Char('q') | KeyCode::Esc => {
            app.should_quit = true;
            true
        }

        _ => false,
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::message::UsbEntry;
    use crate::domain::{DriveInfo, ImageInfo};

    // ── Key-event constructors ────────────────────────────────────────────────

    /// Build a key event with no modifier.
    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    /// Build a key event with Ctrl held.
    fn ctrl(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::CONTROL)
    }

    // ── Test helpers ─────────────────────────────────────────────────────────

    fn make_drive(name: &str, device: &str) -> DriveInfo {
        DriveInfo::new(name.into(), format!("/media/{name}"), 16.0, device.into())
    }

    fn app_on(screen: AppScreen) -> App {
        let mut app = App::new();
        app.screen = screen;
        app
    }

    fn app_with_drives(screen: AppScreen) -> App {
        let mut app = app_on(screen);
        app.available_drives = vec![
            make_drive("USB_A", "/dev/sdb"),
            make_drive("USB_B", "/dev/sdc"),
            make_drive("USB_C", "/dev/sdd"),
        ];
        app.selected_image = Some(ImageInfo {
            path: std::path::PathBuf::from("/tmp/test.img"),
            name: "test.img".into(),
            size_mb: 1.0,
        });
        app
    }

    fn app_complete_with_entries(n: usize) -> App {
        let mut app = app_on(AppScreen::Complete);
        app.usb_contents = (0..n)
            .map(|i| UsbEntry {
                name: format!("file_{i}"),
                size_bytes: 0,
                is_dir: false,
                depth: 0,
            })
            .collect();
        app
    }

    // ── Global shortcuts ──────────────────────────────────────────────────────

    #[test]
    fn ctrl_c_sets_should_quit_on_non_flash_screen() {
        let mut app = App::new(); // SelectImage
        let consumed = handle_key(&mut app, ctrl(KeyCode::Char('c')));
        assert!(consumed, "Ctrl-C must be consumed");
        assert!(app.should_quit, "Ctrl-C must set should_quit");
        assert_eq!(app.screen, AppScreen::SelectImage, "screen must not change");
    }

    #[test]
    fn ctrl_c_cancels_flash_when_flashing() {
        let mut app = app_on(AppScreen::Flashing);
        let consumed = handle_key(&mut app, ctrl(KeyCode::Char('c')));
        assert!(consumed);
        // cancel_flash() moves to Error, not quit
        assert!(!app.should_quit);
        assert_eq!(app.screen, AppScreen::Error);
    }

    #[test]
    fn ctrl_q_sets_should_quit_always() {
        for screen in [
            AppScreen::SelectImage,
            AppScreen::SelectDrive,
            AppScreen::DriveInfo,
            AppScreen::ConfirmFlash,
            AppScreen::Complete,
            AppScreen::Error,
        ] {
            let mut app = app_on(screen.clone());
            let consumed = handle_key(&mut app, ctrl(KeyCode::Char('q')));
            assert!(consumed, "Ctrl-Q must be consumed on {screen:?}");
            assert!(app.should_quit, "Ctrl-Q must quit on {screen:?}");
        }
    }

    // ── SelectImage — Editing mode ────────────────────────────────────────────

    #[test]
    fn select_image_editing_char_inserts_and_consumes() {
        let mut app = App::new();
        assert_eq!(app.input_mode, InputMode::Editing);
        let consumed = handle_key(&mut app, key(KeyCode::Char('a')));
        assert!(consumed);
        assert_eq!(app.image_input, "a");
        assert_eq!(app.image_cursor, 1);
    }

    #[test]
    fn select_image_editing_multiple_chars_build_string() {
        let mut app = App::new();
        for c in "hello".chars() {
            handle_key(&mut app, key(KeyCode::Char(c)));
        }
        assert_eq!(app.image_input, "hello");
        assert_eq!(app.image_cursor, 5);
    }

    #[test]
    fn select_image_editing_backspace_deletes_char() {
        let mut app = App::new();
        for c in "path".chars() {
            app.image_insert(c);
        }
        let consumed = handle_key(&mut app, key(KeyCode::Backspace));
        assert!(consumed);
        assert_eq!(app.image_input, "pat");
    }

    #[test]
    fn select_image_editing_delete_removes_char_under_cursor() {
        let mut app = App::new();
        for c in "abc".chars() {
            app.image_insert(c);
        }
        // Place cursor at position 1 (between 'a' and 'b')
        app.image_cursor = 1;
        let consumed = handle_key(&mut app, key(KeyCode::Delete));
        assert!(consumed);
        // Delete shifts right then backspaces, effectively removing 'b'
        assert_eq!(app.image_input.len(), 2);
    }

    #[test]
    fn select_image_editing_left_moves_cursor() {
        let mut app = App::new();
        app.image_insert('x');
        app.image_insert('y');
        handle_key(&mut app, key(KeyCode::Left));
        assert_eq!(app.image_cursor, 1);
    }

    #[test]
    fn select_image_editing_right_moves_cursor() {
        let mut app = App::new();
        app.image_insert('x');
        app.image_insert('y');
        app.image_cursor = 0;
        handle_key(&mut app, key(KeyCode::Right));
        assert_eq!(app.image_cursor, 1);
    }

    #[test]
    fn select_image_editing_home_moves_cursor_to_start() {
        let mut app = App::new();
        for c in "some/path".chars() {
            app.image_insert(c);
        }
        assert_eq!(app.image_cursor, 9);
        handle_key(&mut app, key(KeyCode::Home));
        assert_eq!(app.image_cursor, 0);
    }

    #[test]
    fn select_image_editing_end_moves_cursor_to_end() {
        let mut app = App::new();
        for c in "some/path".chars() {
            app.image_insert(c);
        }
        app.image_cursor = 0;
        handle_key(&mut app, key(KeyCode::End));
        assert_eq!(app.image_cursor, 9);
    }

    #[test]
    fn select_image_editing_esc_quits() {
        let mut app = App::new();
        assert_eq!(app.input_mode, InputMode::Editing);
        handle_key(&mut app, key(KeyCode::Esc));
        assert!(app.should_quit);
    }

    #[test]
    fn select_image_editing_enter_with_missing_file_goes_to_error() {
        let mut app = App::new();
        app.image_input = "/definitely/nonexistent/path.iso".into();
        handle_key(&mut app, key(KeyCode::Enter));
        assert_eq!(app.screen, AppScreen::Error);
        assert!(!app.error_message.is_empty());
    }

    #[tokio::test]
    async fn select_image_editing_enter_with_valid_file_advances() {
        use std::io::Write;
        let path = std::env::temp_dir().join("fk_evt_test.img");
        {
            let mut f = std::fs::File::create(&path).expect("create");
            f.write_all(&[0u8; 1024]).expect("write");
        }

        let mut app = App::new();
        app.image_input = path.to_string_lossy().into();
        handle_key(&mut app, key(KeyCode::Enter));
        let _ = std::fs::remove_file(&path);

        assert_eq!(app.screen, AppScreen::SelectDrive);
    }

    // ── SelectImage — Normal mode ─────────────────────────────────────────────

    #[test]
    fn select_image_normal_enter_switches_to_editing() {
        let mut app = App::new();
        app.input_mode = InputMode::Normal;
        handle_key(&mut app, key(KeyCode::Enter));
        assert_eq!(app.input_mode, InputMode::Editing);
    }

    #[test]
    fn select_image_normal_i_switches_to_editing() {
        let mut app = App::new();
        app.input_mode = InputMode::Normal;
        handle_key(&mut app, key(KeyCode::Char('i')));
        assert_eq!(app.input_mode, InputMode::Editing);
    }

    #[test]
    fn select_image_normal_esc_quits() {
        let mut app = App::new();
        app.input_mode = InputMode::Normal;
        handle_key(&mut app, key(KeyCode::Esc));
        assert!(app.should_quit);
    }

    #[test]
    fn select_image_normal_q_quits() {
        let mut app = App::new();
        app.input_mode = InputMode::Normal;
        handle_key(&mut app, key(KeyCode::Char('q')));
        assert!(app.should_quit);
    }

    // ── SelectDrive screen ────────────────────────────────────────────────────

    #[test]
    fn select_drive_down_increments_cursor() {
        let mut app = app_with_drives(AppScreen::SelectDrive);
        handle_key(&mut app, key(KeyCode::Down));
        assert_eq!(app.drive_cursor, 1);
    }

    #[test]
    fn select_drive_j_increments_cursor() {
        let mut app = app_with_drives(AppScreen::SelectDrive);
        handle_key(&mut app, key(KeyCode::Char('j')));
        assert_eq!(app.drive_cursor, 1);
    }

    #[test]
    fn select_drive_up_decrements_cursor() {
        let mut app = app_with_drives(AppScreen::SelectDrive);
        app.drive_cursor = 2;
        handle_key(&mut app, key(KeyCode::Up));
        assert_eq!(app.drive_cursor, 1);
    }

    #[test]
    fn select_drive_k_decrements_cursor() {
        let mut app = app_with_drives(AppScreen::SelectDrive);
        app.drive_cursor = 2;
        handle_key(&mut app, key(KeyCode::Char('k')));
        assert_eq!(app.drive_cursor, 1);
    }

    #[test]
    fn select_drive_enter_confirms_valid_drive() {
        let mut app = app_with_drives(AppScreen::SelectDrive);
        handle_key(&mut app, key(KeyCode::Enter));
        assert_eq!(app.screen, AppScreen::DriveInfo);
        assert!(app.selected_drive.is_some());
    }

    #[test]
    fn select_drive_space_confirms_valid_drive() {
        let mut app = app_with_drives(AppScreen::SelectDrive);
        handle_key(&mut app, key(KeyCode::Char(' ')));
        assert_eq!(app.screen, AppScreen::DriveInfo);
    }

    #[test]
    fn select_drive_enter_on_empty_list_is_noop() {
        let mut app = app_on(AppScreen::SelectDrive);
        // no drives in list
        handle_key(&mut app, key(KeyCode::Enter));
        assert_eq!(app.screen, AppScreen::SelectDrive);
    }

    #[test]
    fn select_drive_backspace_goes_back_to_select_image() {
        let mut app = app_with_drives(AppScreen::SelectDrive);
        handle_key(&mut app, key(KeyCode::Backspace));
        assert_eq!(app.screen, AppScreen::SelectImage);
        assert_eq!(app.input_mode, InputMode::Editing);
    }

    #[test]
    fn select_drive_esc_goes_back_to_select_image() {
        let mut app = app_with_drives(AppScreen::SelectDrive);
        handle_key(&mut app, key(KeyCode::Esc));
        assert_eq!(app.screen, AppScreen::SelectImage);
    }

    #[test]
    fn select_drive_b_goes_back_to_select_image() {
        let mut app = app_with_drives(AppScreen::SelectDrive);
        handle_key(&mut app, key(KeyCode::Char('b')));
        assert_eq!(app.screen, AppScreen::SelectImage);
    }

    #[tokio::test]
    async fn select_drive_refresh_retriggers_loading() {
        let mut app = app_with_drives(AppScreen::SelectDrive);
        app.drives_loading = false;
        handle_key(&mut app, key(KeyCode::Char('r')));
        // After refresh the loading flag should be set and a receiver installed
        assert!(app.drives_loading);
        assert!(app.drives_rx.is_some());
    }

    #[tokio::test]
    async fn select_drive_f5_retriggers_loading() {
        let mut app = app_with_drives(AppScreen::SelectDrive);
        app.drives_loading = false;
        handle_key(&mut app, key(KeyCode::F(5)));
        assert!(app.drives_loading);
    }

    // ── DriveInfo screen ──────────────────────────────────────────────────────

    #[test]
    fn drive_info_enter_advances_to_confirm_flash() {
        let mut app = app_on(AppScreen::DriveInfo);
        handle_key(&mut app, key(KeyCode::Enter));
        assert_eq!(app.screen, AppScreen::ConfirmFlash);
    }

    #[test]
    fn drive_info_f_advances_to_confirm_flash() {
        let mut app = app_on(AppScreen::DriveInfo);
        handle_key(&mut app, key(KeyCode::Char('f')));
        assert_eq!(app.screen, AppScreen::ConfirmFlash);
    }

    #[test]
    fn drive_info_esc_goes_back_to_select_drive() {
        let mut app = app_on(AppScreen::DriveInfo);
        handle_key(&mut app, key(KeyCode::Esc));
        assert_eq!(app.screen, AppScreen::SelectDrive);
    }

    #[test]
    fn drive_info_backspace_goes_back_to_select_drive() {
        let mut app = app_on(AppScreen::DriveInfo);
        handle_key(&mut app, key(KeyCode::Backspace));
        assert_eq!(app.screen, AppScreen::SelectDrive);
    }

    #[test]
    fn drive_info_b_goes_back_to_select_drive() {
        let mut app = app_on(AppScreen::DriveInfo);
        handle_key(&mut app, key(KeyCode::Char('b')));
        assert_eq!(app.screen, AppScreen::SelectDrive);
    }

    // ── ConfirmFlash screen ───────────────────────────────────────────────────

    #[test]
    fn confirm_flash_n_goes_back_to_drive_info() {
        let mut app = app_on(AppScreen::ConfirmFlash);
        handle_key(&mut app, key(KeyCode::Char('n')));
        assert_eq!(app.screen, AppScreen::DriveInfo);
    }

    #[test]
    fn confirm_flash_capital_n_goes_back_to_drive_info() {
        let mut app = app_on(AppScreen::ConfirmFlash);
        handle_key(&mut app, key(KeyCode::Char('N')));
        assert_eq!(app.screen, AppScreen::DriveInfo);
    }

    #[test]
    fn confirm_flash_esc_goes_back_to_drive_info() {
        let mut app = app_on(AppScreen::ConfirmFlash);
        handle_key(&mut app, key(KeyCode::Esc));
        assert_eq!(app.screen, AppScreen::DriveInfo);
    }

    #[test]
    fn confirm_flash_b_goes_back_to_drive_info() {
        let mut app = app_on(AppScreen::ConfirmFlash);
        handle_key(&mut app, key(KeyCode::Char('b')));
        assert_eq!(app.screen, AppScreen::DriveInfo);
    }

    #[test]
    fn confirm_flash_y_without_selections_shows_error() {
        // begin_flash requires image + drive; pressing y with neither set
        // should produce an error transition.
        let mut app = app_on(AppScreen::ConfirmFlash);
        handle_key(&mut app, key(KeyCode::Char('y')));
        assert_eq!(app.screen, AppScreen::Error);
        assert!(!app.error_message.is_empty());
    }

    #[test]
    fn confirm_flash_capital_y_without_selections_shows_error() {
        let mut app = app_on(AppScreen::ConfirmFlash);
        handle_key(&mut app, key(KeyCode::Char('Y')));
        assert_eq!(app.screen, AppScreen::Error);
    }

    // ── Flashing screen ───────────────────────────────────────────────────────

    #[test]
    fn flashing_c_cancels_operation() {
        let mut app = app_on(AppScreen::Flashing);
        let consumed = handle_key(&mut app, key(KeyCode::Char('c')));
        assert!(consumed);
        assert_eq!(app.screen, AppScreen::Error);
        assert!(!app.error_message.is_empty());
    }

    #[test]
    fn flashing_esc_cancels_operation() {
        let mut app = app_on(AppScreen::Flashing);
        handle_key(&mut app, key(KeyCode::Esc));
        assert_eq!(app.screen, AppScreen::Error);
    }

    #[test]
    fn flashing_other_keys_not_consumed() {
        let mut app = app_on(AppScreen::Flashing);
        let consumed = handle_key(&mut app, key(KeyCode::Char('x')));
        assert!(!consumed, "unrelated keys should not be consumed");
        assert_eq!(app.screen, AppScreen::Flashing);
    }

    // ── Complete screen ───────────────────────────────────────────────────────

    #[test]
    fn complete_down_increments_scroll() {
        let mut app = app_complete_with_entries(10);
        handle_key(&mut app, key(KeyCode::Down));
        assert_eq!(app.contents_scroll, 1);
    }

    #[test]
    fn complete_j_increments_scroll() {
        let mut app = app_complete_with_entries(10);
        handle_key(&mut app, key(KeyCode::Char('j')));
        assert_eq!(app.contents_scroll, 1);
    }

    #[test]
    fn complete_up_decrements_scroll() {
        let mut app = app_complete_with_entries(10);
        app.contents_scroll = 5;
        handle_key(&mut app, key(KeyCode::Up));
        assert_eq!(app.contents_scroll, 4);
    }

    #[test]
    fn complete_k_decrements_scroll() {
        let mut app = app_complete_with_entries(10);
        app.contents_scroll = 5;
        handle_key(&mut app, key(KeyCode::Char('k')));
        assert_eq!(app.contents_scroll, 4);
    }

    #[test]
    fn complete_page_down_scrolls_by_ten() {
        let mut app = app_complete_with_entries(25);
        handle_key(&mut app, key(KeyCode::PageDown));
        assert_eq!(app.contents_scroll, 10);
    }

    #[test]
    fn complete_page_up_scrolls_by_ten() {
        let mut app = app_complete_with_entries(25);
        app.contents_scroll = 15;
        handle_key(&mut app, key(KeyCode::PageUp));
        assert_eq!(app.contents_scroll, 5);
    }

    #[test]
    fn complete_page_up_clamps_at_zero() {
        let mut app = app_complete_with_entries(25);
        app.contents_scroll = 3;
        handle_key(&mut app, key(KeyCode::PageUp));
        assert_eq!(app.contents_scroll, 0);
    }

    #[test]
    fn complete_r_resets_to_select_image() {
        let mut app = app_complete_with_entries(5);
        handle_key(&mut app, key(KeyCode::Char('r')));
        assert_eq!(app.screen, AppScreen::SelectImage);
    }

    #[test]
    fn complete_capital_r_resets_to_select_image() {
        let mut app = app_complete_with_entries(5);
        handle_key(&mut app, key(KeyCode::Char('R')));
        assert_eq!(app.screen, AppScreen::SelectImage);
    }

    #[test]
    fn complete_q_quits() {
        let mut app = app_complete_with_entries(5);
        handle_key(&mut app, key(KeyCode::Char('q')));
        assert!(app.should_quit);
    }

    #[test]
    fn complete_esc_quits() {
        let mut app = app_complete_with_entries(5);
        handle_key(&mut app, key(KeyCode::Esc));
        assert!(app.should_quit);
    }

    // ── Error screen ──────────────────────────────────────────────────────────

    #[test]
    fn error_r_resets_to_select_image() {
        let mut app = app_on(AppScreen::Error);
        handle_key(&mut app, key(KeyCode::Char('r')));
        assert_eq!(app.screen, AppScreen::SelectImage);
    }

    #[test]
    fn error_enter_resets_to_select_image() {
        let mut app = app_on(AppScreen::Error);
        handle_key(&mut app, key(KeyCode::Enter));
        assert_eq!(app.screen, AppScreen::SelectImage);
    }

    #[test]
    fn error_q_quits() {
        let mut app = app_on(AppScreen::Error);
        handle_key(&mut app, key(KeyCode::Char('q')));
        assert!(app.should_quit);
    }

    #[test]
    fn error_esc_quits() {
        let mut app = app_on(AppScreen::Error);
        handle_key(&mut app, key(KeyCode::Esc));
        assert!(app.should_quit);
    }

    #[test]
    fn error_unrelated_key_not_consumed() {
        let mut app = app_on(AppScreen::Error);
        let consumed = handle_key(&mut app, key(KeyCode::Char('x')));
        assert!(!consumed);
        assert_eq!(app.screen, AppScreen::Error);
    }

    // ── Return value (consumed flag) ──────────────────────────────────────────

    #[test]
    fn consumed_flag_true_for_recognised_keys() {
        // Ctrl-Q is always consumed
        let mut app = App::new();
        assert!(handle_key(&mut app, ctrl(KeyCode::Char('q'))));
    }

    #[test]
    fn consumed_flag_false_for_unrecognised_keys_on_drive_info() {
        let mut app = app_on(AppScreen::DriveInfo);
        // F2 is not handled on DriveInfo
        assert!(!handle_key(&mut app, key(KeyCode::F(2))));
    }

    // ── SelectImage: Tab / Ctrl-F open the file explorer ─────────────────────

    #[test]
    fn select_image_editing_tab_opens_file_explorer() {
        let mut app = App::new();
        assert_eq!(app.input_mode, InputMode::Editing);
        let consumed = handle_key(&mut app, key(KeyCode::Tab));
        assert!(consumed);
        assert_eq!(app.screen, AppScreen::BrowseImage);
    }

    #[test]
    fn select_image_editing_ctrl_f_opens_file_explorer() {
        let mut app = App::new();
        assert_eq!(app.input_mode, InputMode::Editing);
        let consumed = handle_key(&mut app, ctrl(KeyCode::Char('f')));
        assert!(consumed);
        assert_eq!(app.screen, AppScreen::BrowseImage);
    }

    #[test]
    fn select_image_normal_tab_opens_file_explorer() {
        let mut app = App::new();
        app.input_mode = InputMode::Normal;
        let consumed = handle_key(&mut app, key(KeyCode::Tab));
        assert!(consumed);
        assert_eq!(app.screen, AppScreen::BrowseImage);
    }

    // ── BrowseImage screen ────────────────────────────────────────────────────

    #[test]
    fn browse_image_esc_returns_to_select_image() {
        let mut app = app_on(AppScreen::BrowseImage);
        let consumed = handle_key(&mut app, key(KeyCode::Esc));
        assert!(consumed);
        assert_eq!(app.screen, AppScreen::SelectImage);
        assert_eq!(app.input_mode, InputMode::Editing);
    }

    #[test]
    fn browse_image_q_dismisses_to_select_image() {
        let mut app = app_on(AppScreen::BrowseImage);
        let consumed = handle_key(&mut app, key(KeyCode::Char('q')));
        assert!(consumed);
        assert_eq!(app.screen, AppScreen::SelectImage);
        assert_eq!(app.input_mode, InputMode::Editing);
    }

    #[test]
    fn browse_image_down_is_consumed_and_stays_on_browse() {
        let mut app = app_on(AppScreen::BrowseImage);
        let consumed = handle_key(&mut app, key(KeyCode::Down));
        assert!(consumed);
        assert_eq!(app.screen, AppScreen::BrowseImage);
    }

    #[test]
    fn browse_image_j_is_consumed_and_stays_on_browse() {
        let mut app = app_on(AppScreen::BrowseImage);
        let consumed = handle_key(&mut app, key(KeyCode::Char('j')));
        assert!(consumed);
        assert_eq!(app.screen, AppScreen::BrowseImage);
    }

    #[test]
    fn browse_image_up_is_consumed_and_stays_on_browse() {
        let mut app = app_on(AppScreen::BrowseImage);
        let consumed = handle_key(&mut app, key(KeyCode::Up));
        assert!(consumed);
        assert_eq!(app.screen, AppScreen::BrowseImage);
    }

    #[test]
    fn browse_image_k_is_consumed_and_stays_on_browse() {
        let mut app = app_on(AppScreen::BrowseImage);
        let consumed = handle_key(&mut app, key(KeyCode::Char('k')));
        assert!(consumed);
        assert_eq!(app.screen, AppScreen::BrowseImage);
    }

    #[test]
    fn browse_image_page_down_is_consumed() {
        let mut app = app_on(AppScreen::BrowseImage);
        let consumed = handle_key(&mut app, key(KeyCode::PageDown));
        assert!(consumed);
        assert_eq!(app.screen, AppScreen::BrowseImage);
    }

    #[test]
    fn browse_image_page_up_is_consumed() {
        let mut app = app_on(AppScreen::BrowseImage);
        let consumed = handle_key(&mut app, key(KeyCode::PageUp));
        assert!(consumed);
        assert_eq!(app.screen, AppScreen::BrowseImage);
    }

    #[test]
    fn browse_image_unrecognised_key_not_consumed() {
        let mut app = app_on(AppScreen::BrowseImage);
        // F5 is not handled by the file explorer.
        let consumed = handle_key(&mut app, key(KeyCode::F(5)));
        assert!(!consumed);
        assert_eq!(app.screen, AppScreen::BrowseImage);
    }

    #[test]
    fn browse_image_selected_file_populates_input_and_returns() {
        use std::fs;
        let tmp = tempfile::tempdir().unwrap();
        let iso = tmp.path().join("chosen.iso");
        fs::write(&iso, b"fake").unwrap();

        let mut app = app_on(AppScreen::BrowseImage);
        // Navigate the explorer to the temp dir and point cursor at the iso.
        app.file_explorer.navigate_to(tmp.path().to_path_buf());
        let iso_idx = app
            .file_explorer
            .entries
            .iter()
            .position(|e| e.name == "chosen.iso")
            .expect("chosen.iso must be in entries");
        app.file_explorer.cursor = iso_idx;

        let consumed = handle_key(&mut app, key(KeyCode::Enter));
        assert!(consumed);
        assert_eq!(app.screen, AppScreen::SelectImage);
        assert_eq!(app.input_mode, InputMode::Editing);
        assert_eq!(app.image_input, iso.to_string_lossy().as_ref());
        assert_eq!(app.image_cursor, iso.to_string_lossy().chars().count());
    }

    #[test]
    fn browse_image_backspace_ascends_directory() {
        use std::fs;
        let tmp = tempfile::tempdir().unwrap();
        let subdir = tmp.path().join("inner");
        fs::create_dir(&subdir).unwrap();

        let mut app = app_on(AppScreen::BrowseImage);
        app.file_explorer.navigate_to(subdir.clone());
        assert_eq!(app.file_explorer.current_dir, subdir);

        let consumed = handle_key(&mut app, key(KeyCode::Backspace));
        assert!(consumed);
        // Should still be on BrowseImage, just one level up.
        assert_eq!(app.screen, AppScreen::BrowseImage);
        assert_eq!(app.file_explorer.current_dir, tmp.path());
    }

    // ── Global theme cycling (t key) ──────────────────────────────────────────

    #[test]
    fn t_cycles_theme_on_select_drive() {
        let mut app = app_on(AppScreen::SelectDrive);
        let initial_idx = app.explorer_theme_idx;
        let consumed = handle_key(&mut app, key(KeyCode::Char('t')));
        assert!(consumed, "'t' must be consumed on SelectDrive");
        assert_eq!(
            app.explorer_theme_idx,
            (initial_idx + 1) % app.explorer_themes.len(),
            "theme index must advance by 1"
        );
    }

    #[test]
    fn t_cycles_theme_on_drive_info() {
        let mut app = app_on(AppScreen::DriveInfo);
        let initial_idx = app.explorer_theme_idx;
        let consumed = handle_key(&mut app, key(KeyCode::Char('t')));
        assert!(consumed);
        assert_eq!(
            app.explorer_theme_idx,
            (initial_idx + 1) % app.explorer_themes.len()
        );
    }

    #[test]
    fn t_cycles_theme_on_confirm_flash() {
        let mut app = app_on(AppScreen::ConfirmFlash);
        let initial_idx = app.explorer_theme_idx;
        let consumed = handle_key(&mut app, key(KeyCode::Char('t')));
        assert!(consumed);
        assert_eq!(
            app.explorer_theme_idx,
            (initial_idx + 1) % app.explorer_themes.len()
        );
    }

    #[test]
    fn t_cycles_theme_on_complete() {
        let mut app = app_on(AppScreen::Complete);
        let initial_idx = app.explorer_theme_idx;
        let consumed = handle_key(&mut app, key(KeyCode::Char('t')));
        assert!(consumed);
        assert_eq!(
            app.explorer_theme_idx,
            (initial_idx + 1) % app.explorer_themes.len()
        );
    }

    #[test]
    fn t_cycles_theme_on_error() {
        let mut app = app_on(AppScreen::Error);
        let initial_idx = app.explorer_theme_idx;
        let consumed = handle_key(&mut app, key(KeyCode::Char('t')));
        assert!(consumed);
        assert_eq!(
            app.explorer_theme_idx,
            (initial_idx + 1) % app.explorer_themes.len()
        );
    }

    #[test]
    fn t_cycles_theme_on_select_image_normal_mode() {
        let mut app = App::new();
        app.input_mode = InputMode::Normal;
        let initial_idx = app.explorer_theme_idx;
        let consumed = handle_key(&mut app, key(KeyCode::Char('t')));
        assert!(
            consumed,
            "'t' must be consumed in Normal mode on SelectImage"
        );
        assert_eq!(
            app.explorer_theme_idx,
            (initial_idx + 1) % app.explorer_themes.len()
        );
    }

    #[test]
    fn t_inserts_char_on_select_image_editing_mode() {
        let mut app = App::new();
        app.input_mode = InputMode::Editing;
        let initial_idx = app.explorer_theme_idx;
        let consumed = handle_key(&mut app, key(KeyCode::Char('t')));
        assert!(
            consumed,
            "'t' must be consumed (as text input) in Editing mode"
        );
        assert_eq!(
            app.explorer_theme_idx, initial_idx,
            "theme index must NOT change when typing"
        );
        assert_eq!(
            app.image_input, "t",
            "character must be inserted into input"
        );
    }

    #[test]
    fn ctrl_t_cycles_theme_even_in_editing_mode() {
        let mut app = App::new();
        app.input_mode = InputMode::Editing;
        let initial_idx = app.explorer_theme_idx;
        let consumed = handle_key(
            &mut app,
            KeyEvent::new(KeyCode::Char('t'), KeyModifiers::CONTROL),
        );
        assert!(consumed, "Ctrl+T must be consumed in Editing mode");
        assert_eq!(
            app.explorer_theme_idx,
            (initial_idx + 1) % app.explorer_themes.len(),
            "theme index must advance even in Editing mode"
        );
        assert_eq!(
            app.image_input, "",
            "Ctrl+T must NOT insert text into the input field"
        );
    }

    #[test]
    fn ctrl_t_cycles_theme_on_every_screen() {
        for screen in [
            AppScreen::SelectImage,
            AppScreen::SelectDrive,
            AppScreen::DriveInfo,
            AppScreen::ConfirmFlash,
            AppScreen::Complete,
            AppScreen::Error,
        ] {
            let mut app = app_on(screen.clone());
            let initial_idx = app.explorer_theme_idx;
            let consumed = handle_key(
                &mut app,
                KeyEvent::new(KeyCode::Char('t'), KeyModifiers::CONTROL),
            );
            assert!(consumed, "Ctrl+T must be consumed on {screen:?}");
            assert_eq!(
                app.explorer_theme_idx,
                (initial_idx + 1) % app.explorer_themes.len(),
                "theme must advance on {screen:?}"
            );
        }
    }

    #[test]
    fn t_wraps_theme_index_around() {
        let mut app = app_on(AppScreen::SelectDrive);
        let total = app.app_themes.len();
        app.explorer_theme_idx = total - 1;
        handle_key(&mut app, key(KeyCode::Char('t')));
        assert_eq!(app.explorer_theme_idx, 0, "theme index must wrap to 0");
    }

    #[test]
    fn t_and_app_palette_stay_in_sync() {
        let mut app = app_on(AppScreen::SelectDrive);
        handle_key(&mut app, key(KeyCode::Char('t')));
        // palette() should return the palette at the new index without panicking
        let _pal = app.palette();
        let expected_name = &app.app_themes[app.explorer_theme_idx].0;
        assert_eq!(app.current_theme_name(), expected_name.as_str());
    }

    // ── Global theme panel (T key) ────────────────────────────────────────────

    #[test]
    fn capital_t_opens_theme_panel_on_any_screen() {
        for screen in [
            AppScreen::SelectImage,
            AppScreen::SelectDrive,
            AppScreen::DriveInfo,
            AppScreen::ConfirmFlash,
            AppScreen::Complete,
            AppScreen::Error,
        ] {
            let mut app = app_on(screen.clone());
            assert!(
                !app.show_app_theme_panel,
                "panel must start closed on {screen:?}"
            );
            let consumed = handle_key(&mut app, key(KeyCode::Char('T')));
            assert!(consumed, "T must be consumed on {screen:?}");
            assert!(app.show_app_theme_panel, "panel must open on {screen:?}");
        }
    }

    #[test]
    fn capital_t_closes_panel_when_already_open() {
        let mut app = app_on(AppScreen::SelectDrive);
        app.open_app_theme_panel();
        assert!(app.show_app_theme_panel);
        let consumed = handle_key(&mut app, key(KeyCode::Char('T')));
        assert!(consumed);
        assert!(!app.show_app_theme_panel, "second T must close the panel");
    }

    #[test]
    fn open_app_theme_panel_positions_cursor_at_active_theme() {
        let mut app = app_on(AppScreen::SelectDrive);
        app.explorer_theme_idx = 5;
        app.open_app_theme_panel();
        assert_eq!(
            app.app_theme_panel_cursor, 5,
            "panel cursor must start at the active theme index"
        );
    }

    #[test]
    fn theme_panel_down_moves_cursor() {
        let mut app = app_on(AppScreen::SelectDrive);
        app.open_app_theme_panel();
        let initial = app.app_theme_panel_cursor;
        let consumed = handle_key(&mut app, key(KeyCode::Down));
        assert!(consumed);
        assert_eq!(app.app_theme_panel_cursor, initial + 1);
    }

    #[test]
    fn theme_panel_j_moves_cursor_down() {
        let mut app = app_on(AppScreen::SelectDrive);
        app.open_app_theme_panel();
        let initial = app.app_theme_panel_cursor;
        let consumed = handle_key(&mut app, key(KeyCode::Char('j')));
        assert!(consumed);
        assert_eq!(app.app_theme_panel_cursor, initial + 1);
    }

    #[test]
    fn theme_panel_up_moves_cursor() {
        let mut app = app_on(AppScreen::SelectDrive);
        app.explorer_theme_idx = 3;
        app.open_app_theme_panel();
        let initial = app.app_theme_panel_cursor;
        let consumed = handle_key(&mut app, key(KeyCode::Up));
        assert!(consumed);
        assert_eq!(app.app_theme_panel_cursor, initial - 1);
    }

    #[test]
    fn theme_panel_k_moves_cursor_up() {
        let mut app = app_on(AppScreen::SelectDrive);
        app.explorer_theme_idx = 3;
        app.open_app_theme_panel();
        let initial = app.app_theme_panel_cursor;
        let consumed = handle_key(&mut app, key(KeyCode::Char('k')));
        assert!(consumed);
        assert_eq!(app.app_theme_panel_cursor, initial - 1);
    }

    #[test]
    fn theme_panel_up_clamps_at_zero() {
        let mut app = app_on(AppScreen::SelectDrive);
        app.open_app_theme_panel();
        app.app_theme_panel_cursor = 0;
        handle_key(&mut app, key(KeyCode::Up));
        assert_eq!(app.app_theme_panel_cursor, 0, "cursor must clamp at 0");
    }

    #[test]
    fn theme_panel_down_clamps_at_last() {
        let mut app = app_on(AppScreen::SelectDrive);
        app.open_app_theme_panel();
        let last = app.app_themes.len() - 1;
        app.app_theme_panel_cursor = last;
        handle_key(&mut app, key(KeyCode::Down));
        assert_eq!(
            app.app_theme_panel_cursor, last,
            "cursor must clamp at last"
        );
    }

    #[test]
    fn theme_panel_enter_applies_cursor_theme_and_closes() {
        let mut app = app_on(AppScreen::SelectDrive);
        app.explorer_theme_idx = 0; // start from a known index regardless of saved settings
        app.open_app_theme_panel();
        // Navigate down to index 4 — live preview should already set it
        for _ in 0..4 {
            handle_key(&mut app, key(KeyCode::Down));
        }
        assert_eq!(
            app.explorer_theme_idx, 4,
            "live preview must update theme during navigation"
        );
        let consumed = handle_key(&mut app, key(KeyCode::Enter));
        assert!(consumed);
        assert_eq!(
            app.explorer_theme_idx, 4,
            "active theme must remain at cursor position after Enter"
        );
        assert!(!app.show_app_theme_panel, "panel must close after Enter");
    }

    #[test]
    fn theme_panel_esc_closes_without_changing_theme() {
        let mut app = app_on(AppScreen::SelectDrive);
        app.explorer_theme_idx = 2;
        app.open_app_theme_panel();
        app.app_theme_panel_cursor = 7;
        let consumed = handle_key(&mut app, key(KeyCode::Esc));
        assert!(consumed);
        assert!(!app.show_app_theme_panel, "panel must close on Esc");
        assert_eq!(
            app.explorer_theme_idx, 2,
            "active theme must NOT change on Esc"
        );
    }

    #[test]
    fn theme_panel_navigation_live_preview_updates_active_theme() {
        // Navigation immediately updates the active theme for live preview.
        let mut app = app_on(AppScreen::SelectDrive);
        app.explorer_theme_idx = 1;
        app.open_app_theme_panel();
        handle_key(&mut app, key(KeyCode::Down));
        assert_eq!(app.explorer_theme_idx, 2, "Down must preview theme index 2");
        handle_key(&mut app, key(KeyCode::Down));
        assert_eq!(app.explorer_theme_idx, 3, "Down must preview theme index 3");
        handle_key(&mut app, key(KeyCode::Up));
        assert_eq!(
            app.explorer_theme_idx, 2,
            "Up must revert preview to index 2"
        );
    }

    #[test]
    fn theme_panel_esc_reverts_live_preview_to_original_theme() {
        // Esc must roll back the live preview to the theme that was active
        // when the panel was opened.
        let mut app = app_on(AppScreen::SelectDrive);
        app.explorer_theme_idx = 2;
        app.open_app_theme_panel();
        handle_key(&mut app, key(KeyCode::Down)); // preview index 3
        handle_key(&mut app, key(KeyCode::Down)); // preview index 4
        assert_eq!(
            app.explorer_theme_idx, 4,
            "live preview should be at 4 before Esc"
        );
        let consumed = handle_key(&mut app, key(KeyCode::Esc));
        assert!(consumed);
        assert!(!app.show_app_theme_panel, "panel must close on Esc");
        assert_eq!(
            app.explorer_theme_idx, 2,
            "Esc must revert to the theme active when the panel was opened"
        );
    }

    #[test]
    fn theme_panel_intercepts_jk_even_on_complete_screen() {
        // Normally j/k scroll USB contents on the Complete screen.
        // When the theme panel is open those keys must go to the panel instead.
        let mut app = app_complete_with_entries(20);
        app.open_app_theme_panel();
        app.app_theme_panel_cursor = 3;
        let scroll_before = app.contents_scroll;
        handle_key(&mut app, key(KeyCode::Char('j')));
        assert_eq!(
            app.app_theme_panel_cursor, 4,
            "j must move panel cursor down"
        );
        assert_eq!(
            app.contents_scroll, scroll_before,
            "j must NOT scroll USB contents while panel is open"
        );
    }
}
