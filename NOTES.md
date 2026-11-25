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
  - At runtime, `grant()` in `lib.rs` adds a capability for `https://<server>/*` with only `allow-notify`,
    `allow-open-external` and `allow-set-badge`, and a few more on macOS and Windows, each with its reason:
    - `allow-set-badge` (0.2.1): the number on the app's icon. Marea's page title only has the unread waves
      ("(3) Marea"), and the unread mail is just as pending, so the bridge counts both and hands the app one
      whole number. The worst a page could do with it is show a wrong number on the icon.
    - `allow-locate`: WKWebView never gives pages your location inside apps, so the bridge asks the app, which
      asks Core Location (`location.rs`). macOS shows its own permission prompt; kilometre accuracy only.
    - `core:window:allow-start-dragging` and `core:window:allow-internal-toggle-maximize`: with the title bar
      overlaid, Marea's top bar is the only place to drag the window from (and double-click to zoom). The worst
      a page could do with them is move or zoom the window.
    - Windows (0.2.3): the window has no Windows frame (`decorations(false)` with `shadow(true)`, like Slack),
      so the same two, plus `core:window:allow-minimize`, `allow-toggle-maximize`, `allow-is-maximized` and
      `allow-close`, for the buttons the bridge draws at the top right. The worst a page could do with them is
      move, minimize, maximize or close the window, and closing only hides it, as the real button does.
    The plugins (notification, opener, dialog, store, autostart) are used from Rust only: no page has their
    JavaScript permissions. The connection screen also gets the window permissions, for its drag strip and, on
    Windows, its window buttons.
- **The bridge** (`src-tauri/src/bridge.js`, injected before every page): replaces `window.Notification` so
  Marea's standard notifications become system notifications, sends clicks on links to other sites to the
  browser, and exposes `window.__MAREA_DESKTOP__ = { version, platform, openInBrowser, vibrancy }` (read-only;
  `vibrancy` only when it is `true`).
  `openInBrowser(url)` (0.2.4) exists only in Marea's top window (not on the connection screen nor in inner
  frames), accepts only addresses of the page's own origin and sends them to `open_external`, which checks them
  again; it resolves when the browser was asked and rejects anything else. No new permission: the page already had
  `allow-open-external`, and only your Marea's origin has it. It is a contract the other way round: Marea uses it,
  when it finds it, to link Google and Microsoft in the browser (`docs/browser-sign-in.md`).
- **The system's material behind the window (0.4, `vibrancy`):** macOS puts the sidebar material (`Effect::Sidebar`,
  following the window's active state) behind a transparent window, and Windows 11 (build 22000 and later, read with
  `windows-version`, which Tauri already uses) puts Mica. Windows 10 stays opaque (Acrylic lags when the window
  moves) and Linux has no effect. `has_material()` in `lib.rs` decides both the effect and the bridge's
  `vibrancy: true`, so Marea never makes its sidebar transparent over a window with nothing behind it (it would
  show the desktop). Marea 0.25.1 and later read it once, when the page loads, and only paint their sidebar
  transparent (with a veil of the panel colour) while signed in and with the window 900 px wide or more; everything
  else stays opaque, and so do the connection and offline screens (`body` has a background). Since Tauri 2.12.1
  transparent windows on macOS no longer need `macOSPrivateApi`. Untested on a real Mac and a real Windows 11.
- **New mail notifications (0.4):** once a minute the bridge calls Marea's `api.php?a=mail_new&since=MARK`
  (Marea 0.25.1; contract in Marea's `dev/briefs/app-escritorio.md`): `{ mark, items: [{ box, addr, id, from,
  subject, date, url }], quiet }`. The first call has no `since` and only returns the mark, so nothing old is
  announced; the mark lives in the page (a reload starts again). Marea leaves out mail read elsewhere, blocked
  senders and everything during «No molestar» or «solo menciones» (`quiet`), and refuses more than 240 calls an
  hour. A 4xx other than 429 on the first call means an older Marea: the bridge stops until the page loads again.
  At most 3 notifications a minute (title: sender, body: subject), never while you're looking at the window; the
  newest one's `url` (`#/mail/b/BOX/ID`, checked against that shape) opens on focus within 30 s, like the waves.
- **About (0.4):** `about_metadata()` in `menu.rs`, the same on every system: name, version, the copyright with the
  licence and «no warranty» (`about.copyright`; the AGPL asks interactive programs to show both) and the
  repository. macOS has it in the app menu; Windows and Linux, in the menu of the icon next to the clock.
