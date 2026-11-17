//! Is there a newer version of the app? Checked against the releases of the GitHub repository.
//!
//! In this first version, when there is one the app says so and opens the download page. Automatic installation
//! will come once the repository has its own key to sign updates (see NOTES.md).

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::Deserialize;
use tauri::{AppHandle, Runtime};
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};
use tauri_plugin_opener::OpenerExt;

use crate::i18n::{t, tf};
use crate::server;

const LATEST: &str = "https://api.github.com/repos/mareabypaperclip/marea-desktop/releases/latest";
const RELEASES: &str = "https://github.com/mareabypaperclip/marea-desktop/releases/latest";
/// How often to look again while the app stays open.
const EVERY: Duration = Duration::from_secs(12 * 3600);
/// A version already announced is not announced again for this many days.
const QUIET_DAYS: u64 = 3;

#[derive(Deserialize)]
struct Release {
    tag_name: String,
    html_url: Option<String>,
    draft: Option<bool>,
    prerelease: Option<bool>,
}

/// Automatic check: shortly after launch and then every 12 hours. Silent when there is nothing new.
pub fn schedule<R: Runtime>(app: AppHandle<R>) {
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(Duration::from_secs(20)).await;
        loop {
            check(&app, false).await;
            tokio::time::sleep(EVERY).await;
        }
    });
}

/// Check asked for from the menu: always answers something.
pub fn check_now<R: Runtime>(app: AppHandle<R>) {
    tauri::async_runtime::spawn(async move {
        check(&app, true).await;
    });
}

async fn check<R: Runtime>(app: &AppHandle<R>, asked: bool) {
    let current = env!("CARGO_PKG_VERSION");
    match latest().await {
        Ok(release) => {
            let newer = version(&release.tag_name).is_some_and(|v| Some(v) > version(current));
            if !newer {
                if asked {
                    info(app, &t("update.uptodate.title"), &tf("update.uptodate.body", &[("current", current)]));
                }
                return;
            }
            let tag = release.tag_name.trim_start_matches('v').to_string();
            if !asked && recently_announced(app, &tag) {
                return;
            }
            server::set_flag(app, "announced_version", serde_json::json!({ "version": tag, "at": now() }));
            let page = release.html_url.unwrap_or_else(|| RELEASES.to_string());
            let opener = app.clone();
            app.dialog()
                .message(tf("update.body", &[("new", &tag), ("current", current)]))
                .title(t("update.title"))
                .kind(MessageDialogKind::Info)
                .buttons(MessageDialogButtons::OkCancelCustom(t("update.download"), t("update.later")))
                .show(move |download| {
                    if download {
                        let _ = opener.opener().open_url(page, None::<&str>);
                    }
                });
        }
        Err(e) => {
            if asked {
                info(app, &t("update.error.title"), &tf("update.error.body", &[("error", &e)]));
            }
        }
    }
}

/// The newest published version. First from the address GitHub's website itself redirects to
/// (`…/releases/latest` → `…/releases/tag/v1.2.3`), which has no request limit; then, if that doesn't answer as
/// expected, from GitHub's API, which only allows 60 requests an hour from each internet connection without an
/// account (0.2.1 asked only the API, and a shared or busy connection got "403").
async fn latest() -> Result<Release, String> {
    match from_redirect().await {
        Some(release) => Ok(release),
        None => from_api().await,
    }
}

fn client(redirects: bool) -> Result<reqwest::Client, String> {
    let policy = if redirects { reqwest::redirect::Policy::default() } else { reqwest::redirect::Policy::none() };
    reqwest::Client::builder()
        .user_agent(concat!("MareaDesktop/", env!("CARGO_PKG_VERSION")))
        .timeout(Duration::from_secs(10))
        .redirect(policy)
        .build()
        .map_err(|e| e.to_string())
}

