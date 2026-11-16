//! The pending count on the app icon (macOS Dock, Windows taskbar, Linux launcher).
//!
//! Marea already puts that number in the page title ("(3) Marea"): the app reads it from there, without changing
//! Marea.

use tauri::{Runtime, WebviewWindow};

/// "(3) Marea" → 3. Any other title → 0.
pub fn count_from_title(title: &str) -> u32 {
    let Some(rest) = title.trim_start().strip_prefix('(') else {
        return 0;
    };
    let Some(end) = rest.find(')') else {
        return 0;
    };
    rest[..end].trim().trim_end_matches('+').parse().unwrap_or(0)
}

/// Sets (or clears, with 0) the number on the icon.
pub fn apply<R: Runtime>(window: &WebviewWindow<R>, count: u32) {
    #[cfg(not(target_os = "windows"))]
    {
        let _ = window.set_badge_count(if count > 0 { Some(i64::from(count)) } else { None });
    }
    #[cfg(target_os = "windows")]
    {
        let _ = window.set_overlay_icon(overlay(count));
    }
}

/// Windows has no badge count: a small red circle is laid over the icon instead, numbered up to 9 and then "9+".
#[cfg(target_os = "windows")]
fn overlay(count: u32) -> Option<tauri::image::Image<'static>> {
    let bytes: &'static [u8] = match count {
        0 => return None,
        1 => include_bytes!("../icons/badges/badge-1.png"),
        2 => include_bytes!("../icons/badges/badge-2.png"),
        3 => include_bytes!("../icons/badges/badge-3.png"),
        4 => include_bytes!("../icons/badges/badge-4.png"),
        5 => include_bytes!("../icons/badges/badge-5.png"),
        6 => include_bytes!("../icons/badges/badge-6.png"),
        7 => include_bytes!("../icons/badges/badge-7.png"),
        8 => include_bytes!("../icons/badges/badge-8.png"),
        9 => include_bytes!("../icons/badges/badge-9.png"),
        _ => include_bytes!("../icons/badges/badge-9plus.png"),
    };
    tauri::image::Image::from_bytes(bytes).ok()
}

#[cfg(test)]
mod tests {
    use super::count_from_title;

    #[test]
    fn reads_the_count() {
        assert_eq!(count_from_title("(3) Marea"), 3);
        assert_eq!(count_from_title("(120) Paperclip"), 120);
        assert_eq!(count_from_title("Marea"), 0);
        assert_eq!(count_from_title("(unread) Marea"), 0);
        assert_eq!(count_from_title("File transfer · Marea"), 0);
        assert_eq!(count_from_title(""), 0);
    }
}
