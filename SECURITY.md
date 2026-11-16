# Security policy

## Supported versions

Only the [latest release](https://github.com/mareabypaperclip/marea-desktop/releases/latest) gets security fixes.
The app tells you when a new one is out.

## Reporting a vulnerability

Please **don't open a public issue**. Report it privately through GitHub instead:
[Security → Report a vulnerability](https://github.com/mareabypaperclip/marea-desktop/security/advisories/new).

Include what you found, how to reproduce it and which system and app version you used. You'll get an answer within
a week; once a fix is out, you'll be credited in the advisory unless you'd rather not be.

Problems in Marea itself (the server and the web app) belong in the
[Marea repository](https://github.com/mareabypaperclip/marea/security/advisories/new).

## What the app is designed to guarantee

These are the boundaries we treat as security bugs if they break:

- The app only loads the Marea chosen by the user, over https (plain http only for `localhost`).
- Pages from that Marea can call only a few app functions: show a notification, open a link in the system
  browser and, on macOS, get the location (after the system's own permission prompt, at kilometre accuracy) and
  move or zoom the window. No other site can call any.
- Going to another site opens it in the system browser; the app window stays on the chosen Marea (the Google and
  Microsoft sign-in pages excepted).
- Downloads are saved only inside the user's Downloads folder, with cleaned-up file names.
- The app sends nothing anywhere except to the chosen Marea and, to check for updates, to GitHub's public API.
