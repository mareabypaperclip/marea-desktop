// Runs the app's bridge (as macOS) inside the test Marea with a fake app behind it, and checks the new parts:
// notifications of new messages, opening that wave on focus, the title bar room and drag region, location, the
// number on the app's icon (unread waves and mail, with the window on screen and hidden) and openInBrowser; then,
// as Windows, the window buttons the bridge draws.
//
// Needs the Marea repository next to this one (or MAREA_REPO) with its test install running, and Playwright.
// It changes wave 4 of the test install (marks it unread) and puts it back. Run: node tools/check-bridge.mjs
import fs from 'node:fs';
import path from 'node:path';
import { execSync } from 'node:child_process';
import { fileURLToPath, pathToFileURL } from 'node:url';

const here = path.dirname(fileURLToPath(import.meta.url));
const marea = process.env.MAREA_REPO || path.join(here, '..', '..', 'marea');
const { B, ok } = await import(pathToFileURL(path.join(marea, 'dev', 'devlib.mjs')).href);
const { chromium } = await import(process.env.PLAYWRIGHT || '/opt/npm-tools/node_modules/playwright/index.mjs');

const bridge = fs.readFileSync(path.join(here, '..', 'src-tauri', 'src', 'bridge.js'), 'utf8')
  .replace(/__PLATFORM__/g, 'macos').replace(/__VERSION__/g, '0.2.0')
  .replace('"__LABELS__"', JSON.stringify({ minimize: 'Minimizar', maximize: 'Maximizar', restore: 'Restaurar', close: 'Cerrar' }))
  .replace('setInterval(lookForNews, 45000)', 'setInterval(lookForNews, 2000)')
  .replace('setTimeout(lookForNews, 5000)', 'setTimeout(lookForNews, 1500)')
  .replace('setInterval(mailWhileHidden, 300000)', 'setInterval(mailWhileHidden, 2000)')
  .replace(/__VIBRANCY__/g, 'yes')
  .replace('setTimeout(lookForMail, 8000)', 'setTimeout(lookForMail, 1000)')
  .replace('setInterval(lookForMail, 60000)', 'setInterval(lookForMail, 1500)');
const fake = `window.__calls = []; window.__fetched = [];
{ const f = window.fetch.bind(window); window.fetch = (u, o) => { window.__fetched.push(String(u)); return f(u, o); }; }
window.__TAURI_INTERNALS__ = { invoke: (cmd, args) => { window.__calls.push([cmd, args]);
  if (cmd === 'locate') return Promise.resolve({ latitude: 39.4699, longitude: -0.3763, accuracy: 1000 });
  return Promise.resolve(null); } };`;

const browser = await chromium.launch({ executablePath: '/usr/bin/google-chrome', args: ['--no-sandbox'] });
const ctx = await browser.newContext({ viewport: { width: 1280, height: 800 }, locale: 'es-ES' });
await ctx.addInitScript(fake + '\n' + bridge);
// Marea 0.25.1's `mail_new`, answered by the test: every call is kept, and the answer is whatever mailReply says
const mailCalls = [];
let mailReply = { status: 200, body: { mark: 'm1', items: [], quiet: '' } };
await ctx.route('**/api.php?a=mail_new*', (route) => {
  mailCalls.push(route.request().url());
  route.fulfill({ status: mailReply.status, contentType: 'application/json', body: JSON.stringify(mailReply.body) });
});
const a = await ctx.newPage();
const errors = []; a.on('pageerror', (e) => errors.push(e.message));
await a.goto(B);
await a.fill('input[type=email]', 'ana@example.com'); await a.fill('input[type=password]', 'supersecreta1');
await a.click('button.primary:has-text("Entrar")'); await a.waitForSelector('.shell');

// Title bar
await a.waitForTimeout(500);
const top = await a.evaluate(() => { const t = document.querySelector('header.top'); return { drag: t.getAttribute('data-tauri-drag-region'), pad: getComputedStyle(t).paddingLeft, strip: getComputedStyle(document.getElementById('marea-desktop-drag')).display }; });
ok(top.drag === 'deep' && top.pad === '86px' && top.strip === 'none', 'top bar: drag region, room for the window buttons, no extra strip ' + JSON.stringify(top));

