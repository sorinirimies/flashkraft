//! Debug Logging Utilities
//!
//! Centralized logging macros and utilities for FlashKraft.
//! All debug logging is conditionally compiled and only active in debug builds.

/// Internal implementation for debug-only tagged logging.
///
/// Not intended for direct use — call [`debug_log!`], [`flash_debug!`], or
/// [`status_log!`] instead.
#[doc(hidden)]
#[macro_export]
macro_rules! __debug_log_impl {
    ($tag:expr, $($arg:tt)*) => {
        #[cfg(debug_assertions)]
        eprintln!(concat!("[", $tag, "] {}"), format!($($arg)*));
    };
}

/// Debug logging macro for general application messages.
///
/// Only prints in debug builds. Automatically prefixes with `[DEBUG]`.
/// Delegates to [`__debug_log_impl!`].
///
/// # Example
/// ```no_run
/// # use flashkraft_core::debug_log;
/// let path = "/path/to/image.iso";
/// debug_log!("User selected image: {}", path);
/// ```
#[macro_export]
macro_rules! debug_log {
    ($($arg:tt)*) => { $crate::__debug_log_impl!("DEBUG", $($arg)*) };
}

/// Debug logging macro for flash subscription messages.
///
/// Only prints in debug builds. Automatically prefixes with `[FLASH_DEBUG]`.
/// Delegates to [`__debug_log_impl!`].
///
/// # Example
/// ```no_run
/// # use flashkraft_core::flash_debug;
/// let progress = 0.75;
/// flash_debug!("Progress: {:.1}%", progress * 100.0);
/// ```
#[macro_export]
macro_rules! flash_debug {
    ($($arg:tt)*) => { $crate::__debug_log_impl!("FLASH_DEBUG", $($arg)*) };
}

/// Debug logging macro for status messages.
///
/// Only prints in debug builds. Automatically prefixes with `[STATUS]`.
/// Delegates to [`__debug_log_impl!`].
///
/// # Example
/// ```no_run
/// # use flashkraft_core::status_log;
/// status_log!("Starting flash operation");
/// ```
#[macro_export]
macro_rules! status_log {
    ($($arg:tt)*) => { $crate::__debug_log_impl!("STATUS", $($arg)*) };
}

/// Conditional debug logging - only logs if condition is true
///
/// # Example
/// ```no_run
/// # use flashkraft_core::debug_if;
/// let verbose_mode = true;
/// let data = vec![1, 2, 3];
/// debug_if!(verbose_mode, "Detailed info: {:?}", data);
/// ```
#[macro_export]
macro_rules! debug_if {
    ($condition:expr, $($arg:tt)*) => {
        #[cfg(debug_assertions)]
        if $condition {
            eprintln!("[DEBUG] {}", format!($($arg)*));
        }
    };
}

/// Format a byte count as a compact human-readable string.
///
/// Uses binary prefixes (1 KiB = 1024 bytes) and one decimal place.
/// Output examples: `"1.5G"`, `"512.0M"`, `"3.2K"`, `"42B"`.
///
/// # Example
/// ```
/// use flashkraft_core::fmt_bytes;
/// assert_eq!(fmt_bytes(0),              "0B");
/// assert_eq!(fmt_bytes(1_023),          "1023B");
/// assert_eq!(fmt_bytes(1_024),          "1.0K");
/// assert_eq!(fmt_bytes(1_048_576),      "1.0M");
/// assert_eq!(fmt_bytes(1_073_741_824),  "1.0G");
/// ```
pub fn fmt_bytes(bytes: u64) -> String {
    const GIB: u64 = 1_073_741_824;
    const MIB: u64 = 1_048_576;
    const KIB: u64 = 1_024;

    if bytes >= GIB {
        format!("{:.1}G", bytes as f64 / GIB as f64)
    } else if bytes >= MIB {
        format!("{:.1}M", bytes as f64 / MIB as f64)
    } else if bytes >= KIB {
        format!("{:.1}K", bytes as f64 / KIB as f64)
    } else {
        format!("{bytes}B")
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_debug_macros_compile() {
        // These should compile without errors
        debug_log!("Test message");
        flash_debug!("Flash test: {}", 42);
        status_log!("Status test");
        debug_if!(true, "Conditional test");
    }

    // ── fmt_bytes ─────────────────────────────────────────────────────────────

    #[test]
    fn fmt_bytes_zero() {
        assert_eq!(super::fmt_bytes(0), "0B");
    }

    #[test]
    fn fmt_bytes_bytes_range() {
        assert_eq!(super::fmt_bytes(1), "1B");
        assert_eq!(super::fmt_bytes(1_023), "1023B");
    }

    #[test]
    fn fmt_bytes_kib_boundary() {
        assert_eq!(super::fmt_bytes(1_024), "1.0K");
        assert_eq!(super::fmt_bytes(1_536), "1.5K");
        assert_eq!(super::fmt_bytes(1_047_552), "1023.0K");
    }

    #[test]
    fn fmt_bytes_mib_boundary() {
        assert_eq!(super::fmt_bytes(1_048_576), "1.0M");
        assert_eq!(super::fmt_bytes(524_288_000), "500.0M");
    }

    #[test]
    fn fmt_bytes_gib_boundary() {
        assert_eq!(super::fmt_bytes(1_073_741_824), "1.0G");
        assert_eq!(super::fmt_bytes(32_212_254_720), "30.0G");
    }
}
