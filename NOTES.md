# Technical notes

What a future session needs to know to keep working on Marea for desktop. Read it before changing anything.

## How it works

- **Tauri 2** (Rust + the system webview: WKWebView on macOS, WebView2 on Windows, WebKitGTK on Linux). No
  frontend framework; the only local page is the connection screen in `ui/`.
- **The app doesn't bundle Marea.** On launch it reads the saved server (`settings.json` in the app data folder,
  via `tauri-plugin-store`), checks it answers `api.php?a=boot` with a `version` and an `instance`, grants
  permissions to that exact origin and navigates the main window there.
- **Permissions (capabilities):**
  - `capabilities/local.json`: the connection screen may call `app_info`, `start`, `check_server`, `connect`,
    `retry`, `cancel_setup` and `open_external`.
  - At runtime, `grant()` in `lib.rs` adds a capability for `https://<server>/*` with only `allow-notify` and
    `allow-open-external`. The plugins (notification, opener, dialog, store) are used from Rust only: no page has
    their JavaScript permissions.
- **The bridge** (`src-tauri/src/bridge.js`, injected before every page): replaces `window.Notification` so
  Marea's standard notifications become system notifications, sends clicks on links to other sites to the
  browser, and exposes `window.__MAREA_DESKTOP__ = { version, platform }`.
- **Badge:** Marea already writes the count in the page title ("(3) Marea"); `on_document_title_changed` reads it.
  macOS and Linux use `set_badge_count`; Windows uses an overlay icon (`icons/badges/*.png`, 1–9 and 9+).
- **Links:** see the comment at the top of `links.rs`. On macOS the navigation handler can't tell the main frame
  from inner frames (videos, emails), so web navigations are allowed there and `on_page_load` corrects a whole
  window that ended up on another site.
- **Closing the window hides it** (Dock on macOS, tray on Windows/Linux); quitting is *Quit Marea*. A second
  launch brings the window back (`tauri-plugin-single-instance`). Size and position are remembered
  (`tauri-plugin-window-state`, without the visibility flag).
- **Languages:** `ui/locales/{en,es,fr,ca}.json`, shared by Rust (`i18n.rs`) and the connection screen (which gets
  the active catalog from `app_info`). A test checks every catalog has every key and every placeholder. Galician,
  Basque, Asturian and Aragonese systems get Spanish; anything else unknown gets English. The Windows installer
  is in English, Spanish and French (Tauri's NSIS templates have no Catalan).
- **Updates:** `updates.rs` checks the latest GitHub release at launch and every 12 hours and offers the download
  page. Automatic installation (`tauri-plugin-updater`) needs a signing key stored as repository secrets
  (`TAURI_SIGNING_PRIVATE_KEY` and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`). Agent sessions can't set GitHub secrets,
  and the private key must never travel through a chat, so the owner has to generate it on his computer
  (`npx tauri signer generate -w ~/.tauri/marea.key`), paste both values into *Settings → Secrets and variables →
  Actions*, and give the session only the public key.

## Releasing

1. Bump the version in `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml` and `package.json` (same number).
2. Commit, then tag `vX.Y.Z` and push the tag. `.github/workflows/release.yml` creates a draft release, builds
   macOS (universal), Windows and Linux in parallel and publishes the release only if all three succeed.
3. Or run the *Release* workflow by hand from the Actions tab.

## Testing locally (Linux, headless)

```sh
npx tauri build --debug --no-bundle     # plain `cargo build` has no embedded pages (blank window)
Xvfb :99 -screen 0 1440x900x24 &
DISPLAY=:99 LANG=es_ES.UTF-8 WEBKIT_DISABLE_SANDBOX_THIS_IS_DANGEROUS=1 WEBKIT_DISABLE_DMABUF_RENDERER=1 \
  LIBGL_ALWAYS_SOFTWARE=1 dbus-run-session -- src-tauri/target/debug/marea-desktop
DISPLAY=:99 import -window root shot.png   # ImageMagick; drive it with xdotool
```

`http://localhost…` is accepted as a server for testing (everything else must be https); the Marea test install of
the main repository (`http://localhost:8090/dev/`) works.

## Known limitations (0.1)

- **Unsigned:** macOS Gatekeeper and Windows SmartScreen warn on first launch. Signing needs an Apple Developer
  account and a Windows code-signing certificate.
- **Notifications only cover what Marea sends through the standard API while the window is hidden:** today,
  calendar reminders and incoming calls. Mentions and messages arrive in Marea through Web Push, which webviews
  don't support; Marea itself has to start showing them with `Notification` when the page isn't focused (noted
  in Marea's roadmap). Marea checks `document.visibilityState`, so a window that is visible but behind others
  gets no notification yet.
- **Passkeys** may not work inside the macOS webview (WKWebView needs associated-domain entitlements, which need
  signing). Password + 2FA code works.
- **Screen sharing** in calls depends on each webview; untested.
- **Clicking a notification** brings the app forward on macOS; it doesn't open the related wave yet.
