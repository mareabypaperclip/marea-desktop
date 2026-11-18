// Bridge between your Marea and the desktop app. Loaded in every page before the page itself.
//
// - Tells Marea it runs inside the app (window.__MAREA_DESKTOP__), in case it ever wants to behave differently,
//   and lets it open one of its own addresses in the browser (openInBrowser, for linking Google and Microsoft).
// - Notifications Marea sends through the browser's standard API (Notification) become system notifications.
// - Links to other sites open in your browser, not inside the window.
// - On Windows and Linux, Ctrl+1 to Ctrl+7 open Marea's sections and Ctrl+, its settings (on macOS the menu bar
//   has these keys).
// - Notifies new messages while the window isn't in front: Marea itself only does it for reminders and calls,
//   because web push doesn't exist inside apps. Coming back to the window right after one opens that wave.
// - Puts the number of unread waves and mail on the app's icon (the Dock, the taskbar).
// - macOS and Windows: Marea's top bar reaches the top of the window, with room for the window buttons (on
//   Windows the bridge draws them, like Slack), and the window is dragged from it.
// - macOS: your location comes from the system (the web engine never gives it inside apps).
//
// It can only ask the app for a few harmless things: show a notification, open a link outside, put a number on
// the app's icon, moving the window (and on Windows minimizing, maximizing and closing it) and, on macOS, your
// location (behind the system's own prompt). No reading files or touching the computer: the app's permissions
// allow nothing more.
(function () {
  'use strict';
  if (window.__MAREA_DESKTOP__) return;

  var isLocal = location.protocol === 'tauri:' || location.hostname === 'tauri.localhost';
  var desktop = { version: '__VERSION__', platform: '__PLATFORM__' };
  // Marea's own page (not the connection screen, not inner frames) may also open one of its addresses in the browser
  if (!isLocal && window.top === window) desktop.openInBrowser = openInBrowser;
  Object.defineProperty(window, '__MAREA_DESKTOP__', {
    value: Object.freeze(desktop),
    enumerable: false,
    configurable: false,
    writable: false,
  });
  // Windows: the window's own buttons, here because the connection screen needs them too (see windowButtons)
  if ('__PLATFORM__' === 'windows' && window.top === window) windowButtons();
  if (isLocal || window.top !== window) return; // the connection screen and inner frames don't need the rest

  var BUTTONS_WIDTH = 138; // Windows: minimize, maximize and close, 46 px each
  var OAUTH = ['accounts.google.com', 'login.microsoftonline.com', 'login.microsoft.com', 'login.live.com', 'account.live.com'];

  function invoke(cmd, args) {
    try {
      return window.__TAURI_INTERNALS__.invoke(cmd, args || {});
    } catch (e) {
      return Promise.reject(e);
    }
  }

  // ── Opening one of Marea's own addresses in the browser (0.2.4) ──
  // For steps that can't happen inside an app, such as linking a Google account (Google refuses to sign in inside
  // embedded web views). Marea calls window.__MAREA_DESKTOP__.openInBrowser(url) only when it finds it, so older
  // versions of either side keep working (docs/browser-sign-in.md). Only addresses of the page's own origin: links
  // to other sites already go to the browser when clicked. The app's `open_external` command checks the address
  // again, and only your Marea's origin may call it.
  function openInBrowser(url) {
    var target;
    try {
      target = new URL(String(url), location.href);
    } catch (e) {
      return Promise.reject(new Error('Not a valid address'));
    }
    if (target.origin !== location.origin || (target.protocol !== 'https:' && target.protocol !== 'http:')) {
      return Promise.reject(new Error('Only addresses of this Marea open in the browser'));
    }
    return invoke('open_external', { url: target.href }).then(function () {});
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

  // ── The number on the app's icon ──
  // Unread waves plus unread mail: the same two numbers Marea shows next to «Waves» and «Correo» in its menu. While
  // the window is on screen they come from Marea itself (the waves from the page title, "(3) Marea"; the mail from
  // its menu, each time Marea counts it). While the window is hidden Marea stops asking its server, so the bridge
  // asks instead: the waves with the request it already makes for notifications, the mail every 5 minutes (as
  // often as Marea itself). Signed out, or on a page that isn't Marea, no number.
  var waiting = { waves: 0, mail: 0 };
  var onIcon = -1;
  function showWaiting() {
    var n = document.querySelector('.shell') ? waiting.waves + waiting.mail : 0;
    if (n === onIcon) return;
    onIcon = n;
    invoke('set_badge', { count: n }).catch(function () {
      onIcon = -1;
    });
  }
  function apiUrl(action) {
    return new URL('api.php?a=' + action, location.href).href;
  }
  function wavesFromTitle() {
    if (document.visibilityState !== 'visible') return; // hidden, Marea's title may be old: the bridge asks
    var m = /^\s*\((\d+)\+?\)/.exec(document.title);
    waiting.waves = m ? Number(m[1]) : 0;
    showWaiting();
  }
  function mailFromMenu() {
    var badge = document.querySelector('a[data-nav="mail"]:not(.sub) .badge');
    if (!badge) return;
    waiting.mail = badge.hidden ? 0 : parseInt(badge.textContent, 10) || 0; // "99+" counts as 99, like the menu
    showWaiting();
  }
  function wavesWhileHidden(waves) {
    if (document.visibilityState === 'visible') return;
    waiting.waves = waves.filter(function (w) {
      return w.unread && !w.trashed && !w.archived;
    }).length;
    showWaiting();
  }
  function mailWhileHidden() {
    if (document.visibilityState === 'visible' || !document.querySelector('.shell')) return;
    fetch(apiUrl('panel_mail'), { credentials: 'same-origin' })
      .then(function (r) {
        return r.ok ? r.json() : null;
      })
      .then(function (data) {
        if (!data || !data.set || !Array.isArray(data.boxes)) return;
        waiting.mail = data.boxes.reduce(function (sum, box) {
          return sum + (Number(box.unseen) || 0);
        }, 0);
        showWaiting();
      })
      .catch(function () {});
  }
  document.addEventListener('marea:mailcount', function () {
    setTimeout(mailFromMenu, 0); // Marea fires it just before painting its menu
  });
  var watchTitle = function () {
    wavesFromTitle();
    new MutationObserver(wavesFromTitle).observe(document.head, { childList: true, subtree: true, characterData: true });
  };
  if (document.head) watchTitle();
  else document.addEventListener('DOMContentLoaded', watchTitle);
  document.addEventListener('visibilitychange', function () {
    if (document.visibilityState !== 'visible') return;
    wavesFromTitle();
    mailFromMenu();
  });
  setInterval(mailWhileHidden, 300000);

  // ── New messages, as system notifications ──
  // Every 45 s, the same list of waves Marea's inbox shows. Only waves with something unread that changed since
  // the last look, never while you're looking at the window, and respecting each wave's own setting (muted,
  // mentions only, snoozed). The first look only takes note of what's there.
  var known = null; // wave id -> last "updated" seen
  var lastShown = null; // { hash, at } of the latest notification
  function lookForNews() {
    if (!document.querySelector('.shell')) return; // only in Marea itself, signed in
    fetch(apiUrl('inbox'), { credentials: 'same-origin' })
      .then(function (r) {
        return r.ok ? r.json() : null;
      })
      .then(function (data) {
        if (!data || !Array.isArray(data.waves)) return;
        wavesWhileHidden(data.waves);
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

  // ── macOS and Windows: the window's top bar is Marea's own ──
  // Styles go through the CSSOM (element.style): Marea's Content Security Policy rightly blocks <style> elements.
  // macOS keeps its window buttons at the top left (inside Marea's bar); on Windows the bridge draws them at the
  // top right (windowButtons). Either way the bar makes room for them and the window is dragged from it.
  if ('__PLATFORM__' === 'macos' || '__PLATFORM__' === 'windows') {
    var mac = '__PLATFORM__' === 'macos';
    var strip = document.createElement('div');
    strip.id = 'marea-desktop-drag';
    strip.setAttribute('data-tauri-drag-region', '');
    css(strip, [['position', 'fixed'], ['top', '0'], ['left', '0'], ['right', mac ? '0' : BUTTONS_WIDTH + 'px'],
      ['height', mac ? '28px' : '32px'], ['z-index', '2147483646']]);
    var fit = function () {
      var top = document.querySelector('header.top');
      if (top && top.getAttribute('data-tauri-drag-region') !== 'deep') {
        top.setAttribute('data-tauri-drag-region', 'deep'); // empty parts of the bar drag; its buttons still work
        // room for the window buttons
        top.style.setProperty(mac ? 'padding-left' : 'padding-right', (mac ? 86 : BUTTONS_WIDTH + 8) + 'px', 'important');
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
  }

  if ('__PLATFORM__' === 'macos') {
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

  // ── Helpers ──
  function css(element, rules) {
    rules.forEach(function (rule) {
      element.style.setProperty(rule[0], rule[1]);
    });
  }

  // ── Windows: minimize, maximize and close, drawn by the app ──
  // The window has no Windows frame (like Slack): Marea reaches the top and these three buttons sit at the top
  // right, with Windows' own sizes, symbols and colours (red to close). The symbols are drawn (SVG), not taken
  // from Windows' icon font, so they look the same on Windows 10 and 11. Closing keeps Marea next to the clock, as
  // always. They take the colour of Marea's top bar, or a dark one where there's no bar. The keyboard keeps
  // Windows' own keys (Win+↑, Win+↓, Alt+F4), so they stay out of the page's tab order, like Windows' own.
  function windowButtons() {
    var LABELS = "__LABELS__";
    var SHAPES = {
      minimize: [['path', { d: 'M0 5.5H10' }]],
      maximize: [['rect', { x: '0.5', y: '0.5', width: '9', height: '9' }]],
      restore: [['rect', { x: '0.5', y: '2.5', width: '7', height: '7' }], ['path', { d: 'M2.5 2.5V0.5H9.5V7.5H7.5' }]],
      close: [['path', { d: 'M0.5 0.5L9.5 9.5M9.5 0.5L0.5 9.5' }]],
    };
    var symbol = function (name) {
      var NS = 'http://www.w3.org/2000/svg';
      var svg = document.createElementNS(NS, 'svg');
      [['width', '10'], ['height', '10'], ['viewBox', '0 0 10 10'], ['fill', 'none'], ['stroke', 'currentColor'],
        ['stroke-width', '1'], ['aria-hidden', 'true']].forEach(function (a) {
        svg.setAttribute(a[0], a[1]);
      });
      SHAPES[name].forEach(function (shape) {
        var el = document.createElementNS(NS, shape[0]);
        Object.keys(shape[1]).forEach(function (k) {
          el.setAttribute(k, shape[1][k]);
        });
        svg.appendChild(el);
      });
      return svg;
    };
    var label = 'main';
    try {
      label = window.__TAURI_INTERNALS__.metadata.currentWindow.label || label;
    } catch (e) {}
    var call = function (command) {
      return invoke('plugin:window|' + command, { label: label });
    };
    var box = document.createElement('div');
    box.id = 'marea-desktop-buttons';
    css(box, [['position', 'fixed'], ['top', '0'], ['right', '0'], ['height', '32px'], ['display', 'flex'],
      ['z-index', '2147483647'], ['color', '#1f1f1f']]);
    var onDark = false;
    var button = function (name, action) {
      var b = document.createElement('button');
      b.type = 'button';
      b.tabIndex = -1;
      b.appendChild(symbol(name));
      b.title = LABELS[name];
      b.setAttribute('aria-label', LABELS[name]);
      css(b, [['width', '46px'], ['height', '32px'], ['margin', '0'], ['padding', '0'], ['border', '0'],
        ['border-radius', '0'], ['background', 'transparent'], ['color', 'inherit'], ['cursor', 'default'],
        ['display', 'flex'], ['align-items', 'center'], ['justify-content', 'center'], ['outline', 'none']]);
      b.addEventListener('mouseenter', function () {
        var red = b.getAttribute('data-name') === 'close';
        b.style.setProperty('background', red ? '#c42b1c' : onDark ? 'rgba(255,255,255,0.16)' : 'rgba(0,0,0,0.06)');
        if (red) b.style.setProperty('color', '#fff');
      });
      b.addEventListener('mouseleave', function () {
        b.style.setProperty('background', 'transparent');
        b.style.setProperty('color', 'inherit');
      });
      b.addEventListener('click', action);
      b.setAttribute('data-name', name);
      box.appendChild(b);
      return b;
    };
    var max;
    var state = function () {
      call('is_maximized')
        .then(function (on) {
          var name = on ? 'restore' : 'maximize';
          max.replaceChildren(symbol(name));
          max.title = LABELS[name];
          max.setAttribute('aria-label', LABELS[name]);
        })
        .catch(function () {});
    };
    button('minimize', function () {
      call('minimize').catch(function () {});
    });
    max = button('maximize', function () {
      call('toggle_maximize').then(state).catch(function () {});
    });
    button('close', function () {
      call('close').catch(function () {});
    });
    var paint = function () {
      var top = document.querySelector('header.top');
      var color = top ? getComputedStyle(top).color : '#1f1f1f';
      box.style.setProperty('color', color);
      var rgb = (color.match(/\d+(\.\d+)?/g) || ['31', '31', '31']).map(Number);
      onDark = 0.2126 * rgb[0] + 0.7152 * rgb[1] + 0.0722 * rgb[2] > 140; // light symbols: a dark bar behind
      if (document.body && !box.isConnected) document.body.appendChild(box);
    };
    var resized = 0;
    window.addEventListener('resize', function () {
      clearTimeout(resized);
      resized = setTimeout(state, 150);
    });
    var start = function () {
      paint();
      state();
      var queued = false;
      new MutationObserver(function () {
        if (queued) return;
        queued = true;
        requestAnimationFrame(function () {
          queued = false;
          paint();
        });
      }).observe(document.body, { childList: true, subtree: true });
    };
    if (document.readyState === 'loading') document.addEventListener('DOMContentLoaded', start);
    else start();
  }
})();
