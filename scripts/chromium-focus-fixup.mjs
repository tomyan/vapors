// Chromium's focus fixup, for crates/elements/tests/focus_fixup.rs: a
// focused <input> with "er" typed into it is hidden (itself, an
// ancestor or html display: none, visibility: hidden), disabled or
// removed; the events that reach a document capture listener are
// printed per case, then where a key typed afterwards goes.
//
// Usage: node scripts/chromium-focus-fixup.mjs
// Needs node 22 or later and chromium ($CHROME overrides). Exits 1
// naming the step that failed.
import {spawn} from 'node:child_process';
import {mkdtemp, rm} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import {join} from 'node:path';

// a cold Chromium took 58 s to its first page here
// (COLD_START in crates/text/src/comparison/chromium_warm.rs)
const START_MS = 60000;
const CASES = {
  'self display: none': "f().style.display='none'",
  'ancestor display: none': "f().parentElement.style.display='none'",
  'html display: none': "document.documentElement.style.display='none'",
  'visibility: hidden': "f().style.visibility='hidden'",
  'disabled': 'f().disabled=true',
  'removed': 'f().remove()',
  // a change handler that focuses a button (review-v190 M1)
  'change focuses a button': "f().onchange=()=>b().focus();f().style.display='none'",
  // the same while focus moves to another field
  'change focuses a button, focus going to g': "f().onchange=()=>b().focus();g().focus()",
  // a blur handler that refocuses the hidden field (M2)
  'blur refocuses the field': "f().onblur=()=>f().focus();f().style.display='none'",
};
// the key sent straight after the hide, with no wait between (L1)
const NO_WAIT = {'key with no wait': "f().style.display='none'"};
// a composition in progress at the hide (L2)
const COMPOSING = {'composing': "f().style.display='none'"};
const PAGE = `<!doctype html><div><input id=f value=Warm><button id=b>B</button><input id=g></div><script>
window.log=[];const f=()=>document.getElementById('f');const b=()=>document.getElementById('b');const g=()=>document.getElementById('g');
const at=()=>document.activeElement.id||document.activeElement.nodeName;
for(const n of ['blur','focusout','focus','focusin','change','input','keydown','compositionstart','compositionend'])
 document.addEventListener(n,e=>log.push(n+' on '+(e.target.id||e.target.nodeName)+
  ' (active '+at()+', connected '+e.target.isConnected+')'),true);
</script>`;

const sleep = ms => new Promise(r => setTimeout(r, ms));

async function launch(dir) {
  const port = 9300 + Math.floor(Math.random() * 600);
  const env = {...process.env};
  delete env.DBUS_SESSION_BUS_ADDRESS; // see CLAUDE.md: HTTP loads wait 120 s
  const args = ['--headless=new', '--no-sandbox', `--remote-debugging-port=${port}`,
    `--user-data-dir=${join(dir, 'profile')}`, 'about:blank'];
  const browser = spawn(process.env.CHROME || 'chromium', args, {env, detached: true, stdio: 'ignore'});
  const failed = {};
  browser.once('error', error => { failed.error = error; });
  return {browser, page: await firstPage(port, failed)};
}

async function firstPage(port, failed) {
  for (const end = Date.now() + START_MS; Date.now() < end; await sleep(200)) {
    if (failed.error) throw new Error(`${failed.error.message}; fix: install chromium or set CHROME`);
    try {
      const list = await (await fetch(`http://127.0.0.1:${port}/json`)).json();
      const page = list.find(t => t.type === 'page');
      if (page) return page;
    } catch {}
  }
  throw new Error(`no Chromium page on port ${port} within ${START_MS} ms; fix: check that chromium starts headless`);
}

async function connect(page) {
  const ws = new WebSocket(page.webSocketDebuggerUrl);
  await new Promise((ok, fail) => { ws.onopen = ok; ws.onerror = fail; });
  let id = 0; const waiting = new Map();
  ws.onmessage = m => { const d = JSON.parse(m.data); waiting.get(d.id)?.(d); waiting.delete(d.id); };
  const send = (method, params = {}) => new Promise((ok, fail) => {
    const sent = ++id; waiting.set(sent, d => d.error ? fail(new Error(`${method}: ${d.error.message}`)) : ok(d.result));
    ws.send(JSON.stringify({id: sent, method, params}));
  });
  return {ws, send};
}

async function run(send, name, mutation, {wait = 300, composing = false} = {}) {
  const evaluate = expression => send('Runtime.evaluate', {expression, returnByValue: true});
  await send('Page.navigate', {url: 'data:text/html,' + encodeURIComponent(PAGE)});
  await sleep(500);
  await evaluate("f().focus();f().setSelectionRange(0,0)");
  await send('Input.insertText', {text: 'er'});
  if (composing) await send('Input.imeSetComposition', {text: 'ka', selectionStart: 2, selectionEnd: 2});
  await evaluate(`log.push('-- ${name}');${mutation};log.push('-- after the mutation, active '+at())`);
  if (wait) await sleep(wait);
  if (wait) await evaluate("log.push('-- x typed')");
  await send('Input.dispatchKeyEvent', {type: 'keyDown', key: 'x', text: 'x', code: 'KeyX', windowsVirtualKeyCode: 88});
  await send('Input.dispatchKeyEvent', {type: 'keyUp', key: 'x', code: 'KeyX', windowsVirtualKeyCode: 88});
  await evaluate("log.push('value '+(f()?f().value:'(removed)'))");
  return (await evaluate("log.join('\\n')")).result.value;
}

const dir = await mkdtemp(join(tmpdir(), 'focus-fixup-'));
let browser;
try {
  const started = await launch(dir);
  browser = started.browser;
  const {ws, send} = await connect(started.page);
  await send('Page.enable');
  await send('Emulation.setFocusEmulationEnabled', {enabled: true});
  for (const [name, mutation] of Object.entries(CASES)) console.log(await run(send, name, mutation) + '\n');
  for (const [name, mutation] of Object.entries(NO_WAIT)) console.log(await run(send, name, mutation, {wait: 0}) + '\n');
  for (const [name, mutation] of Object.entries(COMPOSING)) console.log(await run(send, name, mutation, {composing: true}) + '\n');
  ws.close();
} catch (error) {
  console.error(`chromium-focus-fixup: ${error.message}; rerun: node scripts/chromium-focus-fixup.mjs`);
  process.exitCode = 1;
} finally {
  if (browser?.pid) process.kill(-browser.pid);
  // the killed Chromium may still be writing its profile
  await rm(dir, {recursive: true, force: true, maxRetries: 10, retryDelay: 200});
}
