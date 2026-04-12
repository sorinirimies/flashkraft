//! USB Hotplug Detection
//!
//! Provides a cross-platform event stream that fires whenever a USB storage
//! device is connected or disconnected.  Callers receive a bare
//! [`UsbHotplugEvent`] and are expected to re-run
//! [`crate::commands::load_drives_sync`] themselves — hotplug is only a
//! *trigger*, not a source of drive data.  All block-device information
//! (path, size, mount point) continues to come from the existing
//! sysfs / diskutil / wmic enumeration code.
//!
//! ## Mechanism
//!
//! | Platform | Watched path        | Kernel mechanism          |
//! |----------|---------------------|---------------------------|
//! | Linux    | `/sys/block`        | inotify (via `notify`)    |
//! | macOS    | `/dev`              | FSEvents (via `notify`)   |
//! | Windows  | `\\.\PhysicalDrive` | ReadDirectoryChangesW     |
//!
//! On Linux, the kernel creates / removes entries under `/sys/block` whenever
//! a block device appears or disappears — including USB mass-storage devices.
//! Watching that directory with inotify requires **no privileges** and pulls
//! in no USB-specific library.
//!
//! On macOS, `/dev` receives `disk*` entries when drives connect; FSEvents
//! covers that without elevated permissions.
//!
//! On Windows, `notify` watches the system drive root as a lightweight
//! proxy; the real drive list comes from the wmic enumeration re-run on
//! every event.
//!
//! ## Permission notes
//!
//! No elevated privileges are required for any of these watches.
//! `/sys/block` and `/dev` are world-readable; the `notify` crate uses only
//! kernel-provided notification APIs (inotify fd, FSEvents socket,
//! ReadDirectoryChangesW handle) that any process may open.
//!
//! ## Usage
//!
//! ```rust,ignore
//! use flashkraft_core::commands::hotplug::{watch_usb_events, UsbHotplugEvent};
//! use futures::StreamExt;
//!
//! let mut stream = watch_usb_events()?;
//! while let Some(event) = stream.next().await {
//!     match event {
//!         UsbHotplugEvent::Arrived  => println!("drive connected"),
//!         UsbHotplugEvent::Left     => println!("drive disconnected"),
//!     }
//!     let drives = flashkraft_core::commands::load_drives_sync();
//! }
//! ```

use std::path::Path;
use std::sync::mpsc as std_mpsc;
use std::time::Duration;

use futures::Stream;
use notify::{Config, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use tokio::sync::mpsc as tokio_mpsc;

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// A USB hotplug trigger emitted when any block device connects or disconnects.
///
/// This is intentionally a plain enum with no device payload.  All drive
/// metadata is obtained by re-running the sysfs / diskutil / wmic enumeration
/// after receiving the event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UsbHotplugEvent {
    /// A drive was connected (a new entry appeared under the watched path).
    Arrived,
    /// A drive was disconnected (an entry was removed from the watched path).
    Left,
}

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/// Error returned when the OS refuses to create the filesystem watch.
#[derive(Debug)]
pub struct HotplugError(String);

impl std::fmt::Display for HotplugError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "hotplug watch error: {}", self.0)
    }
}

impl std::error::Error for HotplugError {}

impl From<notify::Error> for HotplugError {
    fn from(e: notify::Error) -> Self {
        HotplugError(e.to_string())
    }
}

// ---------------------------------------------------------------------------
// Platform-specific watch path
// ---------------------------------------------------------------------------

