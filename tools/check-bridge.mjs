// Runs the app's bridge (as macOS) inside the test Marea with a fake app behind it, and checks the new parts:
// notifications of new messages, opening that wave on focus, the title bar room and drag region, and location.
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
  .replace('setInterval(lookForNews, 45000)', 'setInterval(lookForNews, 2000)')
  .replace('setTimeout(lookForNews, 5000)', 'setTimeout(lookForNews, 1500)');
const fake = `window.__calls = [];
window.__TAURI_INTERNALS__ = { invoke: (cmd, args) => { window.__calls.push([cmd, args]);
  if (cmd === 'locate') return Promise.resolve({ latitude: 39.4699, longitude: -0.3763, accuracy: 1000 });
  return Promise.resolve(null); } };`;

const browser = await chromium.launch({ executablePath: '/usr/bin/google-chrome', args: ['--no-sandbox'] });
const ctx = await browser.newContext({ viewport: { width: 1280, height: 800 }, locale: 'es-ES' });
await ctx.addInitScript(fake + '\n' + bridge);
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
ok(errors.length === 0, 'no page errors ' + JSON.stringify(errors));
execSync(`mysql oladev -e "UPDATE marea_participants p JOIN marea_waves w ON w.id = p.wave_id SET p.last_read_seq = w.seq WHERE p.wave_id = 4"`);
await browser.close();
