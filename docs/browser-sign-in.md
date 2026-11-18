# Linking Google and Microsoft in the browser

What Marea and the desktop app each do so that «Enlazar con Google» and «Enlazar con Microsoft» open in the
person's own browser, as in Slack. Proposal of 18 November 2026: Marea's part goes in Marea 0.25, the app's part
in the app version after 0.2.3. The app's part can ship first: Marea only uses it when it finds it.

## Why

- Google refuses to sign in inside apps that embed a web view (error `disallowed_useragent`). Microsoft allows it,
  but the browser is better there too: saved passwords, passkeys and sessions the person already has.
- Today `link()` in Marea's `assets/js/mail-connect.js` sets `location.href` to Google's or Microsoft's page. The
  `state` of that step lives in the PHP session (`$_SESSION['google_state']`, `$_SESSION['ms_state']`), and the
  callbacks (`do_google_callback`, `do_ms_callback` in `app/ApiAgenda.php`) check it there and take the person
  from the session (`$this->user()`).
- The browser has its own cookies, so it has no Marea session. Opening today's Google address in the browser
  would end at the callback with «La conexión con Google ha caducado».

## The flow

1. In the app, the person presses «Enlazar con Google» (or Microsoft) in *Ajustes → Mi correo* or in the tour.
2. Marea sees `window.__MAREA_DESKTOP__.openInBrowser` and, instead of navigating, asks its server for a
   *handoff*: a one-time link tied to that person and that provider. It answers a token.
3. Marea builds the link from its own address (`location.origin` plus its path, for example
   `…/#/enlazar/<token>`) and calls `openInBrowser(link)`. The app opens it in the default browser. Marea shows a
   small dialog: «Sigue en tu navegador». Closing it cancels the handoff. If `openInBrowser` fails (no browser
   set, for example), Marea cancels the handoff and says so.
4. In the browser, a page of Marea that works without a session (like the public forms, `#/f/<token>`) says what
   is about to happen: «Vas a enlazar Google con la cuenta de Marea de **Ana López** (a•••@example.com) en
   **marea.example.com**», and one button, «Seguir con Google». Just opening the page uses nothing up.
5. The button sends a POST with the token. The server uses the token up, makes the `state`, sets a short cookie in
   that browser (see *Rules*) and sends it to Google or Microsoft with the same `login_hint` as today.
6. Google or Microsoft come back to the same callback as today (`api.php?a=google_callback` or `ms_callback`), so
   nothing changes in Google Cloud or in Azure. The callback finds the handoff by its `state`, checks the cookie,
   takes the person from the handoff (not from a session), saves the mailbox as today and marks the handoff done
   or failed, with the message. The browser ends on a page that says «Listo: vuelve a Marea. Puedes cerrar esta
   pestaña» (or the error). No Marea session is opened in the browser.
7. Meanwhile Marea, in the app, asks every 3 seconds while its dialog is open, and at once when the window gets
   the focus back, how the handoff is going. When it is done, it closes the dialog and does what it does today on
   return (`#/settings/micorreo/g-ok`): the toast «Cuenta de Google enlazada» and *Mi correo* reloaded. If the
   window isn't in front, it also sends a `Notification`, which the app turns into a system notification;
   clicking it brings the app forward.

## Marea's part (0.25)

The names are suggestions; only `openInBrowser` is a contract with the app.

- **A table of handoffs**, for example `link_handoffs`: person, provider (`google` or `microsoft`), the `login_hint`,
  SHA-256 of the token, of the `state` and of the cookie (never the values), created, expires, used, and the result
  (`waiting`, `done`, `failed`, `cancelled`, `expired`) with its message and the linked address.
- **Actions:**
  - `link_handoff` (POST, with session, like the other POSTs): creates the handoff and returns `{ id, token }`.
  - `link_handoff_info` (GET, no session, by token): provider, the server's host, the person's name, the masked
    address and the expiry, for the page in the browser. It never uses the token up.
  - `link_handoff_go` (POST, no session, by token): uses the token up, sets the cookie and sends the browser to
    Google or Microsoft (`authUrl()` of `app/Google.php` or `app/Microsoft.php`, unchanged).
  - `link_handoff_status` (GET, with session, only the owner): `{ state, message, email }`.
  - `link_handoff_cancel` (POST, with session, only the owner).
  - `do_google_callback` and `do_ms_callback`: first today's check against the session; if it doesn't match, the
    handoff. They must not call `$this->user()` before knowing which of the two it is.
- **In `link()`:** with `typeof window.__MAREA_DESKTOP__?.openInBrowser === 'function'`, the new flow; without it
  (the browser, the installed web app, app versions up to 0.2.3), today's, untouched.
- **The audit log:** «enlazó su cuenta de Google (…) desde el navegador».

## Rules

- **The token:** 32 random bytes, in base64url. It lasts 10 minutes and is used once. A new handoff for the same
  person and provider cancels the one before. At most 10 handoffs per person and hour.
- **The confirmation page** stops someone sending you a link that would join your Gmail to *their* Marea: it says
  whose account it is before anything happens, and nothing happens without the button.
- **The cookie** stops the same trick done one step later (someone presses «Seguir» and sends you Google's
  address). It is random, `HttpOnly`, `Secure` on https, `SameSite=Lax` (it has to travel on the way back from
  Google), limited to Marea's path and gone in 10 minutes. The callback refuses a handoff whose cookie doesn't
  match, and deletes it when done.
- **Only POST uses anything up**, so link previews and scanners that open links don't break it.
- **The masked address** (first letter and domain) is enough to recognize your account and gives little to someone
  who finds the link.
- **Nothing new for the admin:** same callback addresses, same client ids and secrets.

## The app's part

- **`window.__MAREA_DESKTOP__.openInBrowser(url)`**, in the bridge, returns a promise. Only in the top window (not
  in the frames of emails or videos). It only accepts addresses on the page's own origin (https, or
  `http://localhost…` for testing); anything else is refused. It calls the `open_external` command the page
  already has: **no new permission**.
- The Google and Microsoft pages keep opening inside the window (`OAUTH_HOSTS` in `links.rs`), because Mareas
  before 0.25 still link that way.
- `tools/check-bridge.mjs`: the function exists only in the top window, refuses other origins and calls
  `open_external` with the address.
- Marea's brief lists what it must not break in the app: `openInBrowser` is added to that list.

## On purpose, not done

- **No `marea://` link** to come back to the app at the end, as Slack does: any website could open the app with
  it. The notification and the focus are enough; it can be revisited.
- **The app reads nothing from the browser**, and the browser gets no Marea session.

## Tests for Marea

- With `openInBrowser` replaced by a function that keeps the address: the link is on Marea's origin, the dialog
  shows, and closing it cancels the handoff.
- In a new browser context without a session: the page shows the host, the name and the masked address; reloading
  it uses nothing up; «Seguir» goes to the provider with `state` and hint (the mock provider of today's tests, as
  in `dev/v0211/microsoft.mjs`).
- The callback with the right cookie links the mailbox to the person of the handoff (not to whoever is signed in
  in that browser); without the cookie, with another one, used twice or after 10 minutes, it refuses with a clear
  message and links nothing.
- Status: `waiting`, then `done` with the address; the toast and *Mi correo* reloaded; a `Notification` when the
  window isn't focused.
- A second handoff cancels the first; the 11th in an hour is refused.
- Without `openInBrowser`, today's flow, unchanged (today's linking tests keep passing as they are).