- **Windows title bar (0.2.3):** Marea reaches the top of the window, as on macOS. `windowButtons()` in the
  bridge draws minimize, maximize or restore, and close at the top right with Windows' sizes (46 × 32 px) and
  colours (red to close), drawn as SVG so they look the same on Windows 10 and 11, in the colour of Marea's top
  bar; their labels come from the locale files (`window.*`). Marea's bar gets `padding-right` for them and is the
  drag region; pages without it get a thin strip. Google's and Microsoft's sign-in pages can't draw the buttons,
  so `on_page_load` gives the window its Windows frame back there and takes it away on Marea and the connection
  screen. Windows 11's Snap layouts don't appear when hovering our maximize button (they need the real one);
  Win+Z, Win+arrows and dragging to the edges still work.
- **Badge (0.2.1):** unread waves plus unread mail, the two numbers next to «Waves» and «Correo» in Marea's
  menu. With the window on screen the bridge takes them from Marea: the waves from the page title ("(3) Marea"),
  the mail from the menu's counter (`a[data-nav="mail"]:not(.sub) .badge`) each time Marea fires
  `marea:mailcount`. With the window hidden Marea stops asking its server, so the bridge asks: the waves from the
  `api.php?a=inbox` request it already makes for notifications (unread, not archived nor in the trash), the mail
  from `api.php?a=panel_mail` every 5 minutes (the sum of `boxes[].unseen`), as often as Marea itself. It calls
  `set_badge`; macOS and Linux use `set_badge_count` (Linux only on docks that support the Unity launcher API);
  Windows uses an overlay icon (`icons/badges/*.png`, 1–9 and 9+). Until 0.2.0 the app read only the title.
  These parts of Marea are a contract: Marea's brief lists them under "don't break the desktop app".
- **Drag and drop belongs to Marea** (0.2.1): the window is built with `disable_drag_drop_handler()`. With
  Tauri's handler on, every drag over the window went to the app (for file drops into the app itself): on macOS
  and Windows Marea never saw a single drag (mail onto a folder, files into an email or a transfer), and on Linux
  it never saw files. The app has no use for dropped files of its own.
- **Links:** see the comment at the top of `links.rs`. On macOS the navigation handler can't tell the main frame
  from inner frames (videos, emails), so web navigations are allowed there and `on_page_load` corrects a whole
  window that ended up on another site. Google's and Microsoft's sign-in pages stay allowed inside, because
  Mareas before 0.25 still link that way; from 0.25 Marea sends that step to the browser with `openInBrowser`
  (`docs/browser-sign-in.md`).
- **Marea's shortcuts in the menus** (`actions.rs`): each menu item runs a fixed script in the Marea page that
  changes `location.hash` (`#/mail`, `#/new`…) or presses one of Marea's keys on the page body (⌘K for the
  Assistant, `?` for its shortcuts). They only run while the window shows the chosen Marea. ⌘K has no native
  accelerator on purpose: inside Marea's editor it means "link". On Windows and Linux there is no menu bar, so the
  bridge handles Ctrl+1–7 and Ctrl+, itself; its list must match `actions::SECTIONS` (a test checks it). If Marea
  changes a route, update both and run `node tools/check-actions.mjs`, which runs every script in Marea's test
  install and checks what it did.
  `node tools/check-bridge.mjs` does the same for the bridge (as macOS, with a fake app behind it): new-message
  notifications, focus opening the wave, the top bar's room and drag region, and location through Marea's
  weather dialog.
