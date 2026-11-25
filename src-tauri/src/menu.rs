//! Menus. On macOS, the usual menu bar (with Edit, so ⌘C and ⌘V work) plus Marea's own shortcuts: new wave,
//! compose, search and a Go menu with every section (⌘1 to ⌘7). On Windows and Linux there is no menu bar (like
//! Slack): an icon next to the clock opens Marea, has the main shortcuts, changes server or quits, and closing the
//! window keeps the app running so notifications keep arriving. The section keys (Ctrl+1 to Ctrl+7) are handled
//! by the bridge there.

use tauri::menu::{CheckMenuItem, CheckMenuItemBuilder, Menu, MenuBuilder, MenuItemBuilder};
use tauri::{AppHandle, Manager, Runtime};

use tauri_plugin_autostart::ManagerExt as _;

use crate::actions;
use crate::i18n::t;

pub const CHANGE_SERVER: &str = "change_server";
pub const CHECK_UPDATES: &str = "check_updates";
pub const RELOAD: &str = "reload";
pub const BACK: &str = "back";
pub const FORWARD: &str = "forward";
pub const ZOOM_IN: &str = "zoom_in";
pub const ZOOM_OUT: &str = "zoom_out";
pub const ZOOM_RESET: &str = "zoom_reset";
pub const HELP: &str = "help";
pub const OPEN: &str = "open";
pub const PRINT: &str = "print";
pub const OPEN_AT_LOGIN: &str = "open_at_login";
pub const QUIT: &str = "quit";

/// The "Open at Login" ticks (app menu on macOS, tray menu elsewhere), kept to update them when it changes.
struct LoginItems<R: Runtime>(std::sync::Mutex<Vec<CheckMenuItem<R>>>);

fn open_at_login_item<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<CheckMenuItem<R>> {
    let on = app.autolaunch().is_enabled().unwrap_or(false);
    let item = CheckMenuItemBuilder::with_id(OPEN_AT_LOGIN, t("menu.open_at_login")).checked(on).build(app)?;
    if app.try_state::<LoginItems<R>>().is_none() {
        app.manage(LoginItems::<R>(std::sync::Mutex::new(Vec::new())));
    }
    if let Ok(mut items) = app.state::<LoginItems<R>>().0.lock() {
        items.push(item.clone());
    }
    Ok(item)
}

/// Puts the tick where it belongs after turning "Open at Login" on or off.
pub fn set_open_at_login<R: Runtime>(app: &AppHandle<R>, on: bool) {
    if let Some(items) = app.try_state::<LoginItems<R>>() {
        if let Ok(items) = items.0.lock() {
            for item in items.iter() {
                let _ = item.set_checked(on);
            }
        }
    }
}

/// The macOS menu bar. Compiled everywhere, so the checks on Linux cover it too; only macOS uses it.
/// What «About Marea» shows on every system: name, version, the copyright with the licence and «no warranty» (the
/// AGPL asks interactive programs to show both) and where the source code is.
fn about_metadata() -> tauri::menu::AboutMetadata<'static> {
    tauri::menu::AboutMetadataBuilder::new()
        .name(Some("Marea"))
        .version(Some(env!("CARGO_PKG_VERSION")))
        .copyright(Some(t("about.copyright")))
        .license(Some("AGPL-3.0"))
        .website(Some("https://github.com/mareabypaperclip/marea-desktop"))
        .website_label(Some(t("menu.help_github")))
        .build()
}

