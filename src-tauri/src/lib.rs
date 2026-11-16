//! Marea for desktop: your Marea in its own window, on macOS, Windows and Linux.
//!
//! The app does not ship Marea inside: it opens the Marea you choose (your company's) and adds what a browser
//! can't: its own icon in the Dock or taskbar, system notifications, the pending count on the icon, downloads in
//! the Downloads folder and outside links in your browser.
//!
//! Security: your Marea can only ask the app for a few harmless things ("show this notification", "open this
//! link outside" and, on macOS, "where am I?" behind the system's own permission prompt, plus moving the window
//! by its top bar), and only the Marea you chose (permissions are granted to its exact origin when connecting).
//! The connection screen is one of the app's own pages, with access to nothing else.

mod actions;
mod badge;
mod downloads;
mod i18n;
mod links;
mod location;
mod menu;
mod server;
mod updates;

use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

use serde::Serialize;
use tauri::ipc::CapabilityBuilder;
use tauri::webview::{DownloadEvent, NewWindowResponse, PageLoadEvent};
use tauri::{AppHandle, Manager, RunEvent, Runtime, WebviewUrl, WebviewWindow, WebviewWindowBuilder, WindowEvent};
use tauri_plugin_autostart::ManagerExt as _;
use tauri_plugin_notification::NotificationExt;
use tauri_plugin_opener::OpenerExt;
use url::Url;

use i18n::{t, tf};

/// The main window.
pub const MAIN: &str = "main";
/// The icon next to the clock (Windows and Linux).
pub const TRAY: &str = "marea";

const BRIDGE: &str = include_str!("bridge.js");

#[cfg(target_os = "macos")]
const PLATFORM: &str = "macos";
#[cfg(target_os = "windows")]
const PLATFORM: &str = "windows";
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
const PLATFORM: &str = "linux";

/// What the app keeps in mind while it runs.
#[derive(Default)]
struct Session {
    /// Origin of the open Marea (`https://marea.example.com`): what counts as "inside".
    origin: Mutex<Option<String>>,
    /// Origins already allowed to send notifications and open links.
    granted: Mutex<HashSet<String>>,
    /// Page zoom (1.0 = actual size).
    zoom: Mutex<f64>,
    /// Quitting for real (not just closing the window).
    quitting: AtomicBool,
}

fn current_origin<R: Runtime>(app: &AppHandle<R>) -> Option<String> {
    app.try_state::<Session>().and_then(|s| s.origin.lock().ok().and_then(|o| o.clone()))
}

/// What the connection screen has to show when it opens.
#[derive(Serialize)]
#[serde(tag = "mode", rename_all = "lowercase")]
enum Start {
    /// No Marea chosen yet: it has to be typed.
    Setup,
    /// There is one and it answers: the window is already on its way.
    Connecting { host: String, name: String },
    /// There is one, but it doesn't answer (offline, server down…).
    Offline { host: String, message: String },
}

/// Opens the Marea at `base` in the window, after granting permissions to its exact origin.
fn enter<R: Runtime>(app: &AppHandle<R>, base: &Url) -> Result<(), String> {
    let origin = server::origin(base);
    grant(app, &origin)?;
    if let Some(session) = app.try_state::<Session>() {
        if let Ok(mut current) = session.origin.lock() {
            *current = Some(origin);
        }
    }
    let window = app.get_webview_window(MAIN).ok_or_else(|| t("error.window"))?;
    window.navigate(base.clone()).map_err(|e| tf("error.open", &[("error", &e.to_string())]))
}

/// The argument the app gets when the system opens it at login: it starts with the window hidden.
const AUTOSTART_ARG: &str = "--autostart";

/// Lets that Marea (and only that one) send notifications and open links outside. On macOS, also ask for your
/// location (Core Location shows its own prompt) and move the window by its top bar, which reaches the top of
/// the window there (see NOTES.md, "Permissions").
fn grant<R: Runtime>(app: &AppHandle<R>, origin: &str) -> Result<(), String> {
    let session = app.state::<Session>();
    let mut granted = session.granted.lock().map_err(|_| t("error.internal"))?;
    if granted.contains(origin) {
        return Ok(());
    }
    let capability = CapabilityBuilder::new(format!("marea-remote-{}", granted.len() + 1))
        .remote(format!("{origin}/*"))
        .local(false)
        .window(MAIN)
        .permission("allow-notify")
        .permission("allow-open-external");
    #[cfg(target_os = "macos")]
    let capability = capability
        .permission("allow-locate")
        .permission("core:window:allow-start-dragging")
        .permission("core:window:allow-internal-toggle-maximize");
    app.add_capability(capability).map_err(|e| tf("error.permission", &[("error", &e.to_string())]))?;
    granted.insert(origin.to_string());
    Ok(())
}