async fn from_redirect() -> Option<Release> {
    let response = client(false).ok()?.head(RELEASES).send().await.ok()?;
    if !response.status().is_redirection() {
        return None;
    }
    let location = response.headers().get(reqwest::header::LOCATION)?.to_str().ok()?;
    let tag = tag_from_location(location)?;
    Some(Release { tag_name: tag, html_url: Some(location.to_string()), draft: None, prerelease: None })
}

/// "https://github.com/…/releases/tag/v1.2.3" → "v1.2.3". With no release yet GitHub redirects to the list of
/// releases instead: `None`.
fn tag_from_location(location: &str) -> Option<String> {
    let (repository, tag) = location.split_once("/releases/tag/")?;
    let tag = tag.split(['?', '#']).next()?.trim_end_matches('/');
    let ours = repository.ends_with("github.com/mareabypaperclip/marea-desktop");
    (ours && version(tag).is_some()).then(|| tag.to_string())
}

async fn from_api() -> Result<Release, String> {
    let response = client(true)?
        .get(LATEST)
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .map_err(|_| t("update.err.offline"))?;
    let status = response.status().as_u16();
    if status == 404 {
        return Err(t("update.err.none"));
    }
    if status == 403 || status == 429 {
        return Err(t("update.err.busy"));
    }
    if !response.status().is_success() {
        return Err(tf("update.err.status", &[("status", &status.to_string())]));
    }
    let release: Release = response.json().await.map_err(|_| t("update.err.unexpected"))?;
    if release.draft.unwrap_or(false) || release.prerelease.unwrap_or(false) {
        return Err(t("update.err.none"));
    }
    Ok(release)
}

fn recently_announced<R: Runtime>(app: &AppHandle<R>, tag: &str) -> bool {
    let Some(flag) = server::get_flag(app, "announced_version") else {
        return false;
    };
    let same = flag.get("version").and_then(|v| v.as_str()) == Some(tag);
    let when = flag.get("at").and_then(|v| v.as_u64()).unwrap_or(0);
    same && now().saturating_sub(when) < QUIET_DAYS * 86_400
}

fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

fn info<R: Runtime>(app: &AppHandle<R>, title: &str, text: &str) {
    app.dialog()
        .message(text.to_string())
        .title(title.to_string())
        .kind(MessageDialogKind::Info)
        .buttons(MessageDialogButtons::OkCustom(t("update.ok")))
        .show(|_| {});
}

/// "v1.2.3" or "1.2.3-beta" → (1, 2, 3). Anything else → `None`.
fn version(text: &str) -> Option<(u64, u64, u64)> {
    let core = text.trim().trim_start_matches('v');
    let core = core.split(['-', '+']).next()?;
    let mut parts = core.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next().unwrap_or("0").parse().ok()?;
    let patch = parts.next().unwrap_or("0").parse().ok()?;
    Some((major, minor, patch))
}

#[cfg(test)]
mod tests {
    use super::{tag_from_location, version};

    #[test]
    fn reads_the_tag_from_the_redirect() {
        let at = |path: &str| format!("https://github.com/mareabypaperclip/marea-desktop/releases{path}");
        assert_eq!(tag_from_location(&at("/tag/v0.2.2")).as_deref(), Some("v0.2.2"));
        assert_eq!(tag_from_location(&at("/tag/v1.0.0/")).as_deref(), Some("v1.0.0"));
        assert_eq!(tag_from_location(&at("")), None, "no release yet: the list of releases");
        assert_eq!(tag_from_location(&at("/tag/nightly")), None);
        assert_eq!(tag_from_location("https://github.com/someone/else/releases/tag/v9.9.9"), None);
    }

    #[test]
    fn compares_versions() {
        assert!(version("v0.2.0") > version("0.1.9"));
        assert!(version("0.10.0") > version("0.9.0"));
        assert_eq!(version("v1.2.3-beta.1"), Some((1, 2, 3)));
        assert_eq!(version("1.2"), Some((1, 2, 0)));
        assert_eq!(version("hello"), None);
    }
}
