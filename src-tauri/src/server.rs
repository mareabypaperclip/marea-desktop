//! The Marea server the app connects to: reading what the person types, checking it really is a Marea and
//! remembering it between launches.

use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Runtime};
use tauri_plugin_store::StoreExt;
use url::Url;

use crate::i18n::{t, tf};

const STORE: &str = "settings.json";
const SERVER_KEY: &str = "server";

/// What is known about a Marea after checking it.
#[derive(Clone, Debug, Serialize)]
pub struct Info {
    /// Entry address, always ending in `/` (for example `https://marea.example.com/`).
    pub base: String,
    /// The name its administrator gave it ("Marea" if none).
    pub name: String,
    /// The Marea version it runs.
    pub version: String,
    /// Just the host, to show on screen (`marea.example.com`).
    pub host: String,
}

/// Turns what the person typed into a valid address.
///
/// Accepts `marea.example.com`, `https://example.com/marea/`, `https://…/index.php`… Requires https except for
/// the computer itself (`localhost`), which is handy for testing.
pub fn normalize(input: &str) -> Result<Url, String> {
    let text = input.trim();
    if text.is_empty() {
        return Err(t("error.empty"));
    }
    if text.len() > 300 {
        return Err(t("error.too_long"));
    }
    let with_scheme = if text.contains("://") { text.to_string() } else { format!("https://{text}") };
    let mut url = Url::parse(&with_scheme).map_err(|_| t("error.invalid"))?;
    let host = url.host_str().ok_or_else(|| t("error.invalid"))?.to_ascii_lowercase();
    let local = matches!(host.as_str(), "localhost" | "127.0.0.1" | "[::1]");
    match url.scheme() {
        "https" => {}
        "http" if local => {}
        "http" => return Err(t("error.https")),
        _ => return Err(t("error.invalid")),
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err(t("error.credentials"));
    }
    url.set_query(None);
    url.set_fragment(None);
    let mut path = url.path().to_string();
    for entry in ["index.php", "api.php"] {
        if let Some(rest) = path.strip_suffix(entry) {
            path = rest.to_string();
        }
    }
    if !path.ends_with('/') {
        path.push('/');
    }
    url.set_path(&path);
    Ok(url)
}

/// The origin of an address (`https://marea.example.com`): what decides "inside Marea" versus "another site".
pub fn origin(url: &Url) -> String {
    url.origin().ascii_serialization()
}

fn client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .user_agent(concat!("MareaDesktop/", env!("CARGO_PKG_VERSION")))
        .timeout(Duration::from_secs(10))
        .connect_timeout(Duration::from_secs(6))
        .redirect(reqwest::redirect::Policy::limited(3))
        .build()
        .map_err(|e| tf("error.prepare", &[("error", &e.to_string())]))
}

/// Checks there is a Marea at that address: its `api.php?a=boot` answers with its version and its name.
pub async fn probe(base: &Url) -> Result<Info, String> {
    let endpoint = base.join("api.php?a=boot").map_err(|_| t("error.invalid"))?;
    let response = client()?.get(endpoint).header("Accept", "application/json").send().await.map_err(|e| {
        if e.is_timeout() {
            t("error.timeout")
        } else if e.is_connect() {
            t("error.unreachable")
        } else {
            t("error.connect")
        }
    })?;
    // Redirected to another server (or from https to http): not the Marea that was typed
    if origin(response.url()) != origin(base) {
        return Err(t("error.redirect"));
    }
    let status = response.status();
    let body: serde_json::Value = response.json().await.map_err(|_| t("error.not_marea"))?;
    let version = body.get("version").and_then(|v| v.as_str()).unwrap_or_default();
    let instance = body.get("instance").and_then(|v| v.as_object());
    if version.is_empty() || instance.is_none() {
        if status.as_u16() == 503 {
            return Err(t("error.not_installed"));
        }
        return Err(t("error.not_marea"));
    }
    let name = instance
        .and_then(|i| i.get("name"))
        .and_then(|n| n.as_str())
        .map(str::trim)
        .filter(|n| !n.is_empty())
        .unwrap_or("Marea");
    Ok(Info {
        base: base.to_string(),
        name: name.chars().take(80).collect(),
        version: version.chars().take(40).collect(),
        host: base.host_str().unwrap_or_default().to_string(),
    })
}

/// The saved Marea, if any.
pub fn saved<R: Runtime>(app: &AppHandle<R>) -> Option<Url> {
    let store = app.store(STORE).ok()?;
    let value = store.get(SERVER_KEY)?;
    normalize(value.as_str()?).ok()
}

/// Saves the chosen Marea for the next launches.
pub fn save<R: Runtime>(app: &AppHandle<R>, base: &Url) -> Result<(), String> {
    let fail = |e: tauri_plugin_store::Error| tf("error.save", &[("error", &e.to_string())]);
    let store = app.store(STORE).map_err(fail)?;
    store.set(SERVER_KEY, serde_json::Value::String(base.to_string()));
    store.save().map_err(fail)
}

/// A loose app setting (notices already shown, etc.).
pub fn get_flag<R: Runtime>(app: &AppHandle<R>, key: &str) -> Option<serde_json::Value> {
    app.store(STORE).ok()?.get(key)
}

pub fn set_flag<R: Runtime>(app: &AppHandle<R>, key: &str, value: serde_json::Value) {
    if let Ok(store) = app.store(STORE) {
        store.set(key, value);
        let _ = store.save();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adds_https_and_trailing_slash() {
        assert_eq!(normalize("marea.example.com").unwrap().as_str(), "https://marea.example.com/");
        assert_eq!(normalize("  https://Marea.Example.com  ").unwrap().as_str(), "https://marea.example.com/");
        assert_eq!(normalize("https://example.com/marea").unwrap().as_str(), "https://example.com/marea/");
    }

    #[test]
    fn strips_entry_files_query_and_fragment() {
        assert_eq!(
            normalize("https://example.com/marea/index.php?x=1#/mail").unwrap().as_str(),
            "https://example.com/marea/"
        );
        assert_eq!(normalize("example.com/api.php").unwrap().as_str(), "https://example.com/");
    }

    #[test]
    fn requires_https_except_localhost() {
        assert!(normalize("http://marea.example.com").is_err());
        assert!(normalize("http://localhost:8090/dev/").is_ok());
        assert!(normalize("ftp://marea.example.com").is_err());
        assert!(normalize("").is_err());
        assert!(normalize("https://user:secret@example.com").is_err());
    }

    #[test]
    fn origin_ignores_path() {
        let url = normalize("https://example.com/marea/").unwrap();
        assert_eq!(origin(&url), "https://example.com");
    }
}
