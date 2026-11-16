//! Files downloaded from Marea (attachments, transfers, backups…) go to the Downloads folder, with a notification
//! when they are done.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use tauri::{AppHandle, Manager, Runtime};
use url::Url;

use crate::i18n::{t, tf};
use crate::notify_system;

/// Names of the downloads in progress, to tell the person which one finished.
#[derive(Default)]
pub struct Downloads(Mutex<HashMap<String, String>>);

/// A download starts: it is saved in Downloads without overwriting anything (the engines already suggest that
/// folder and the file name; this only makes sure of it).
pub fn requested<R: Runtime>(app: &AppHandle<R>, url: &Url, destination: &mut PathBuf) -> bool {
    let dir =
        app.path().download_dir().ok().or_else(|| destination.parent().map(Path::to_path_buf)).unwrap_or_default();
    let name = destination
        .file_name()
        .and_then(|n| n.to_str())
        .filter(|n| !n.trim().is_empty())
        .map(clean_name)
        .unwrap_or_else(|| {
            url.path_segments()
                .and_then(|mut s| s.next_back())
                .filter(|s| !s.is_empty())
                .map(clean_name)
                .unwrap_or_else(|| String::from("download"))
        });
    let path = unique(&dir, &name);
    let shown = path.file_name().and_then(|n| n.to_str()).unwrap_or(&name).to_string();
    if let Some(state) = app.try_state::<Downloads>() {
        if let Ok(mut map) = state.0.lock() {
            map.insert(url.to_string(), shown);
        }
    }
    *destination = path;
    true
}

/// It finished (well or not): a notification with the file name.
pub fn finished<R: Runtime>(app: &AppHandle<R>, url: &Url, path: Option<&Path>, success: bool) {
    let name = app
        .try_state::<Downloads>()
        .and_then(|s| s.0.lock().ok().and_then(|mut m| m.remove(url.as_str())))
        .or_else(|| path.and_then(|p| p.file_name()).and_then(|n| n.to_str()).map(String::from))
        .unwrap_or_else(|| t("download.default_name"));
    if success {
        notify_system(app, &t("download.done.title"), &tf("download.done.body", &[("name", &name)]));
    } else {
        notify_system(app, &t("download.failed.title"), &tf("download.failed.body", &[("name", &name)]));
    }
}

/// A file name without slashes or characters that break on Windows.
fn clean_name(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            c if c.is_control() => '_',
            c => c,
        })
        .collect();
    let trimmed = cleaned.trim().trim_start_matches('.').trim();
    if trimmed.is_empty() {
        String::from("download")
    } else {
        trimmed.chars().take(180).collect()
    }
}

/// `report.pdf`, `report (1).pdf`, `report (2).pdf`…
fn unique(dir: &Path, name: &str) -> PathBuf {
    let first = dir.join(name);
    if !first.exists() {
        return first;
    }
    let (stem, ext) = match name.rfind('.') {
        Some(i) if i > 0 => (&name[..i], &name[i..]),
        _ => (name, ""),
    };
    for n in 1..1000 {
        let candidate = dir.join(format!("{stem} ({n}){ext}"));
        if !candidate.exists() {
            return candidate;
        }
    }
    first
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cleans_names() {
        assert_eq!(clean_name("report.pdf"), "report.pdf");
        assert_eq!(clean_name("../../etc/passwd"), "_.._etc_passwd");
        assert_eq!(clean_name("a:b*c?.txt"), "a_b_c_.txt");
        assert_eq!(clean_name("   "), "download");
        assert_eq!(clean_name(".hidden"), "hidden");
    }

    #[test]
    fn never_overwrites() {
        let dir = std::env::temp_dir().join(format!("marea-desktop-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("report.pdf"), b"x").unwrap();
        std::fs::write(dir.join("report (1).pdf"), b"x").unwrap();
        assert_eq!(unique(&dir, "report.pdf"), dir.join("report (2).pdf"));
        assert_eq!(unique(&dir, "new.pdf"), dir.join("new.pdf"));
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
