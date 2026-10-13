#![no_main]
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use flashkraft_tui::{App, handle_key};
use libfuzzer_sys::fuzz_target;
use std::sync::OnceLock;

static RT: OnceLock<tokio::runtime::Runtime> = OnceLock::new();

fn key(b: [u8; 2]) -> KeyEvent {
    let code = match b[0] % 24 {
        0 => KeyCode::Enter, 1 => KeyCode::Esc, 2 => KeyCode::Backspace, 3 => KeyCode::Tab,
        4 => KeyCode::Up, 5 => KeyCode::Down, 6 => KeyCode::Left, 7 => KeyCode::Right,
        8 => KeyCode::Home, 9 => KeyCode::End, 10 => KeyCode::PageUp, 11 => KeyCode::PageDown,
        12 => KeyCode::Delete, 13 => KeyCode::Char(' '), 14 => KeyCode::Char('/'),
        15 => KeyCode::Char('m'), 16 => KeyCode::Char('r'), 17 => KeyCode::Char('f'),
        18 => KeyCode::Char('n'), 19 => KeyCode::Char('y'),
        _ => KeyCode::Char((b'a' + (b[0] % 26)) as char),
    };
    let mods = KeyModifiers::from_bits_truncate(b[1] & 0b111);
    KeyEvent::new(code, mods)
}

// Random key sequences must never panic the TUI state machine. Demo mode and a
// throw-away HOME guarantee no real drive is detected or written.
fuzz_target!(|data: &[u8]| {
    static INIT: std::sync::Once = std::sync::Once::new();
    INIT.call_once(|| {
        let home = std::env::temp_dir().join("flashkraft-fuzz-tui");
        let _ = std::fs::create_dir_all(&home);
        // SAFETY: before any other thread exists.
        unsafe {
            std::env::set_var("FLASHKRAFT_DEMO", "1");
            std::env::set_var("HOME", &home);
            std::env::set_var("XDG_CONFIG_HOME", &home);
        }
    });
    let rt = RT.get_or_init(|| tokio::runtime::Builder::new_multi_thread().worker_threads(1).enable_all().build().unwrap());
    let _guard = rt.enter();

    let mut app = App::new();
    for pair in data.chunks_exact(2).take(256) {
        let _ = handle_key(&mut app, key([pair[0], pair[1]]));
        if app.should_quit { break; }
    }
});