// Notifications: first look only takes note
await a.waitForTimeout(2500);
ok(!(await a.evaluate(() => window.__calls.some((c) => c[0] === 'notify'))), 'nothing notified at the first look');
// Someone writes in wave 4 while the window isn't in front
await a.evaluate(() => { Document.prototype.hasFocus = () => false; });
execSync(`mysql oladev -e "UPDATE marea_participants SET last_read_seq = 0 WHERE wave_id = 4 AND user_id = 1; UPDATE marea_waves SET updated_at = UNIX_TIMESTAMP() + 5 WHERE id = 4;"`);
await a.waitForTimeout(3000);
const notes = await a.evaluate(() => window.__calls.filter((c) => c[0] === 'notify').map((c) => c[1]));
ok(notes.length === 1 && notes[0].title === 'Ideas para el boletín de diciembre', 'new message notified once: ' + JSON.stringify(notes));
await a.waitForTimeout(2500);
ok((await a.evaluate(() => window.__calls.filter((c) => c[0] === 'notify').length)) === 1, 'not notified again');

// The number on the app's icon: unread waves (Marea's title) plus unread mail (Marea's menu); the test mailbox has 2
const icon = () => a.evaluate(() => { const c = window.__calls.filter((x) => x[0] === 'set_badge'); return c.length ? c[c.length - 1][1].count : null; });
const counts = () => a.evaluate(() => {
  const m = /^\((\d+)\)/.exec(document.title); const b = document.querySelector('a[data-nav="mail"]:not(.sub) .badge');
  return { waves: m ? +m[1] : 0, mail: b && !b.hidden ? parseInt(b.textContent, 10) || 0 : 0 };
});
for (let i = 0; i < 20 && (await counts()).waves !== 1; i++) await a.waitForTimeout(500);
const seen = await counts();
ok(seen.waves === 1 && seen.mail === 2 && (await icon()) === 3, 'icon: 1 unread wave + 2 unread mails = 3 ' + JSON.stringify(seen));
// Hidden window: Marea stops asking its server, the bridge asks (wave 4 read elsewhere meanwhile)
await a.evaluate(() => { window.__fetched.length = 0; Object.defineProperty(document, 'visibilityState', { get: () => 'hidden', configurable: true }); });
execSync(`mysql oladev -e "UPDATE marea_participants p JOIN marea_waves w ON w.id = p.wave_id SET p.last_read_seq = w.seq WHERE p.wave_id = 4 AND p.user_id = 1"`);
for (let i = 0; i < 20 && (await icon()) !== 2; i++) await a.waitForTimeout(500);
ok((await icon()) === 2, 'icon while hidden follows the server: 0 waves + 2 mails = 2');
ok((await a.evaluate(() => window.__fetched.some((u) => u.includes('a=panel_mail')))), 'while hidden, the bridge itself asks for the unread mail');
ok((await a.evaluate(() => window.__calls.filter((x) => x[0] === 'set_badge').every((x) => Number.isInteger(x[1].count) && x[1].count >= 0))), 'icon counts are whole numbers');
await a.evaluate(() => { delete document.visibilityState; document.dispatchEvent(new Event('visibilitychange')); });

// Coming back to the window opens it
await a.evaluate(() => window.dispatchEvent(new Event('focus')));
await a.waitForTimeout(800);
ok((await a.evaluate(() => location.hash)) === '#/w/4', 'focus right after the notification opens that wave');
await a.evaluate(() => { Document.prototype.hasFocus = () => true; });

// Location, through Marea's own weather dialog
await a.evaluate(() => import('./assets/js/weather.js').then((m) => m.choosePlace(() => {})));
await a.click('dialog[open] button:has-text("Usar mi ubicación")');
await a.waitForTimeout(2000);
const loc = await a.evaluate(() => ({ asked: window.__calls.some((c) => c[0] === 'locate'), saved: localStorage.getItem('marea.wx'), dialog: !!document.querySelector('dialog[open]') }));
ok(loc.asked && loc.saved && JSON.parse(loc.saved).lat === 39.47 && !loc.dialog, 'location comes from the app and Marea keeps it ' + JSON.stringify(loc));

