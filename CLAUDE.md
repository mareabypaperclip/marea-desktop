# Marea for desktop — instructions for agents

The desktop app of [Marea](https://github.com/mareabypaperclip/marea) (Tauri 2: macOS, Windows, Linux). Owner:
Isacar (Paperclip). Read `NOTES.md` before changing anything.

Claude Opus, in the app's own chat, takes care of everything about the app, end to end: code, releases, its
roadmap to 1.0 and what it needs from Marea (Isacar, 18 November 2026). Marea itself is made by Claude Fable in its
own sessions: ask Marea for things through Isacar, with a message ready to send.

## Talking to Isacar

- **Always in Spanish**, short sentences: he often reads on his phone. No flattery; be honest about what is untested.
- Don't wait for approval between steps on work he has asked for. Ask before publishing a release only if he
  hasn't asked for it.

## Rules

- **Code, comments, commit messages and docs in English.** User-facing text never goes in code: it goes in
  `ui/locales/{en,es,fr,ca}.json` (all four, same keys and placeholders: `cargo test` checks it).
- His design preferences: buttons with one style, right-aligned in forms; nothing duplicated on screen; no logos
  or icons assumed (Marea's own logo is fine; a company's logo must be asked for).
- **Security first:** the remote Marea only gets `allow-notify`, `allow-open-external` and `allow-set-badge` (plus
  the macOS and Windows extras listed in `NOTES.md`); never widen that without a clear reason written in `NOTES.md`. Never put
  keys or passwords in the repository or in the chat.
  Note the security impact of each change in the release notes.
- Before committing: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and `cargo test` (in
  `src-tauri/`), and look at the app running (see *Testing locally* in `NOTES.md`), with screenshots for Isacar.
- Every release: same version number in the three files, tag `vX.Y.Z`, and tell Isacar what changed, what is
  untested on real machines and the security notes.
- The app has its own roadmap: the «App de escritorio» tab of the Marea roadmap (a Claude doc, linked from the main
  repository's `CLAUDE.md`; Isacar gives you the link). Keep it in sync when the app's plans change, and read the
  part you change first. The other tabs and sections are Marea's: touch them only for what the app needs from
  Marea. The app also has its own chat with Isacar, apart from Marea's.
