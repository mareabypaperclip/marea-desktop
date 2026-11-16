//! Menus. On macOS, the usual menu bar (with Edit, so ⌘C and ⌘V work). On Windows and Linux there is no menu bar
//! (like Slack), and an icon next to the clock opens Marea, changes server or quits: closing the window keeps the
//! app running so notifications keep arriving.

use tauri::menu::{Menu, MenuBuilder, MenuItemBuilder};
use tauri::{AppHandle, Runtime};

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
pub const QUIT: &str = "quit";

/// The macOS menu bar.
#[cfg(target_os = "macos")]
pub fn app_menu<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<Menu<R>> {
    use tauri::menu::{AboutMetadataBuilder, PredefinedMenuItem, SubmenuBuilder};

    let about = AboutMetadataBuilder::new()
        .name(Some("Marea"))
        .version(Some(env!("CARGO_PKG_VERSION")))
        .copyright(Some(t("about.copyright")))
        .website(Some("https://github.com/mareabypaperclip/marea-desktop"))
        .website_label(Some(t("menu.help_github")))
        .build();
    let label = |key: &str| Some(t(key));

    let marea = SubmenuBuilder::new(app, "Marea")
        .item(&PredefinedMenuItem::about(app, label("menu.about").as_deref(), Some(about))?)
        .separator()
        .item(&MenuItemBuilder::with_id(CHECK_UPDATES, t("menu.check_updates")).build(app)?)
        .item(&MenuItemBuilder::with_id(CHANGE_SERVER, t("menu.change_server")).build(app)?)
        .separator()
        .item(&PredefinedMenuItem::services(app, label("menu.services").as_deref())?)
        .separator()
        .item(&PredefinedMenuItem::hide(app, label("menu.hide").as_deref())?)
        .item(&PredefinedMenuItem::hide_others(app, label("menu.hide_others").as_deref())?)
        .item(&PredefinedMenuItem::show_all(app, label("menu.show_all").as_deref())?)
        .separator()
        .item(&PredefinedMenuItem::quit(app, label("menu.quit").as_deref())?)
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
        .item(&MenuItemBuilder::with_id(RELOAD, t("menu.reload")).accelerator("CmdOrCtrl+R").build(app)?)
        .item(&MenuItemBuilder::with_id(BACK, t("menu.back")).accelerator("CmdOrCtrl+[").build(app)?)
        .item(&MenuItemBuilder::with_id(FORWARD, t("menu.forward")).accelerator("CmdOrCtrl+]").build(app)?)
        .separator()
        .item(&MenuItemBuilder::with_id(ZOOM_RESET, t("menu.actual_size")).accelerator("CmdOrCtrl+0").build(app)?)
        .item(&MenuItemBuilder::with_id(ZOOM_IN, t("menu.zoom_in")).accelerator("CmdOrCtrl+Plus").build(app)?)
        .item(&MenuItemBuilder::with_id(ZOOM_OUT, t("menu.zoom_out")).accelerator("CmdOrCtrl+-").build(app)?)
        .separator()
        .item(&PredefinedMenuItem::fullscreen(app, label("menu.fullscreen").as_deref())?)
        .build()?;

    let window = SubmenuBuilder::new(app, t("menu.window"))
        .item(&PredefinedMenuItem::minimize(app, label("menu.minimize").as_deref())?)
        .item(&PredefinedMenuItem::maximize(app, label("menu.zoom").as_deref())?)
        .separator()
        .item(&PredefinedMenuItem::close_window(app, label("menu.close_window").as_deref())?)
        .build()?;

    let help = SubmenuBuilder::new(app, t("menu.help"))
        .item(&MenuItemBuilder::with_id(HELP, t("menu.help_github")).build(app)?)
        .build()?;

    MenuBuilder::new(app).items(&[&marea, &edit, &view, &window, &help]).build()
}

/// The menu of the icon next to the clock (Windows and Linux).
#[cfg(not(target_os = "macos"))]
pub fn tray_menu<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<Menu<R>> {
    MenuBuilder::new(app)
        .item(&MenuItemBuilder::with_id(OPEN, t("menu.open")).build(app)?)
        .separator()
        .item(&MenuItemBuilder::with_id(CHANGE_SERVER, t("menu.change_server")).build(app)?)
        .item(&MenuItemBuilder::with_id(CHECK_UPDATES, t("menu.check_updates")).build(app)?)
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