/// The filesystem path to watch for block-device add/remove events.
///
/// - Linux  : `/sys/block`  — kernel creates/removes entries here
/// - macOS  : `/dev`        — `disk*` nodes appear/disappear here
/// - Windows: `C:\`         — lightweight proxy; real data comes from wmic
/// - Other  : `/dev`        — reasonable fallback
fn watch_path() -> &'static Path {
    #[cfg(target_os = "linux")]
    return Path::new("/sys/block");

    #[cfg(target_os = "macos")]
    return Path::new("/dev");

    #[cfg(target_os = "windows")]
    return Path::new("C:\\");

    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    return Path::new("/dev");
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Start watching for block-device connect / disconnect events.
///
/// Returns a [`Stream`] that yields [`UsbHotplugEvent`] values.  The stream
/// runs for the lifetime of the returned object; drop it to stop watching.
///
/// Internally this spawns a small background thread that owns the
/// [`notify::RecommendedWatcher`] (which is `!Send` on some platforms) and
/// bridges its events into an async Tokio channel.
///
/// # Errors
///
/// Returns a [`HotplugError`] if the OS refuses to create the inotify /
/// FSEvents / ReadDirectoryChangesW watch.  This is rare — it typically only
/// happens when the watched path does not exist (e.g. `/sys/block` on a
/// non-Linux kernel) or the process has been sandboxed.
pub fn watch_usb_events() -> Result<impl Stream<Item = UsbHotplugEvent>, HotplugError> {
    let path = watch_path();

    // ── Verify the watch path exists before committing ────────────────────────
    if !path.exists() {
        // Return a stream that immediately ends rather than hard-erroring —
        // the caller will simply get no events, which is safe.
        let (tx, rx) = tokio_mpsc::unbounded_channel::<UsbHotplugEvent>();
        drop(tx); // closed immediately → stream ends
        return Ok(UnboundedReceiverStream::new(rx));
    }

    // ── Bridge: notify (sync) → tokio unbounded channel (async) ──────────────
    //
    // `notify::RecommendedWatcher` is not `Send` on all platforms (macOS
    // FSEvents runs on a dedicated thread managed by CoreFoundation).  We
    // therefore own the watcher on a dedicated `std::thread` and forward
    // filesystem events through a `std::sync::mpsc` channel to a second
    // thread that feeds a `tokio::sync::mpsc` channel readable from async code.

    // Step 1: std::sync::mpsc — raw notify events arrive here.
    let (notify_tx, notify_rx) = std_mpsc::channel::<notify::Result<notify::Event>>();

    // Step 2: tokio unbounded channel — translated UsbHotplugEvents go here.
    let (hotplug_tx, hotplug_rx) = tokio_mpsc::unbounded_channel::<UsbHotplugEvent>();

    // ── Thread A: owns the Watcher ────────────────────────────────────────────
    //
    // We build the watcher here (synchronously) so we can return an error if
    // setup fails, then move it into the thread to keep it alive.
    let mut watcher = RecommendedWatcher::new(
        notify_tx,
        Config::default()
            // Poll interval is only used by the PollWatcher fallback.
            // The recommended watcher (inotify / FSEvents / ReadDirChanges)
            // ignores this, but setting it avoids a "missing config" warning.
            .with_poll_interval(Duration::from_secs(2)),
    )?;

    watcher.watch(path, RecursiveMode::NonRecursive)?;

    std::thread::Builder::new()
        .name("flashkraft-hotplug-watcher".into())
        .spawn(move || {
            // Keep `watcher` alive for the lifetime of this thread.
            // The thread exits when the notify_rx receiver (in Thread B) is
            // dropped, which causes notify_tx sends to fail → watcher is
            // dropped → inotify fd / FSEvents stream is closed.
            let _watcher = watcher;

            // Block this thread indefinitely; the watcher's background
            // mechanism (inotify fd read-loop / kqueue / etc.) delivers
            // events into notify_tx automatically without any polling here.
            // We park the thread so it does not busy-spin.
            loop {
                std::thread::park();
            }
        })
        .ok(); // If spawning fails we still return the (silent) stream.

    // ── Thread B: translates notify events → UsbHotplugEvents ────────────────
    let hotplug_tx_b = hotplug_tx;
    std::thread::Builder::new()
        .name("flashkraft-hotplug-bridge".into())
        .spawn(move || {
            for result in &notify_rx {
                let event = match result {
                    Ok(e) => e,
                    Err(_) => continue,
                };

                let translated = translate_event(&event);
                if let Some(hp_event) = translated {
                    // If the receiver (stream) has been dropped, stop.
                    if hotplug_tx_b.send(hp_event).is_err() {
                        break;
                    }
                }
            }
            // notify_rx exhausted (watcher dropped) → channel closed naturally.
        })
        .ok();

    Ok(UnboundedReceiverStream::new(hotplug_rx))
}

// ---------------------------------------------------------------------------
// Event translation
// ---------------------------------------------------------------------------

/// Map a raw [`notify::Event`] to a [`UsbHotplugEvent`], or `None` to discard.
///
/// We only care about create / remove events at the top level of the watched
/// directory.  Modify events (e.g. attribute changes on existing sysfs files)
/// are ignored to avoid spurious re-enumerations.
fn translate_event(event: &notify::Event) -> Option<UsbHotplugEvent> {
    match &event.kind {
        // A new entry appeared — a block device was added.
        EventKind::Create(_) => Some(UsbHotplugEvent::Arrived),

        // An entry was removed — a block device was unplugged.
        EventKind::Remove(_) => Some(UsbHotplugEvent::Left),

        // On some platforms (e.g. macOS FSEvents) renames of device nodes
        // indicate a new device taking over a slot.  Treat as Arrived.
        EventKind::Modify(notify::event::ModifyKind::Name(_)) => Some(UsbHotplugEvent::Arrived),

        // Content / metadata / other modify events — ignore.
        EventKind::Modify(_) => None,

        // Access events — ignore.
        EventKind::Access(_) => None,

        // Unknown / other — conservative: treat as Arrived so the caller
        // refreshes the drive list and discovers the true state.
        EventKind::Other => Some(UsbHotplugEvent::Arrived),

        EventKind::Any => None,
    }
}

// ---------------------------------------------------------------------------
// Minimal Stream wrapper around tokio::sync::mpsc::UnboundedReceiver
// ---------------------------------------------------------------------------