// Opening one of Marea's own addresses in the browser (0.2.4): only this Marea's addresses, only its top window
const open = await a.evaluate(async () => {
  const d = window.__MAREA_DESKTOP__;
  const opened = () => window.__calls.filter((c) => c[0] === 'open_external').map((c) => c[1].url);
  const before = opened().length;
  const out = { has: typeof d.openInBrowser === 'function', frozen: Object.isFrozen(d), keys: [d.version, d.platform] };
  out.result = await d.openInBrowser('#/enlazar/abc');
  out.sent = opened().slice(before);
  const refused = async (u) => { try { await d.openInBrowser(u); return false; } catch (e) { return true; } };
  out.refused = [await refused('https://accounts.google.com/o/oauth2/v2/auth'), await refused('javascript:alert(1)'),
    await refused('http://[bad'), await refused('//evil.example/x'), await refused(location.origin.replace('localhost', '127.0.0.1') + '/')];
  out.after = opened().length - before;
  const f = document.createElement('iframe'); document.body.append(f); // an inner frame of this same page
  await new Promise((r) => { setTimeout(r, 300); });
  const fd = f.contentWindow.__MAREA_DESKTOP__;
  out.frame = { bridge: !!fd, open: fd ? typeof fd.openInBrowser : 'none' };
  f.remove();
  out.here = location.href.split('#')[0];
  return out;
});
ok(open.has && open.frozen && open.keys.join() === '0.2.0,macos' && open.result === undefined, 'openInBrowser is there, read-only, next to version and platform ' + JSON.stringify(open.keys));
ok(open.sent.length === 1 && open.sent[0] === open.here + '#/enlazar/abc', 'openInBrowser sends this Marea\'s address to the app ' + JSON.stringify(open.sent));
ok(open.refused.every(Boolean) && open.after === 1, 'other sites, other schemes, bad and look-alike addresses are refused ' + JSON.stringify(open.refused));
ok(open.frame.bridge && open.frame.open === 'undefined', 'inner frames get no openInBrowser ' + JSON.stringify(open.frame));
const vib = await a.evaluate(async () => {
  const f = document.createElement('iframe'); document.body.append(f);
  await new Promise((r) => { setTimeout(r, 300); });
  const out = { top: window.__MAREA_DESKTOP__.vibrancy, frame: f.contentWindow.__MAREA_DESKTOP__ ? f.contentWindow.__MAREA_DESKTOP__.vibrancy : 'none' };
  f.remove();
  return out;
});
ok(vib.top === true && vib.frame === undefined, 'macOS: vibrancy is true in Marea\'s window, not in inner frames ' + JSON.stringify(vib));

// New mail (0.4, Marea 0.25.1's `mail_new`): the first answer only gives the mark; then at most 3 notifications,
// newest first, each call with the mark of the one before; the newest opens on focus; never while you're looking
ok(mailCalls.length >= 1 && !mailCalls[0].includes('since='), 'mail: the first call asks without a mark ' + JSON.stringify(mailCalls.slice(0, 2)));
const mailItems = [1, 2, 3, 4].map((n) => ({ box: '3', addr: 'dev@demo.test', id: String(80 - n), from: 'Remitente ' + n,
  subject: 'Asunto ' + n, date: 1795000000 - n, url: '#/mail/b/3/' + (80 - n) }));
await a.evaluate(() => { Document.prototype.hasFocus = () => false; window.__calls.length = 0; });
mailReply = { status: 200, body: { mark: 'm2', items: mailItems, quiet: '' } };
for (let i = 0; i < 20 && !(await a.evaluate(() => window.__calls.some((c) => c[0] === 'notify'))); i++) await a.waitForTimeout(250);
mailReply = { status: 200, body: { mark: 'm3', items: [], quiet: '' } };
await a.waitForTimeout(2000);
const mailNotes = await a.evaluate(() => window.__calls.filter((c) => c[0] === 'notify').map((c) => c[1]));
ok(mailNotes.length === 3 && mailNotes[0].title === 'Remitente 1' && mailNotes[0].body === 'Asunto 1' && mailNotes[2].title === 'Remitente 3',
  'mail: 3 notifications at most, newest first ' + JSON.stringify(mailNotes.map((n) => n.title)));
ok(mailCalls.some((u) => u.includes('since=m1')) && mailCalls.some((u) => u.includes('since=m2')), 'mail: each call sends the mark of the one before');
const opened = await a.evaluate(() => { window.dispatchEvent(new Event('focus')); return location.hash; });
ok(opened === '#/mail/b/3/79', 'mail: focus right after the notification opens the newest email ' + opened);
// Not an address of a mail in Marea: notified, but focus opens nothing (a fresh page: wait for its first call)
const beforeReload = mailCalls.length;
await a.goto(B + '#/'); await a.waitForSelector('.shell');
for (let i = 0; i < 20 && mailCalls.length <= beforeReload; i++) await a.waitForTimeout(250);
await a.evaluate(() => { Document.prototype.hasFocus = () => false; window.__calls.length = 0; });
mailReply = { status: 200, body: { mark: 'm5', items: [{ ...mailItems[0], url: 'javascript:alert(1)' }], quiet: '' } };
for (let i = 0; i < 20 && !(await a.evaluate(() => window.__calls.some((c) => c[0] === 'notify'))); i++) await a.waitForTimeout(250);
mailReply = { status: 200, body: { mark: 'm6', items: [], quiet: '' } };
const stayed = await a.evaluate(() => { const before = location.hash; window.dispatchEvent(new Event('focus')); return location.hash === before; });
ok(stayed, 'mail: an address that isn\'t a mail in Marea is never opened');
// While you're looking at the window: nothing
await a.evaluate(() => { Document.prototype.hasFocus = () => true; window.__calls.length = 0; });
mailReply = { status: 200, body: { mark: 'm7', items: mailItems.slice(0, 1), quiet: '' } };
await a.waitForTimeout(2500);
mailReply = { status: 200, body: { mark: 'm8', items: [], quiet: '' } };
ok(!(await a.evaluate(() => window.__calls.some((c) => c[0] === 'notify'))), 'mail: nothing while you are looking at the window');
// A Marea without `mail_new` (404 at the first call): the bridge stops asking
mailReply = { status: 404, body: { error: 'Acción desconocida' } };
await a.reload(); await a.waitForSelector('.shell');
const callsAfter404 = mailCalls.length;
await a.waitForTimeout(4500);
ok(mailCalls.length - callsAfter404 <= 1, 'mail: a Marea without mail_new is asked once and left alone (' + (mailCalls.length - callsAfter404) + ' calls)');
mailReply = { status: 200, body: { mark: 'm1', items: [], quiet: '' } };
ok(errors.length === 0, 'no page errors ' + JSON.stringify(errors));

