// How headed Chromium orders the events of the window losing and
// regaining focus, which the events oracle cannot record: headless
// Chromium never moves focus between windows. Run under Xvfb:
//
//   env -u DBUS_SESSION_BUS_ADDRESS xvfb-run -a node crates/elements-corpus/probes/window-focus.mjs
//
// For a number field at 5 (#a) and a text field holding "x" (#t): the
// field is focused and 7 typed over it; another window is brought to
// the front, then this one again. Chromium 155 printed, for each:
//
//   lost:     change, blur, focusout, window blur (activeElement: a / t)
//   regained: window focus, focus, focusin (activeElement: a / t)
//
// pinned on vapors by crates/elements/tests/focus_window.rs.
import { spawn } from 'node:child_process';
import { mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

const profile = mkdtempSync(join(tmpdir(), 'window-focus-'));
const env = { ...process.env };
delete env.DBUS_SESSION_BUS_ADDRESS;
const chrome = spawn('chromium', ['--remote-debugging-port=0', `--user-data-dir=${profile}`, '--no-first-run', 'about:blank'], { env });
const port = await new Promise((found) => chrome.stderr.on('data', (d) => {
  const m = /DevTools listening on ws:\/\/[^:]+:(\d+)/.exec(d.toString());
  if (m) found(m[1]);
}));
const json = async (path) => (await fetch(`http://127.0.0.1:${port}${path}`)).json();
let next = 0;
const waiting = new Map();
async function connect(url) {
  const ws = new WebSocket(url);
  ws.onmessage = (e) => {
    const m = JSON.parse(e.data);
    if (waiting.has(m.id)) { waiting.get(m.id)(m); waiting.delete(m.id); }
  };
  await new Promise((open) => { ws.onopen = open; });
  return (method, params = {}) => new Promise((done) => {
    const id = ++next;
    waiting.set(id, done);
    ws.send(JSON.stringify({ id, method, params }));
  });
}
const settle = () => new Promise((r) => setTimeout(r, 300));
const page = (await json('/json')).find((t) => t.type === 'page');
const send = await connect(page.webSocketDebuggerUrl);
const value = async (expression) => (await send('Runtime.evaluate', { expression, returnByValue: true })).result.result.value;
const html = `<input id=a type=number value=5><input id=t type=text value=x><script>window.log=[];
for(const f of [a,t])for(const n of ['change','blur','focusout','focus','focusin'])f.addEventListener(n,()=>log.push(n));
addEventListener('blur',()=>log.push('window blur'));addEventListener('focus',()=>log.push('window focus'));</script>`;
await send('Page.navigate', { url: 'data:text/html,' + encodeURIComponent(html) });
await settle();
await send('Page.bringToFront');
await settle();
const browser = await connect((await json('/json/version')).webSocketDebuggerUrl);
for (const field of ['a', 't']) {
  await value(`${field}.focus(); ${field}.select(); log.length = 0`);
  await send('Input.insertText', { text: '7' });
  const other = await browser('Target.createTarget', { url: 'about:blank', newWindow: true });
  const sendOther = await connect((await json('/json')).find((t) => t.id === other.result.targetId).webSocketDebuggerUrl);
  await sendOther('Page.bringToFront');
  await settle();
  console.log(`#${field} lost:`, await value('log.splice(0).join(", ")'), '(activeElement:', await value('document.activeElement.id') + ')');
  await send('Page.bringToFront');
  await settle();
  console.log(`#${field} regained:`, await value('log.splice(0).join(", ")'), '(activeElement:', await value('document.activeElement.id') + ')');
  await browser('Target.closeTarget', { targetId: other.result.targetId });
  await settle();
}
// Chromium 155 exited 19 to 58 ms after SIGTERM on the dev box (three
// runs, 2026-11-02); 5 s is a hundred times that. Past it the probe
// kills Chromium and fails, rather than waiting for ever.
const EXIT_DEADLINE_MS = 5000;
chrome.kill();
const exited = await Promise.race([
  new Promise((done) => chrome.on('exit', () => done(true))),
  new Promise((done) => setTimeout(() => done(false), EXIT_DEADLINE_MS)),
]);
if (!exited) {
  chrome.kill('SIGKILL');
  rmSync(profile, { recursive: true, force: true });
  console.error(`window-focus probe: Chromium did not exit within ${EXIT_DEADLINE_MS} ms of SIGTERM (the deadline above); it was killed`);
  process.exit(1);
}
// the profile is a Chromium user-data directory, tens of MB: gone with the run
rmSync(profile, { recursive: true, force: true });
process.exit(0);
