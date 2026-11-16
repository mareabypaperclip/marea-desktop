// Bridge between your Marea and the desktop app. Loaded in every page before the page itself.
//
// - Tells Marea it runs inside the app (window.__MAREA_DESKTOP__), in case it ever wants to behave differently.
// - Notifications Marea sends through the browser's standard API (Notification) become system notifications.
// - Links to other sites open in your browser, not inside the window.
// - On Windows and Linux, Ctrl+1 to Ctrl+7 open Marea's sections and Ctrl+, its settings (on macOS the menu bar
//   has these keys).
// - Notifies new messages while the window isn't in front: Marea itself only does it for reminders and calls,
//   because web push doesn't exist inside apps. Coming back to the window right after one opens that wave.
// - macOS: Marea's top bar reaches the top of the window, with room for the window buttons, and the window is
//   dragged from it; and your location comes from the system (the web engine never gives it inside apps).
//
// It can only ask the app for a few harmless things: show a notification, open a link outside and, on macOS,
// your location (behind the system's own prompt) and moving the window. No reading files or touching the
// computer: the app's permissions allow nothing more.
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

  // ── New messages, as system notifications ──
  // Every 45 s, the same list of waves Marea's inbox shows. Only waves with something unread that changed since
  // the last look, never while you're looking at the window, and respecting each wave's own setting (muted,
  // mentions only, snoozed). The first look only takes note of what's there.
  var known = null; // wave id -> last "updated" seen
  var lastShown = null; // { hash, at } of the latest notification
  function inboxUrl() {
    return new URL('api.php?a=inbox', location.href).href;
  }
  function lookForNews() {
    if (!document.querySelector('.shell')) return; // only in Marea itself, signed in
    fetch(inboxUrl(), { credentials: 'same-origin' })
      .then(function (r) {
        return r.ok ? r.json() : null;
      })
      .then(function (data) {
        if (!data || !Array.isArray(data.waves)) return;
        var first = known === null;
        if (first) known = {};
        var looking = document.visibilityState === 'visible' && document.hasFocus();
        data.waves.forEach(function (w) {
          var id = String(w.id);
          var updated = Number(w.updated) || 0;
          var before = known[id] || 0;
          known[id] = Math.max(before, updated);
          if (first || looking || !w.unread || updated <= before || w.trashed || w.archived) return;
          if (w.notify === 'mute' || (w.notify === 'mentions' && !w.mention) || Number(w.snooze) > 0) return;
          var title = (w.mention ? '@ ' : '') + String(w.title || 'Marea');
          invoke('notify', { title: title.slice(0, 200), body: String(w.snippet || '').slice(0, 1000) }).catch(function () {});
          lastShown = { hash: '#/w/' + encodeURIComponent(id), at: Date.now() };
        });
      })
      .catch(function () {});
  }
  setTimeout(lookForNews, 5000);
  setInterval(lookForNews, 45000);
  // Clicking a notification brings the app forward: if that happens soon after one, open its wave.
  window.addEventListener('focus', function () {
    if (lastShown && Date.now() - lastShown.at < 30000 && location.hash !== lastShown.hash) {
      location.hash = lastShown.hash;
    }
    lastShown = null;
  });

  // ── macOS: the window's top bar is Marea's own ──
  // Styles go through the CSSOM (element.style): Marea's Content Security Policy rightly blocks <style> elements.
  if ('__PLATFORM__' === 'macos') {
    var strip = document.createElement('div');
    strip.id = 'marea-desktop-drag';
    strip.setAttribute('data-tauri-drag-region', '');
    [['position', 'fixed'], ['top', '0'], ['left', '0'], ['right', '0'], ['height', '28px'], ['z-index', '2147483646']]
      .forEach(function (rule) {
        strip.style.setProperty(rule[0], rule[1]);
      });
    var fit = function () {
      var top = document.querySelector('header.top');
      if (top && top.getAttribute('data-tauri-drag-region') !== 'deep') {
        top.setAttribute('data-tauri-drag-region', 'deep'); // empty parts of the bar drag; its buttons still work
        top.style.setProperty('padding-left', '86px', 'important'); // room for the window buttons
      }
      strip.style.setProperty('display', top ? 'none' : 'block'); // pages without the bar: a thin strip instead
      if (document.body && !strip.isConnected) document.body.appendChild(strip);
    };
    var ready = function () {
      fit();
      new MutationObserver(fit).observe(document.body, { childList: true, subtree: true });
    };
    if (document.readyState === 'loading') document.addEventListener('DOMContentLoaded', ready);
    else ready();

    // ── macOS: your location, from the system ──
    var geolocation = {
      getCurrentPosition: function (success, failure) {
        invoke('locate').then(
          function (p) {
            if (typeof success !== 'function') return;
            success({
              coords: {
                latitude: p.latitude,
                longitude: p.longitude,
                accuracy: p.accuracy,
                altitude: null,
                altitudeAccuracy: null,
                heading: null,
                speed: null,
              },
              timestamp: Date.now(),
            });
          },
          function (why) {
            if (typeof failure !== 'function') return;
            var code = why === 'denied' ? 1 : why === 'timeout' ? 3 : 2;
            failure({ code: code, message: String(why), PERMISSION_DENIED: 1, POSITION_UNAVAILABLE: 2, TIMEOUT: 3 });
          }
        );
      },
      watchPosition: function (success, failure) {
        geolocation.getCurrentPosition(success, failure);
        return 1;
      },
      clearWatch: function () {},
    };
    try {
      Object.defineProperty(navigator, 'geolocation', { value: geolocation, configurable: true });
    } catch (e) {
      // keep the engine's own
    }
  }

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
