# Contributing

Thanks for wanting to help. Marea for desktop is small on purpose: a secure window onto a Marea server, with the
few things a browser tab can't do. Changes that keep it that way are the easiest to accept.

## Translations

The most welcome contribution. Every text the app shows lives in `ui/locales/<language>.json`, shared by the app
and its connection screen.

1. Copy `ui/locales/en.json` to `ui/locales/<code>.json` (for example `de.json`) and translate the values. Keep
   the keys, and keep placeholders such as `{host}` or `{seconds}` exactly as they are.
2. Register it in `src-tauri/src/i18n.rs`: add the file to `catalogs()`, the code to `pick()` and to the
   `LANGUAGES` list the tests use.
3. For macOS, add `src-tauri/macos/<code>.lproj/InfoPlist.strings` (camera and microphone texts) and list it in
   `src-tauri/tauri.conf.json` and `src-tauri/Info.plist`.
4. Run `cargo test` in `src-tauri/`: it fails if a key or a placeholder is missing.

Write the way people speak, short and friendly; the existing translations show the tone.

## Bugs and ideas

Open an [issue](https://github.com/mareabypaperclip/marea-desktop/issues/new/choose). If the problem also happens
in a browser, it belongs to [Marea](https://github.com/mareabypaperclip/marea/issues) instead.

## Code

- Code, comments and commit messages in English. Text for users never goes in code: it goes in the locale files,
  in all languages.
- Read [NOTES.md](NOTES.md) first: it explains how the app is put together and why.
- **Security first.** Don't widen what the remote Marea can call (`allow-notify` and `allow-open-external`) without a
  strong reason explained in `NOTES.md` and in the pull request.
- Before opening a pull request, in `src-tauri/`:

  ```sh
  cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test
  ```

  and try the app on at least one system (`npx tauri build --debug --no-bundle`), with a screenshot if anything
  visible changed.

By contributing you agree that your work is released under the [AGPL-3.0-or-later](LICENSE) license.