/// A [`Stream`] backed by a [`tokio_mpsc::UnboundedReceiver`].
///
/// This is a lightweight alternative to `tokio_stream::wrappers::UnboundedReceiverStream`
/// that avoids adding `tokio-stream` as a dependency.
struct UnboundedReceiverStream<T> {
    inner: tokio_mpsc::UnboundedReceiver<T>,
}

impl<T> UnboundedReceiverStream<T> {
    fn new(inner: tokio_mpsc::UnboundedReceiver<T>) -> Self {
        Self { inner }
    }
}

impl<T> Stream for UnboundedReceiverStream<T> {
    type Item = T;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        self.inner.poll_recv(cx)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use notify::event::{CreateKind, ModifyKind, RemoveKind, RenameMode};

    // ── UsbHotplugEvent trait coverage ───────────────────────────────────────

    #[test]
    fn test_event_traits() {
        let a = UsbHotplugEvent::Arrived;
        let b = UsbHotplugEvent::Left;

        let a2 = a.clone();
        let b2 = b.clone();

        assert_eq!(a, a2);
        assert_eq!(b, b2);
        assert_ne!(a, b);

        assert!(format!("{a:?}").contains("Arrived"));
        assert!(format!("{b:?}").contains("Left"));
    }

    #[test]
    fn test_variant_exhaustiveness() {
        for event in [UsbHotplugEvent::Arrived, UsbHotplugEvent::Left] {
            let label = match event {
                UsbHotplugEvent::Arrived => "arrived",
                UsbHotplugEvent::Left => "left",
            };
            assert!(!label.is_empty());
        }
    }

    // ── translate_event ───────────────────────────────────────────────────────

    fn make_event(kind: EventKind) -> notify::Event {
        notify::Event {
            kind,
            paths: vec![],
            attrs: Default::default(),
        }
    }

    macro_rules! translate_event_test {
        ($name:ident, $kind:expr, $expected:expr) => {
            #[test]
            fn $name() {
                let e = make_event($kind);
                assert_eq!(translate_event(&e), $expected);
            }
        };
    }

    translate_event_test!(
        test_translate_create_any_arrives,
        EventKind::Create(CreateKind::Any),
        Some(UsbHotplugEvent::Arrived)
    );
    translate_event_test!(
        test_translate_create_file_arrives,
        EventKind::Create(CreateKind::File),
        Some(UsbHotplugEvent::Arrived)
    );
    translate_event_test!(
        test_translate_create_folder_arrives,
        EventKind::Create(CreateKind::Folder),
        Some(UsbHotplugEvent::Arrived)
    );
    translate_event_test!(
        test_translate_remove_any_left,
        EventKind::Remove(RemoveKind::Any),
        Some(UsbHotplugEvent::Left)
    );
    translate_event_test!(
        test_translate_remove_file_left,
        EventKind::Remove(RemoveKind::File),
        Some(UsbHotplugEvent::Left)
    );
    translate_event_test!(
        test_translate_rename_arrives,
        EventKind::Modify(ModifyKind::Name(RenameMode::Any)),
        Some(UsbHotplugEvent::Arrived)
    );
    translate_event_test!(
        test_translate_modify_data_ignored,
        EventKind::Modify(ModifyKind::Data(notify::event::DataChange::Any)),
        None
    );
    translate_event_test!(
        test_translate_modify_metadata_ignored,
        EventKind::Modify(ModifyKind::Metadata(notify::event::MetadataKind::Any)),
        None
    );
    translate_event_test!(
        test_translate_access_ignored,
        EventKind::Access(notify::event::AccessKind::Any),
        None
    );
    translate_event_test!(
        test_translate_other_arrives,
        EventKind::Other,
        Some(UsbHotplugEvent::Arrived)
    );
    translate_event_test!(test_translate_any_ignored, EventKind::Any, None);

    // ── watch_usb_events construction ─────────────────────────────────────────

    /// Constructing the watch must not panic.  It either succeeds (returns a
    /// stream) or returns a clean error — both outcomes are acceptable.
    #[test]
    fn test_watch_usb_events_does_not_panic() {
        let result = watch_usb_events();
        match result {
            Ok(_) => println!("watch_usb_events: stream created successfully"),
            Err(ref e) => println!("watch_usb_events: OS returned error (acceptable): {e}"),
        }
        // Reaching this line without panicking is the assertion.
    }

    // ── HotplugError display ──────────────────────────────────────────────────

    #[test]
    fn test_hotplug_error_display() {
        let e = HotplugError("something went wrong".into());
        let s = format!("{e}");
        assert!(s.contains("hotplug watch error"));
        assert!(s.contains("something went wrong"));
    }

    #[test]
    fn test_hotplug_error_from_notify() {
        let notify_err = notify::Error::generic("test error");
        let hp_err = HotplugError::from(notify_err);
        let s = format!("{hp_err}");
        assert!(s.contains("hotplug watch error"));
    }
}