#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub fn app_menu<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<Menu<R>> {
    use tauri::menu::{PredefinedMenuItem, SubmenuBuilder};

    let about = about_metadata();
    let label = |key: &str| Some(t(key));

    let item = |id: &str, key: &str, accelerator: Option<&str>| {
        let builder = MenuItemBuilder::with_id(id, t(key));
        match accelerator {
            Some(keys) => builder.accelerator(keys).build(app),
            None => builder.build(app),
        }
    };

    let marea = SubmenuBuilder::new(app, "Marea")
        .item(&PredefinedMenuItem::about(app, label("menu.about").as_deref(), Some(about))?)
        .separator()
        .item(&item(actions::SETTINGS, "menu.settings", Some("CmdOrCtrl+Comma"))?)
        .item(&open_at_login_item(app)?)
        .separator()
        .item(&item(CHECK_UPDATES, "menu.check_updates", None)?)
        .item(&item(CHANGE_SERVER, "menu.change_server", None)?)
        .separator()
        .item(&PredefinedMenuItem::services(app, label("menu.services").as_deref())?)
        .separator()
        .item(&PredefinedMenuItem::hide(app, label("menu.hide").as_deref())?)
        .item(&PredefinedMenuItem::hide_others(app, label("menu.hide_others").as_deref())?)
        .item(&PredefinedMenuItem::show_all(app, label("menu.show_all").as_deref())?)
        .separator()
        .item(&PredefinedMenuItem::quit(app, label("menu.quit").as_deref())?)
        .build()?;

    // ⌘K stays with the page (inside an editor it means "link"), so the Assistant item has no shortcut of its own.
    let file = SubmenuBuilder::new(app, t("menu.file"))
        .item(&item(actions::NEW_WAVE, "menu.new_wave", Some("CmdOrCtrl+N"))?)
        .item(&item(actions::COMPOSE, "menu.compose", Some("CmdOrCtrl+Shift+N"))?)
        .separator()
        .item(&item(actions::SEARCH, "menu.search", Some("CmdOrCtrl+F"))?)
        .item(&item(actions::ASSISTANT, "menu.assistant", None)?)
        .separator()
        .item(&item(PRINT, "menu.print", Some("CmdOrCtrl+P"))?)
        .separator()
        .item(&PredefinedMenuItem::close_window(app, label("menu.close_window").as_deref())?)
        .build()?;

    let edit = SubmenuBuilder::new(app, t("menu.edit"))
        .item(&PredefinedMenuItem::undo(app, label("menu.undo").as_deref())?)
        .item(&PredefinedMenuItem::redo(app, label("menu.redo").as_deref())?)
        .separator()
        .item(&PredefinedMenuItem::cut(app, label("menu.cut").as_deref())?)
        .item(&PredefinedMenuItem::copy(app, label("menu.copy").as_deref())?)
        .item(&PredefinedMenuItem::paste(app, label("menu.paste").as_deref())?)
        .item(&PredefinedMenuItem::select_all(app, label("menu.select_all").as_deref())?)
        .build()?;

    let view = SubmenuBuilder::new(app, t("menu.view"))
        .item(&item(RELOAD, "menu.reload", Some("CmdOrCtrl+R"))?)
        .separator()
        .item(&item(ZOOM_RESET, "menu.actual_size", Some("CmdOrCtrl+0"))?)
        .item(&item(ZOOM_IN, "menu.zoom_in", Some("CmdOrCtrl+Plus"))?)
        .item(&item(ZOOM_OUT, "menu.zoom_out", Some("CmdOrCtrl+-"))?)
        .separator()
        .item(&PredefinedMenuItem::fullscreen(app, label("menu.fullscreen").as_deref())?)
        .build()?;

    let mut go = SubmenuBuilder::new(app, t("menu.go"));
    for (n, &(id, key)) in actions::SECTIONS.iter().enumerate() {
        go = go.item(&item(id, key, Some(format!("CmdOrCtrl+{}", n + 1).as_str()))?);
    }
    let go = go
        .separator()
        .item(&item(BACK, "menu.back", Some("CmdOrCtrl+["))?)
        .item(&item(FORWARD, "menu.forward", Some("CmdOrCtrl+]"))?)
        .build()?;

    let window = SubmenuBuilder::new(app, t("menu.window"))
        .item(&PredefinedMenuItem::minimize(app, label("menu.minimize").as_deref())?)
        .item(&PredefinedMenuItem::maximize(app, label("menu.zoom").as_deref())?)
        .build()?;

    let help = SubmenuBuilder::new(app, t("menu.help"))
        .item(&item(actions::SHORTCUTS, "menu.shortcuts", None)?)
        .separator()
        .item(&item(HELP, "menu.help_github", None)?)
        .build()?;

    MenuBuilder::new(app).items(&[&marea, &file, &edit, &view, &go, &window, &help]).build()
}

/// The menu of the icon next to the clock (Windows and Linux).
#[cfg(not(target_os = "macos"))]
pub fn tray_menu<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<Menu<R>> {
    let item = |id: &str, key: &str| MenuItemBuilder::with_id(id, t(key)).build(app);
    MenuBuilder::new(app)
        .item(&item(OPEN, "menu.open")?)
        .item(&tauri::menu::PredefinedMenuItem::about(app, Some(&t("menu.about")), Some(about_metadata()))?)
        .separator()
        .item(&item(actions::NEW_WAVE, "menu.new_wave")?)
        .item(&item(actions::COMPOSE, "menu.compose")?)
        .separator()
        .item(&item(actions::MAIL, "menu.mail")?)
        .item(&item(actions::AGENDA, "menu.agenda")?)
        .item(&item(actions::SETTINGS, "menu.settings")?)
        .separator()
        .item(&MenuItemBuilder::with_id(CHANGE_SERVER, t("menu.change_server")).build(app)?)
        .item(&MenuItemBuilder::with_id(CHECK_UPDATES, t("menu.check_updates")).build(app)?)
        .item(&open_at_login_item(app)?)
        .separator()
        .item(&MenuItemBuilder::with_id(QUIT, t("menu.quit")).build(app)?)
        .build()
}

/// Puts the icon next to the clock (Windows and Linux).
#[cfg(not(target_os = "macos"))]
pub fn install_tray<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};

    let menu = tray_menu(app)?;
    let mut tray = TrayIconBuilder::with_id(crate::TRAY)
        .tooltip("Marea")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                crate::show_main(tray.app_handle());
            }
        });
    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }
    tray.build(app)?;
    Ok(())
}
