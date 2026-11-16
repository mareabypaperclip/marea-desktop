# Changes

## 0.2.0

- **Notifications for new messages.** When a wave you follow gets something new and the window isn't in front,
  a system notification shows its title and the latest text (mentions start with @). Muted waves, "mentions
  only" and snoozed waves are respected. Coming back to the window right after one opens that wave.
- **macOS: no more grey title bar.** Marea's top bar reaches the top of the window, with the window buttons
  inside it, and you drag the window from it.
- **macOS: your location works** (for the weather): the app asks the system, which shows its own permission
  prompt the first time.
- **macOS: camera and microphone allowed in calls.** The app now declares them, as the hardened runtime requires.
- **Open at Login**, in the Marea menu on macOS and the menu next to the clock on Windows and Linux. Opened that
  way, it starts with the window hidden, ready for notifications.
- **Print** (⌘P) in the File menu on macOS.

## 0.1.1

- Menus with Marea's own shortcuts. On macOS: *New Wave* (⌘N), *Compose Mail* (⇧⌘N), *Search* (⌘F), *Assistant*,
  *Settings…* (⌘,), a *Go* menu with every section (⌘1 to ⌘7, plus back and forward) and *Keyboard Shortcuts* in
  Help. On Windows and Linux: the same sections with Ctrl+1 to Ctrl+7, settings with Ctrl+, and new wave, compose,
  mail, calendar and settings in the menu of the icon next to the clock.
- The macOS disk image no longer asks you to accept the license before opening. The AGPL doesn't need to be
  accepted to use the app, and the Windows installer skips that page too.
- A proper disk image on macOS: Marea's own background with an arrow towards Applications.
- The Windows installer shows Marea on its welcome and finish pages.
- The Linux `.deb` package no longer lists the same dependency twice.

## 0.1.0

First release: Marea in its own window on macOS, Windows and Linux, with system notifications, the pending count
on the icon, downloads, external links in the browser, an offline screen that reconnects on its own, update
notices and four languages (English, Spanish, French and Catalan).
