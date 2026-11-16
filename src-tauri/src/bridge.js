// Bridge between your Marea and the desktop app. Loaded in every page before the page itself.
//
// - Tells Marea it runs inside the app (window.__MAREA_DESKTOP__), in case it ever wants to behave differently.
// - Notifications Marea sends through the browser's standard API (Notification) become system notifications.
// - Links to other sites open in your browser, not inside the window.
// - On Windows and Linux, Ctrl+1 to Ctrl+7 open Marea's sections and Ctrl+, its settings (on macOS the menu bar
//   has these keys).
//
// It can only ask the app for two things: "show this notification" and "open this link outside". No reading files
// or touching the computer: the app's permissions allow nothing more.
(function () {
  'use strict';
  if (window.__MAREA_DESKTOP__) return;

  var isLocal = location.protocol === 'tauri:' || location.hostname === 'tauri.localhost';
  Object.defineProperty(window, '__MAREA_DESKTOP__', {
    value: Object.freeze({ version: '__VERSION__', platform: '__PLATFORM__' }),
    enumerable: false,
    configurable: false,
    writable: false,
  });
  if (isLocal || window.top !== window) return; // the connection screen and inner frames don't need it

  var OAUTH = ['accounts.google.com', 'login.microsoftonline.com', 'login.microsoft.com', 'login.live.com', 'account.live.com'];

  function invoke(cmd, args) {
    try {
      return window.__TAURI_INTERNALS__.invoke(cmd, args || {});
    } catch (e) {
      return Promise.reject(e);
    }
  }

  // ── System notifications ──
  function DesktopNotification(title, options) {
    var o = options || {};
    this.title = String(title == null ? '' : title);
    this.body = String(o.body == null ? '' : o.body);
    this.tag = String(o.tag == null ? '' : o.tag);
    this.data = o.data;
    this.onclick = null;
    this.onclose = null;
    this.onshow = null;
    this.onerror = null;
    invoke('notify', { title: this.title.slice(0, 200), body: this.body.slice(0, 1000) }).catch(function () {});
  }
  DesktopNotification.prototype.close = function () {};
  DesktopNotification.prototype.addEventListener = function () {};
  DesktopNotification.prototype.removeEventListener = function () {};
  DesktopNotification.prototype.dispatchEvent = function () { return false; };
  Object.defineProperty(DesktopNotification, 'permission', { get: function () { return 'granted'; } });
  DesktopNotification.requestPermission = function (callback) {
    if (typeof callback === 'function') callback('granted');
    return Promise.resolve('granted');
  };
  DesktopNotification.maxActions = 0;
  try {
    Object.defineProperty(window, 'Notification', { value: DesktopNotification, configurable: true, writable: true });
  } catch (e) {
    window.Notification = DesktopNotification;
  }

  // ── Links to other sites, in the browser ──
  function external(href) {
    var url;
    try {
      url = new URL(href, location.href);
    } catch (e) {
      return null;
    }
    if (url.protocol !== 'http:' && url.protocol !== 'https:') return null;
    if (url.origin === location.origin) return null;
    if (OAUTH.indexOf(url.hostname) !== -1) return null;
    return url.href;
  }
  document.addEventListener(
    'click',
    function (ev) {
      if (ev.defaultPrevented || ev.button !== 0) return;
      var a = ev.target && ev.target.closest ? ev.target.closest('a[href]') : null;
      if (!a || a.hasAttribute('download')) return;
      var href = external(a.getAttribute('href'));
      if (!href) return;
      ev.preventDefault();
      invoke('open_external', { url: href }).catch(function () {});
    },
    true
  );

  // ── Section keys on Windows and Linux (same order as SECTIONS in actions.rs) ──
  if ('__PLATFORM__' !== 'macos') {
    var SECTIONS = ['#/', '#/mail', '#/agenda', '#/people', '#/drop', '#/podcasts', '#/notas'];
    document.addEventListener(
      'keydown',
      function (ev) {
        if (!ev.ctrlKey || ev.altKey || ev.shiftKey || ev.metaKey) return;
        var digit = /^Digit([1-7])$/.exec(ev.code || '');
        var hash = digit ? SECTIONS[Number(digit[1]) - 1] : ev.key === ',' ? '#/settings' : null;
        if (!hash) return;
        ev.preventDefault();
        location.hash = hash;
      },
      true
    );
  }
})();
