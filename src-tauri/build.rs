// The app's own commands get their own permissions ("allow-…"), so each page can only use its own: the
// connection screen, the setup commands; the remote Marea, only notifications, opening links outside, the number on
// the app's icon and (macOS) its location.
const COMMANDS: &[&str] = &[
    // Connection screen (the app's own page)
    "app_info",
    "start",
    "check_server",
    "connect",
    "retry",
    "cancel_setup",
    // The remote Marea
    "notify",
    "locate",
    "set_badge",
    "open_external",
];

fn main() {
    tauri_build::try_build(
        tauri_build::Attributes::new().app_manifest(tauri_build::AppManifest::new().commands(COMMANDS)),
    )
    .expect("could not prepare the app");
}