/// Tries to open the saved Marea; if it doesn't answer, says so, so the connection screen waits and retries.
async fn open_saved<R: Runtime>(app: &AppHandle<R>) -> Start {
    let Some(base) = server::saved(app) else {
        return Start::Setup;
    };
    let host = base.host_str().unwrap_or_default().to_string();
    match server::probe(&base).await {
        Ok(info) => match enter(app, &base) {
            Ok(()) => Start::Connecting { host, name: info.name },
            Err(message) => Start::Offline { host, message },
        },
        Err(message) => Start::Offline { host, message },
    }
}

/// The address of one of the app's own pages (different on Windows).
fn local_url(path: &str) -> Url {
    #[cfg(target_os = "windows")]
    let base = "http://tauri.localhost/";
    #[cfg(not(target_os = "windows"))]
    let base = "tauri://localhost/";
    Url::parse(base).and_then(|b| b.join(path)).expect("valid local address")
}

/// Back to the connection screen to pick another Marea (from the menu).
fn show_setup<R: Runtime>(app: &AppHandle<R>) {
    show_main(app);
    if let Some(window) = app.get_webview_window(MAIN) {
        let _ = window.navigate(local_url("index.html?change"));
    }
}

/// Brings the main window to the front (from the Dock, the tray icon or a second launch).
pub fn show_main<R: Runtime>(app: &AppHandle<R>) {
    if let Some(window) = app.get_webview_window(MAIN) {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

/// A system notification.
pub(crate) fn notify_system<R: Runtime>(app: &AppHandle<R>, title: &str, body: &str) {
    let _ = app.notification().builder().title(title).body(body).show();
}

fn open_outside<R: Runtime>(app: &AppHandle<R>, url: &Url) {
    let _ = app.opener().open_url(url.as_str(), None::<&str>);
}

fn set_zoom<R: Runtime>(app: &AppHandle<R>, change: Option<f64>) {
    let Some(window) = app.get_webview_window(MAIN) else {
        return;
    };
    let session = app.state::<Session>();
    let Ok(mut zoom) = session.zoom.lock() else {
        return;
    };
    let current = if *zoom <= 0.0 { 1.0 } else { *zoom };
    *zoom = match change {
        None => 1.0,
        Some(factor) => (current * factor).clamp(0.5, 3.0),
    };
    let _ = window.set_zoom(*zoom);
}

// ───────────── Commands of the connection screen ─────────────

#[derive(Serialize)]
struct AppInfo {
    version: &'static str,
    platform: &'static str,
    lang: &'static str,
    strings: &'static HashMap<String, String>,
    server: Option<String>,
}

#[tauri::command]
fn app_info(app: AppHandle) -> AppInfo {
    AppInfo {
        version: env!("CARGO_PKG_VERSION"),
        platform: PLATFORM,
        lang: i18n::lang(),
        strings: i18n::catalog(),
        server: server::saved(&app).and_then(|u| u.host_str().map(String::from)),
    }
}

#[tauri::command]
async fn start(app: AppHandle) -> Start {
    open_saved(&app).await
}

#[tauri::command]
async fn retry(app: AppHandle) -> Start {
    open_saved(&app).await
}

#[tauri::command]
async fn check_server(address: String) -> Result<server::Info, String> {
    let base = server::normalize(&address)?;
    server::probe(&base).await
}

#[tauri::command]
async fn connect(app: AppHandle, address: String) -> Result<server::Info, String> {
    let base = server::normalize(&address)?;
    let info = server::probe(&base).await?;
    server::save(&app, &base)?;
    enter(&app, &base)?;
    Ok(info)
}

#[tauri::command]
fn cancel_setup(app: AppHandle) -> Result<(), String> {
    let base = server::saved(&app).ok_or_else(|| t("error.no_server"))?;
    enter(&app, &base)
}

// ───────────── Commands your Marea may call ─────────────

#[tauri::command]
fn notify(app: AppHandle, title: String, body: String) {
    let title: String = title.chars().filter(|c| !c.is_control()).take(200).collect();
    let body: String = body.chars().filter(|c| *c == '\n' || !c.is_control()).take(1000).collect();
    let title = if title.trim().is_empty() { String::from("Marea") } else { title };
    notify_system(&app, &title, &body);
}

#[tauri::command]
async fn locate(app: AppHandle) -> Result<location::Position, String> {
    location::locate(&app).await.map_err(String::from)
}

#[tauri::command]
fn open_external(app: AppHandle, url: String) -> Result<(), String> {
    let url = links::external_ok(&url).ok_or_else(|| t("error.link"))?;
    app.opener().open_url(url.as_str(), None::<&str>).map_err(|e| tf("error.link_open", &[("error", &e.to_string())]))
}

// ───────────── The window ─────────────

fn build_main<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<WebviewWindow<R>> {
    let script = BRIDGE.replace("__VERSION__", env!("CARGO_PKG_VERSION")).replace("__PLATFORM__", PLATFORM);
    let on_nav = app.clone();
    let on_new = app.clone();
    let on_download = app.clone();
    let on_load = app.clone();

    let builder = WebviewWindowBuilder::new(app, MAIN, WebviewUrl::App("index.html".into()))
        .title("Marea")
        .inner_size(1280.0, 840.0)
        .min_inner_size(420.0, 560.0)
        .center()
        // Opened by the system at login: start in the Dock or next to the clock, ready for notifications
        .visible(!std::env::args().any(|arg| arg == AUTOSTART_ARG));

    // macOS: no grey title bar. The page reaches the top of the window and the window buttons sit inside
    // Marea's own top bar (the bridge makes room for them and makes that bar the place to drag the window from).
    #[cfg(target_os = "macos")]
    let builder = builder
        .title_bar_style(tauri::TitleBarStyle::Overlay)
        .hidden_title(true)
        .traffic_light_position(tauri::LogicalPosition::new(18.0, 28.0));

    builder
        .initialization_script(script)
        .zoom_hotkeys_enabled(true)
        .on_navigation(move |url| match links::on_navigation(url) {
            links::Decision::Allow => true,
            links::Decision::External => {
                open_outside(&on_nav, url);
                false
            }
            links::Decision::Block => false,
        })
        .on_new_window(move |url, _features| match links::on_new_window(&url, current_origin(&on_new).as_deref()) {
            links::Decision::Allow => NewWindowResponse::Allow,
            links::Decision::External => {
                open_outside(&on_new, &url);
                NewWindowResponse::Deny
            }
            links::Decision::Block => NewWindowResponse::Deny,
        })
        .on_download(move |_webview, event| match event {
            DownloadEvent::Requested { url, destination } => downloads::requested(&on_download, &url, destination),
            DownloadEvent::Finished { url, path, success } => {
                downloads::finished(&on_download, &url, path.as_deref(), success);
                true
            }
            _ => true,
        })
        .on_document_title_changed(|window, title| {
            let shown = if title.trim().is_empty() { "Marea" } else { title.as_str() };
            let _ = window.set_title(shown);
            badge::apply(&window, badge::count_from_title(&title));
            #[cfg(not(target_os = "macos"))]
            if let Some(tray) = window.app_handle().tray_by_id(TRAY) {
                let _ = tray.set_tooltip(Some(shown));
            }
        })
        .on_page_load(move |window, payload| {
            if payload.event() != PageLoadEvent::Finished {
                return;
            }
            let url = payload.url().clone();
            if links::main_frame_is_foreign(&url, current_origin(&on_load).as_deref()) {
                open_outside(&on_load, &url);
                let _ = window.eval("history.back()");
            }
        })
        .build()
}

/// Runs one of Marea's shortcuts (`actions.rs`) in the window, only while it shows your Marea (not the connection
/// or offline screen).
fn run_in_marea<R: Runtime>(app: &AppHandle<R>, js: &str) {
    let Some(window) = app.get_webview_window(MAIN) else {
        return;
    };
    let on_marea = window.url().is_ok_and(|url| Some(server::origin(&url)) == current_origin(app));
    if on_marea {
        show_main(app);
        let _ = window.eval(js);
    }
}

fn on_menu<R: Runtime>(app: &AppHandle<R>, id: &str) {
    if let Some(js) = actions::script(id) {
        run_in_marea(app, js);
        return;
    }
    let window = app.get_webview_window(MAIN);
    let run = |js: &str| {
        if let Some(w) = &window {
            let _ = w.eval(js);
        }
    };
    match id {
        menu::CHANGE_SERVER => show_setup(app),
        menu::CHECK_UPDATES => updates::check_now(app.clone()),
        menu::RELOAD => run("location.reload()"),
        menu::BACK => run("history.back()"),
        menu::FORWARD => run("history.forward()"),
        menu::ZOOM_IN => set_zoom(app, Some(1.1)),
        menu::ZOOM_OUT => set_zoom(app, Some(1.0 / 1.1)),
        menu::ZOOM_RESET => set_zoom(app, None),
        menu::PRINT => {
            if let Some(w) = &window {
                let _ = w.print();
            }
        }
        menu::OPEN_AT_LOGIN => toggle_open_at_login(app),
        menu::HELP => {
            let _ = app.opener().open_url("https://github.com/mareabypaperclip/marea-desktop", None::<&str>);
        }
        menu::OPEN => show_main(app),
        menu::QUIT => {
            app.state::<Session>().quitting.store(true, Ordering::SeqCst);
            app.exit(0);
        }
        _ => {}
    }
}

/// "Open at login" on or off, and the menu tick to match.
fn toggle_open_at_login<R: Runtime>(app: &AppHandle<R>) {
    let autostart = app.autolaunch();
    let on = !autostart.is_enabled().unwrap_or(false);
    let done = if on { autostart.enable() } else { autostart.disable() };
    let now = autostart.is_enabled().unwrap_or(false);
    menu::set_open_at_login(app, now);
    if done.is_err() || now != on {
        notify_system(app, "Marea", &t("error.open_at_login"));
    }
}

/// Closing the window keeps Marea running (so notifications keep arriving): in the Dock on macOS, next to the
/// clock on Windows and Linux. The first time, on Windows and Linux, a notification says where it went.
fn hide_instead_of_close<R: Runtime>(window: &tauri::Window<R>) {
    let _ = window.hide();
    #[cfg(not(target_os = "macos"))]
    {
        let app = window.app_handle();
        if server::get_flag(app, "tray_hint_shown").is_none() {
            server::set_flag(app, "tray_hint_shown", serde_json::Value::Bool(true));
            notify_system(app, &t("tray.hint.title"), &t("tray.hint.body"));
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    use tauri_plugin_window_state::StateFlags;

    let app = tauri::Builder::default()
        // A single Marea: launching it again brings back the window that was already open
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| show_main(app)))
        .plugin(
            tauri_plugin_window_state::Builder::default()
                .with_state_flags(StateFlags::all() & !StateFlags::VISIBLE)
                .build(),
        )
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_autostart::Builder::new().args([AUTOSTART_ARG]).app_name("Marea").build())
        .manage(Session::default())
        .manage(downloads::Downloads::default())
        .invoke_handler(tauri::generate_handler![
            app_info,
            start,
            retry,
            check_server,
            connect,
            cancel_setup,
            notify,
            locate,
            open_external
        ])
        .on_menu_event(|app, event| on_menu(app, event.id().as_ref()))
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                let quitting = window.app_handle().state::<Session>().quitting.load(Ordering::SeqCst);
                if window.label() == MAIN && !quitting {
                    api.prevent_close();
                    hide_instead_of_close(window);
                }
            }
        })
        .setup(|app| {
            let handle = app.handle();
            #[cfg(target_os = "macos")]
            app.set_menu(menu::app_menu(handle)?)?;
            #[cfg(not(target_os = "macos"))]
            menu::install_tray(handle)?;
            build_main(handle)?;
            updates::schedule(handle.clone());
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("Marea could not start");

    app.run(|app, event| match event {
        // macOS: clicking the Dock icon with the window closed opens it again
        #[cfg(target_os = "macos")]
        RunEvent::Reopen { .. } => show_main(app),
        RunEvent::ExitRequested { .. } => {
            app.state::<Session>().quitting.store(true, Ordering::SeqCst);
        }
        _ => {}
    });
}
