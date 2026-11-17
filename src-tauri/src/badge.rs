//! The pending count on the app icon (macOS Dock, Windows taskbar, Linux launcher).
//!
//! The same two numbers Marea shows next to «Waves» and «Correo» in its menu, added up: unread waves and unread
//! mail. The bridge counts them (see bridge.js, "The number on the app's icon") and calls `set_badge`; this only
//! puts the number on the icon. Until 0.2.0 it was read from the page title ("(3) Marea"), which only has the waves.

use tauri::{Runtime, WebviewWindow};

/// Bigger numbers don't fit on the icon and say nothing more.
pub const MAX: u32 = 9_999;

/// Sets (or clears, with 0) the number on the icon.
pub fn apply<R: Runtime>(window: &WebviewWindow<R>, count: u32) {
    let count = count.min(MAX);
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
    #[cfg(target_os = "windows")]
    #[test]
    fn every_overlay_loads() {
        for count in [1, 5, 9, 10, super::MAX] {
            assert!(super::overlay(count).is_some(), "overlay for {count}");
        }
        assert!(super::overlay(0).is_none());
    }
}
