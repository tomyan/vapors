// The name headless Chromium's accessibility tree gives a button holding
// a role="img" svg that is visibility: hidden, and the same button with
// the svg shown (docs/icons.md, a named image in a button):
//
//   env -u DBUS_SESSION_BUS_ADDRESS node crates/elements-corpus/probes/hidden-icon-name.mjs
//
// Chromium 155 printed (2026-11-10):
//
//   button: "Go"
//   button: "Export Go"
//   image: "Export"
//
// so a hidden named icon adds nothing to the name and is not in the
// tree; pinned on vapors by crates/elements/tests/svg.rs.
import { spawn } from 'node:child_process';
import { mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

// The whole probe took under 1 s on the dev box (2026-11-10); past 30 s
// it fails rather than waiting for ever.
const DEADLINE_MS = 30000;
setTimeout(() => {
  console.error(`hidden-icon-name probe: no result within ${DEADLINE_MS} ms (the deadline above)`);
  process.exit(1);
}, DEADLINE_MS).unref();
const profile = mkdtempSync(join(tmpdir(), 'hidden-icon-name-'));
const env = { ...process.env };
delete env.DBUS_SESSION_BUS_ADDRESS;
const chrome = spawn('chromium', ['--headless', '--remote-debugging-port=0', `--user-data-dir=${profile}`, '--no-first-run', 'about:blank'], { env });
const port = await new Promise((found) => chrome.stderr.on('data', (d) => {
  const m = /DevTools listening on ws:\/\/[^:]+:(\d+)/.exec(d.toString());
  if (m) found(m[1]);
}));
const json = async (path) => (await fetch(`http://127.0.0.1:${port}${path}`)).json();
let next = 0;
const waiting = new Map();
const page = (await json('/json')).find((t) => t.type === 'page');
const ws = new WebSocket(page.webSocketDebuggerUrl);
ws.onmessage = (e) => {
  const m = JSON.parse(e.data);
  if (waiting.has(m.id)) { waiting.get(m.id)(m); waiting.delete(m.id); }
};
await new Promise((open) => { ws.onopen = open; });
const send = (method, params = {}) => new Promise((done) => {
  const id = ++next;
  waiting.set(id, done);
  ws.send(JSON.stringify({ id, method, params }));
});
const icon = (style) => `<svg role="img" aria-label="Export" style="display:block;${style}" viewBox="0 0 16 16" width="16" height="16"><path d="M8 10V2" fill="none" stroke="currentColor" stroke-width="1.5"/></svg>`;
const html = `<button id=hidden style="display:flex">${icon('visibility:hidden')}Go</button>`
  + `<button id=shown style="display:flex">${icon('')}Go</button>`;
await send('Page.navigate', { url: 'data:text/html,' + encodeURIComponent(html) });
await new Promise((r) => setTimeout(r, 300));
const tree = (await send('Accessibility.getFullAXTree')).result.nodes;
for (const node of tree.filter((n) => !n.ignored && ['button', 'image'].includes(n.role?.value))) {
  console.log(`${node.role.value}: ${JSON.stringify(node.name?.value)}`);
}
chrome.kill();
rmSync(profile, { recursive: true, force: true });
process.exit(0);
