//! What opens inside the window and what goes to the browser or another app.
//!
//! - Your Marea: inside.
//! - Google and Microsoft sign-in pages ("Link with Google"…): inside, or the redirect back would land in the
//!   browser instead of the app.
//! - Any other site: in your browser. Plain links are caught by the bridge (`bridge.js`) when clicked; links that
//!   open a new tab, by `on_new_window`; and if something navigates the whole window elsewhere, the app opens it
//!   outside and goes back.
//! - Mail (`mailto:`), phone (`tel:`)…: in the system app. Any other unusual scheme: blocked.

use url::Url;

const OAUTH_HOSTS: &[&str] =
    &["accounts.google.com", "login.microsoftonline.com", "login.microsoft.com", "login.live.com", "account.live.com"];

/// Schemes that may be handed to the system (browser, mail, phone…).
const EXTERNAL_SCHEMES: &[&str] = &["http", "https", "mailto", "tel", "sms", "webcal", "facetime", "facetime-audio"];

#[derive(Debug, PartialEq, Eq)]
pub enum Decision {
    /// Opens inside the app.
    Allow,
    /// Goes outside (browser, Mail, Phone…).
    External,
    /// Neither inside nor outside.
    Block,
}

/// The app's own pages (the connection screen).
pub fn is_local(url: &Url) -> bool {
    url.scheme() == "tauri" || url.host_str() == Some("tauri.localhost")
}

pub fn is_oauth(url: &Url) -> bool {
    url.scheme() == "https" && url.host_str().is_some_and(|h| OAUTH_HOSTS.contains(&h))
}

fn same_origin(url: &Url, server: Option<&str>) -> bool {
    server.is_some_and(|o| url.origin().ascii_serialization() == o)
}

/// On navigation (of the whole window or of a frame inside it: macOS doesn't tell them apart).
///
/// Web addresses are let through because they are also the videos and emails Marea shows inside frames; a whole
/// window going to another site is corrected by `on_page_load`.
pub fn on_navigation(url: &Url) -> Decision {
    match url.scheme() {
        "tauri" | "http" | "https" | "about" | "blob" | "data" => Decision::Allow,
        s if EXTERNAL_SCHEMES.contains(&s) => Decision::External,
        _ => Decision::Block,
    }
}

/// When a new window is requested (`target="_blank"`, `window.open`).
pub fn on_new_window(url: &Url, server: Option<&str>) -> Decision {
    if same_origin(url, server) || is_oauth(url) {
        return Decision::Allow;
    }
    if EXTERNAL_SCHEMES.contains(&url.scheme()) {
        return Decision::External;
    }
    Decision::Block
}

/// When the whole window has loaded a page that is not Marea, a sign-in page or one of the app's own pages, it
/// is opened outside and the window goes back.
pub fn main_frame_is_foreign(url: &Url, server: Option<&str>) -> bool {
    matches!(url.scheme(), "http" | "https") && !is_local(url) && !is_oauth(url) && !same_origin(url, server)
}

/// What a page may open outside: only known schemes and sensible addresses.
pub fn external_ok(text: &str) -> Option<Url> {
    if text.len() > 4096 {
        return None;
    }
    let url = Url::parse(text).ok()?;
    if !EXTERNAL_SCHEMES.contains(&url.scheme()) {
        return None;
    }
    if matches!(url.scheme(), "http" | "https") && url.host_str().is_none() {
        return None;
    }
    Some(url)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn u(s: &str) -> Url {
        Url::parse(s).unwrap()
    }

    const SERVER: Option<&str> = Some("https://marea.example.com");

    #[test]
    fn navigation() {
        assert_eq!(on_navigation(&u("https://www.youtube.com/embed/x")), Decision::Allow);
        assert_eq!(on_navigation(&u("tauri://localhost/index.html")), Decision::Allow);
        assert_eq!(on_navigation(&u("mailto:hello@example.com")), Decision::External);
        assert_eq!(on_navigation(&u("file:///etc/passwd")), Decision::Block);
        assert_eq!(on_navigation(&u("javascript:alert(1)")), Decision::Block);
    }

    #[test]
    fn new_windows() {
        assert_eq!(on_new_window(&u("https://marea.example.com/api.php?a=file&id=1"), SERVER), Decision::Allow);
        assert_eq!(on_new_window(&u("https://accounts.google.com/o/oauth2"), SERVER), Decision::Allow);
        assert_eq!(on_new_window(&u("https://elpais.com/"), SERVER), Decision::External);
        assert_eq!(on_new_window(&u("file:///tmp/x"), SERVER), Decision::Block);
    }

    #[test]
    fn foreign_pages() {
        assert!(main_frame_is_foreign(&u("https://elpais.com/"), SERVER));
        assert!(!main_frame_is_foreign(&u("https://marea.example.com/#/mail"), SERVER));
        assert!(!main_frame_is_foreign(&u("https://login.microsoftonline.com/x"), SERVER));
        assert!(!main_frame_is_foreign(&u("http://tauri.localhost/index.html"), SERVER));
        assert!(!main_frame_is_foreign(&u("tauri://localhost/index.html"), SERVER));
    }

    #[test]
    fn external_links() {
        assert!(external_ok("https://elpais.com/").is_some());
        assert!(external_ok("mailto:hello@example.com").is_some());
        assert!(external_ok("file:///etc/passwd").is_none());
        assert!(external_ok("javascript:alert(1)").is_none());
        assert!(external_ok("vscode://open").is_none());
        assert!(external_ok("not a url").is_none());
    }
}
