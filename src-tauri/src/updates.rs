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

async fn latest() -> Result<Release, String> {
    let client = reqwest::Client::builder()
        .user_agent(concat!("MareaDesktop/", env!("CARGO_PKG_VERSION")))
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| e.to_string())?;
    let response = client
        .get(LATEST)
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .map_err(|_| t("update.err.offline"))?;
    if response.status().as_u16() == 404 {
        return Err(t("update.err.none"));
    }
    if !response.status().is_success() {
        return Err(tf("update.err.status", &[("status", &response.status().as_u16().to_string())]));
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
    use super::version;

    #[test]
    fn compares_versions() {
        assert!(version("v0.2.0") > version("0.1.9"));
        assert!(version("0.10.0") > version("0.9.0"));
        assert_eq!(version("v1.2.3-beta.1"), Some((1, 2, 3)));
        assert_eq!(version("1.2"), Some((1, 2, 0)));
        assert_eq!(version("hello"), None);
    }
}
