// Runs every menu shortcut script from src-tauri/src/actions.rs inside a test Marea and checks what each one did.
//
// Needs the Marea repository checked out next to this one (or MAREA_REPO pointing at it) with its test install
// running (see its CLAUDE.md), and Playwright. Run: node tools/check-actions.mjs
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const here = path.dirname(fileURLToPath(import.meta.url));
const marea = process.env.MAREA_REPO || path.join(here, '..', '..', 'marea');
const { start, ok } = await import(pathToFileURL(path.join(marea, 'dev', 'devlib.mjs')).href);

const rs = fs.readFileSync(path.join(here, '..', 'src-tauri', 'src', 'actions.rs'), 'utf8');
const body = rs.slice(rs.indexOf('pub fn script'));
const consts = Object.fromEntries([...rs.matchAll(/pub const (\w+): &str = "marea:(\w+)";/g)].map((m) => [m[1], m[2]]));
const scripts = {};
for (const m of body.matchAll(/^\s+(\w+) => (?:"([^\n]*)"|press!\("([^\n]*)"\)),$/gm)) {
  scripts[consts[m[1]]] = m[2] !== undefined ? m[2].replace(/\\'/g, "'") : `document.body.dispatchEvent(new KeyboardEvent('keydown',${m[3]}))`;
}
console.log('scripts:', Object.keys(scripts).join(', '));

const { browser, a, errors } = await start();
const run = async (name) => { await a.evaluate(scripts[name]); await a.waitForTimeout(700); };
const closeDialogs = async () => { await a.keyboard.press('Escape'); await a.waitForTimeout(300); };

for (const [name, hash] of [['mail', '#/mail'], ['agenda', '#/agenda'], ['people', '#/people'], ['drops', '#/drop'], ['podcasts', '#/podcasts'], ['notes', '#/notas'], ['settings', '#/settings'], ['waves', '#/']]) {
  await run(name);
  const h = await a.evaluate(() => location.hash);
  ok(h === hash || h.startsWith(hash + '/'), `${name} → ${h}`);
}
// Start from inside an editor-like field to prove keys go to the page body, not the focused box
await a.focus('input.search');
await run('assistant');
ok(await a.locator('.cmd-input').isVisible(), 'assistant opens the command bar even with a text box focused');
await closeDialogs(); await closeDialogs();
await run('search');
ok(await a.evaluate(() => document.activeElement && document.activeElement.matches('input.search')), 'search focuses the search box');
await a.evaluate(() => document.activeElement.blur());
await run('shortcuts');
ok(await a.locator('dialog[open] h2', { hasText: 'Atajos de teclado' }).isVisible(), 'shortcuts opens Marea\'s keyboard help');
await closeDialogs();
await run('new_wave');
ok(await a.locator('dialog[open] h2', { hasText: 'Nueva wave' }).isVisible(), 'new wave opens the new wave dialog');
await closeDialogs();
await run('compose');
ok((await a.evaluate(() => location.hash)) === '#/mail/new', 'compose goes to the mail composer');
ok(errors.length === 0, 'no page errors ' + JSON.stringify(errors));
await browser.close();