- **New-message notifications** (in the bridge): every 45 s the page fetches `api.php?a=inbox` (the same list
  Marea shows; the server already applies each wave's notify setting to `unread`) and notifies waves whose
  `updated` grew while unread, only when the window isn't focused. The first look is only a baseline. Focus
  within 30 s of a notification opens its wave (clicking a notification only brings the app forward; the
  notification plugin has no click callback on desktop). Hidden windows may have their timers throttled by
  WebKit or App Nap: untested on a real Mac how late they arrive.
- **macOS title bar:** `TitleBarStyle::Overlay`, hidden title and the traffic lights at (18, 28), centred on
  Marea's 60 px top bar. The bridge gives `header.top` 86 px of left padding and `data-tauri-drag-region="deep"`
  (Tauri's drag script skips its buttons and fields), through the CSSOM because Marea's CSP blocks `<style>`.
  Pages without that bar (sign-in, public pages) get a 28 px invisible drag strip; so does the connection screen.
- **macOS entitlements** (`Entitlements.plist`): the bundle is signed with the hardened runtime, which blocks
  the camera, the microphone and location unless declared. Without them, calls would fail silently.
- **Open at login:** `tauri-plugin-autostart` (LaunchAgent on macOS, registry on Windows, `~/.config/autostart`
  on Linux) with the `--autostart` argument, which starts the window hidden.
- **Closing the window hides it** (Dock on macOS, tray on Windows/Linux); quitting is *Quit Marea*. A second
  launch brings the window back (`tauri-plugin-single-instance`). Size and position are remembered
  (`tauri-plugin-window-state`, without the visibility flag).
- **Languages:** `ui/locales/{en,es,fr,ca}.json`, shared by Rust (`i18n.rs`) and the connection screen (which gets
  the active catalog from `app_info`). A test checks every catalog has every key and every placeholder. Galician,
  Basque, Asturian and Aragonese systems get Spanish; anything else unknown gets English. The Windows installer
  is in English, Spanish and French (Tauri's NSIS templates have no Catalan).
- **Updates:** `updates.rs` checks the latest GitHub release at launch and every 12 hours and offers the download
  page. It reads the tag from the redirect of `github.com/…/releases/latest` (no request limit) and only falls
  back to the API, which allows 60 requests an hour per connection without an account: 0.2.1 used only the API
  and got "403" on a busy connection. Automatic installation (`tauri-plugin-updater`) needs a signing key stored as repository secrets
  (`TAURI_SIGNING_PRIVATE_KEY` and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`). Agent sessions can't set GitHub secrets,
  and the private key must never travel through a chat, so the owner has to generate it on his computer
  (`npx tauri signer generate -w ~/.tauri/marea.key`), paste both values into *Settings → Secrets and variables →
  Actions*, and give the session only the public key.

## Releasing

1. Bump the version in `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml` and `package.json` (same number), and
   add a `## X.Y.Z` section to `CHANGELOG.md`: the release notes' *What's new* comes from it.
2. Commit, then tag `vX.Y.Z` and push the tag. `.github/workflows/release.yml` creates a draft release, builds
   macOS (universal), Windows and Linux in parallel and publishes the release only if all three succeed.
3. Or run the *Release* workflow by hand from the Actions tab. **From Claude's sessions this is the only way:**
   pushing a tag is refused there, so merge the version's pull request and start the workflow on `main`
   (`gh api -X POST repos/mareabypaperclip/marea-desktop/actions/workflows/release.yml/dispatches -f ref=main`). The
   tag is then created when the draft is published, on whatever `main` is at that moment: merge nothing else into
   `main` until the release is out, or the tag lands on the wrong commit.
4. **GitHub's Mac machines are often unavailable** (November 2026: jobs cancelled after 15 minutes without ever
   getting a machine, on Intel and Apple Silicon alike; GitHub's own issue tracker shows the same). Windows and Linux
   stay in the draft; re-run only the failed job (`…/actions/runs/<id>/rerun-failed-jobs`). The European evening and
   night worked: 0.2.4 got an Intel Mac at once at 22:37 UTC after a day of failures. Keep `macos-15-intel`: the
   Apple Silicon attempt never got a machine either.

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

## Known limitations

- **Unsigned:** macOS Gatekeeper and Windows SmartScreen warn on first launch. Signing needs an Apple Developer
  account and a Windows code-signing certificate.
- **Notifications:** new messages come from the bridge's own check (see above), up to 45 s late. Important
  mail isn't notified yet (reading the mailbox every minute is expensive); Marea's own reminders and calls
  arrive as before.
- **Passkeys** don't work inside the macOS webview (WKWebView needs associated-domain entitlements for each
  domain, which need signing, and every Marea has its own domain). Password + 2FA code works. Plan (Isacar,
  20 November 2026): Marea stops offering them inside the app unless `window.__MAREA_DESKTOP__.passkeys === true`
  (Marea 0.25.1; no app change needed, the app sets the flag only where they are proven to work); later, signing in
  through the system's sign-in window (ASWebAuthenticationSession), where passkeys work with any domain, and Marea
  hands the session to the app with a one-time code bound to a secret only the app holds (app 0.4, Marea 0.26).
  The details sent to Marea are in the roadmap's «App de escritorio» tab. WebKitGTK (Linux) has no passkeys at all;
  Windows is untested.
- **Screen sharing** in calls depends on each webview; untested.
- **Clicking a notification** brings the app forward; the wave opens because focus follows it within 30 s.

## Repository images

`docs/banner.png` (also the repository's social preview, 1280×640) and the `docs/screenshots/*-framed.png` images
were made from small HTML pages rendered with Playwright, using the Inter font from the Marea repository and
screenshots of the app connected to a demo Marea named "Estudio Paperclip". The window frame in them is a neutral
drawing, not a real title bar. Retake the screenshots when the app or Marea changes visibly.

The installer images were made the same way: `src-tauri/installer/dmg-background.png` (640×400, the Finder window
of the disk image; the waves start high because macOS may crop the bottom by the title bar's height) and
`src-tauri/installer/nsis-sidebar.bmp` (164×314, 24-bit BMP, the welcome and finish pages of the Windows
installer). They carry no words, so they work in every language. There is deliberately no license screen: the
AGPL doesn't need to be accepted to use the app (don't set `bundle.licenseFile`).