// Windows: no Windows frame, so the bridge draws minimize, maximize and close at the top right
const win = await browser.newContext({ viewport: { width: 1280, height: 800 }, locale: 'es-ES' });
const windowsBridge = fs.readFileSync(path.join(here, '..', 'src-tauri', 'src', 'bridge.js'), 'utf8')
  .replace(/__PLATFORM__/g, 'windows').replace(/__VERSION__/g, '0.2.3').replace(/__VIBRANCY__/g, 'no')
  .replace('"__LABELS__"', JSON.stringify({ minimize: 'Minimizar', maximize: 'Maximizar', restore: 'Restaurar', close: 'Cerrar' }));
await win.addInitScript(fake.replace("if (cmd === 'locate')", "if (cmd === 'plugin:window|toggle_maximize') { window.__max = !window.__max; return Promise.resolve(null); }\n  if (cmd === 'plugin:window|is_maximized') return Promise.resolve(!!window.__max);\n  if (cmd === 'locate')") + '\n' + windowsBridge);
const w = await win.newPage();
const werrors = []; w.on('pageerror', (e) => werrors.push(e.message));
await w.goto(B);
ok((await w.locator('#marea-desktop-buttons button').count()) === 3, 'windows: three window buttons on the sign-in page');
await w.fill('input[type=email]', 'ana@example.com'); await w.fill('input[type=password]', 'supersecreta1');
await w.click('button.primary:has-text("Entrar")'); await w.waitForSelector('.shell'); await w.waitForTimeout(600);
const wtop = await w.evaluate(() => {
  const t = document.querySelector('header.top'); const box = document.getElementById('marea-desktop-buttons'); const r = box.getBoundingClientRect();
  return { drag: t.getAttribute('data-tauri-drag-region'), pad: getComputedStyle(t).paddingRight, right: Math.round(innerWidth - r.right), top: Math.round(r.top),
    labels: [...box.querySelectorAll('button')].map((b) => b.getAttribute('aria-label')), color: getComputedStyle(box).color, headerColor: getComputedStyle(t).color };
});
ok(wtop.drag === 'deep' && wtop.pad === '146px' && wtop.right === 0 && wtop.top === 0, 'windows: buttons at the top right, the bar makes room and drags ' + JSON.stringify(wtop));
ok((await w.evaluate(() => window.__MAREA_DESKTOP__.vibrancy)) === undefined, 'windows: no vibrancy (Mica waits for a test on a real Windows)');
ok(wtop.labels.join() === 'Minimizar,Maximizar,Cerrar' && wtop.color === wtop.headerColor, 'windows: labels in the app language and the bar colour ' + JSON.stringify(wtop.labels));
await w.click('#marea-desktop-buttons button[data-name=minimize]');
await w.click('#marea-desktop-buttons button[data-name=maximize]'); await w.waitForTimeout(200);
await w.click('#marea-desktop-buttons button[data-name=close]');
const wcalls = await w.evaluate(() => window.__calls.map((c) => c[0]).filter((c) => c.startsWith('plugin:window|')));
ok(['plugin:window|minimize', 'plugin:window|toggle_maximize', 'plugin:window|close'].every((c) => wcalls.includes(c)), 'windows: the buttons ask the app ' + JSON.stringify(wcalls));
ok((await w.getAttribute('#marea-desktop-buttons button[data-name=maximize]', 'aria-label')) === 'Restaurar', 'windows: maximized shows «Restaurar»');
ok(werrors.length === 0, 'windows: no page errors ' + JSON.stringify(werrors));
await win.close();
execSync(`mysql oladev -e "UPDATE marea_participants p JOIN marea_waves w ON w.id = p.wave_id SET p.last_read_seq = w.seq WHERE p.wave_id = 4"`);
await browser.close();
