# Changes

## 0.4.0

- **New mail, as system notifications.** While the window isn't in front, new mail in your inboxes arrives as a
  system notification (at most 3 a minute, the newest first); coming back to the window right after one opens that
  email. Marea decides what counts: not what you've read elsewhere, nor blocked senders, nor anything while «No
  molestar» or «solo menciones» is on. Needs Marea 0.25.1 or later; with an older Marea nothing changes.
- **The system's own look.** On macOS, the sidebar material of Mail or Finder behind Marea's sidebar; on Windows
  11, Mica. Windows 10 and Linux stay as they were. Needs Marea 0.25.1 or later, which leaves its sidebar
  transparent only when the window has that material behind it.
- **«About Marea» on Windows and Linux too** (in the menu of the icon next to the clock), and on every system it
  now says that Marea is free software under the AGPL-3.0 licence, with no warranty.

**Security:** no new permission. The new-mail check uses your Marea's own session, from Marea's own page, once a
minute; it only reads who sent each new email and its subject, and marks nothing as read. The window is now
transparent behind the page on macOS and Windows 11 (pages paint over it as before).

## 0.2.4

- **Ready to link Google and Microsoft in your browser.** From Marea 0.25, «Enlazar con Google» and «Enlazar con
  Microsoft» will open in your usual browser, as in Slack, instead of inside the app, where Google refuses to sign
  in. This version brings the app's part; with an older Marea nothing changes.

**Security:** your Marea may now ask the app to open one of its own addresses in your browser, with the permission
it already had to open links outside. The app refuses any other site, and inner frames (emails, videos) can't ask.

## 0.2.3

- **Windows: Marea from top to bottom, like Slack.** The window no longer has the white Windows title bar: Marea's
  own top bar reaches the top of the window, in your Marea's colour, with minimize, maximize and close at the top
  right. The window is dragged from that bar, and double-clicking it maximizes. On the Google and Microsoft
  sign-in pages the usual Windows bar comes back, because those pages can't show our buttons.

**Security:** on Windows, the Marea you connect to may also minimize, maximize and close the app's window (the
same as its buttons do). Nothing else changes.

## 0.2.2

- **Checking for updates works on any connection.** The app now finds the latest version through the address
  GitHub's own website uses, which has no request limit. Before, it only asked GitHub's API, which answers
  "403" when a connection has asked too often, and the app said it couldn't check.

**Security:** nothing changes in what the app or your Marea may do. The app still only asks GitHub, over https,
which version is the latest.

## 0.2.1

- **Unread mail on the icon too.** The number on the Dock icon (and the taskbar button on Windows) now adds up
  unread waves and unread mail, the same two numbers next to «Waves» and «Correo» in Marea's menu. Before, it
  only counted waves, so with only unread mail there was no number at all. It keeps up while the window is hidden.
- **Drag and drop works again.** Moving mail onto a folder, dropping files into an email or a transfer and every
  other drag inside Marea now reach Marea. On macOS and Windows the app was catching every drag over the window.

**Security:** the Marea you connect to gets one more permission, *set the number on the app's icon*, which can
only show a number. Drag and drop goes to the Marea page, as in a browser; the app itself never receives files.

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
