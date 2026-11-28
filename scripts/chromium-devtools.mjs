// A headless Chromium for a probe script, driven over the DevTools
// protocol: `launch` starts it with a fresh profile and closes it on
// exit, `connect` opens a page in it. Used by chromium-field-scroll.mjs.
//
// Chromium runs in its own process group, which is killed with SIGKILL
// and its profile removed on every exit node runs its exit handlers
// for: the end of a run, a failed step, process.exit, the start
// deadline, a failed start, and SIGINT, SIGTERM, SIGHUP and SIGQUIT.
// SIGKILL to node leaves both. So does a SIGKILL to node's whole
// process group (`kill -KILL -- -<pgid>`, `timeout -s KILL`): Chromium
// is outside it. Before it had its own group, such a kill took Chromium
// down too and left only the profile.
import {spawn} from 'node:child_process';
import {mkdtempSync, rmSync} from 'node:fs';
import {tmpdir} from 'node:os';
import {join} from 'node:path';

const SIGNALS = [['SIGINT', 130], ['SIGTERM', 143], ['SIGHUP', 129], ['SIGQUIT', 131]];

// Chromium started with `args` and a fresh profile under the temp dir,
// up to its DevTools URL within `startMs`. `name` heads every message.
// The deadline also covers opening the WebSocket: pass it to `connect`.
export async function launch({chrome, args, name, startMs}) {
  const profile = mkdtempSync(join(tmpdir(), `${name}-`));
  const chromium = {proc: undefined};
  closeOnExit(chromium, profile, name);
  chromium.proc = start(chrome, profile, args);
  const deadline = setTimeout(() => { console.error(`${name}: no Chromium page within ${startMs} ms; fix: check ${chrome} starts headless`); process.exit(1); }, startMs);
  return {wsUrl: await devtoolsUrl(chromium.proc, chrome), deadline};
}

// in its own process group, so its zygotes and renderers go with it
function start(chrome, profile, args) {
  try {
    return spawn(chrome, ['--headless=new', '--remote-debugging-port=0', `--user-data-dir=${profile}`, ...args],
      {detached: true, stdio: ['ignore', 'ignore', 'pipe']});
  } catch (error) {
    // spawn throws for most errors; ENOENT, EACCES, EAGAIN, EMFILE and
    // ENFILE come as 'error' events in devtoolsUrl
    throw cannotStart(chrome, error);
  }
}

const cannotStart = (chrome, error) => new Error(`cannot start Chromium ${chrome}: ${error.message}; fix: run \`cargo xtask setup\` or set $CHROME to a Chromium`);

function devtoolsUrl(proc, chrome) {
  return new Promise((ok, no) => {
    // before stderr, which is null when the spawn failed (EMFILE)
    proc.on('error', e => no(cannotStart(chrome, e)));
    let buf = '';
    proc.stderr?.on('data', d => { buf += d; const m = buf.match(/ws:\/\/[^\s]+/); if (m) ok(m[0]); });
    proc.on('exit', c => no(new Error(`chrome exited ${c}: ${buf}`)));
  });
}

function closeOnExit(chromium, profile, name) {
  process.on('exit', () => {
    killGroup(chromium.proc, name);
    removeProfile(profile, name);
  });
  for (const [signal, code] of SIGNALS) {
    process.on(signal, () => { console.error(`${name}: stopped by ${signal}; closing Chromium and removing ${profile}`); process.exit(code); });
  }
}

// a failure here prints its fix and sets exit 1 rather than throwing,
// so the profile is still removed
function killGroup(proc, name) {
  if (proc?.pid === undefined) return; // it never started
  try { process.kill(-proc.pid, 'SIGKILL'); } catch (error) {
    if (error.code === 'ESRCH') return;
    console.error(`${name}: cannot kill Chromium's process group ${proc.pid}: ${error.message}; fix: kill -KILL -- -${proc.pid}`);
    process.exitCode = 1;
  }
}

function removeProfile(profile, name) {
  // Chromium may still be writing the profile as it exits
  try { rmSync(profile, {recursive: true, force: true, maxRetries: 10, retryDelay: 100}); } catch (error) {
    console.error(`${name}: cannot remove ${profile}: ${error.message}; fix: rm -rf '${profile}'`);
    process.exitCode = 1;
  }
}

// A new page in the Chromium at `wsUrl`, once its WebSocket is open
// (clearing `launch`'s deadline): `browser(method, params)` sends to the
// browser, `page(method, params)` to the page.
export async function connect(wsUrl, deadline) {
  const socket = new WebSocket(wsUrl);
  await new Promise(r => socket.addEventListener('open', r));
  clearTimeout(deadline);
  let id = 0;
  const waiting = new Map();
  socket.addEventListener('message', e => { const m = JSON.parse(e.data); waiting.get(m.id)?.(m); waiting.delete(m.id); });
  const browser = (method, params = {}, sessionId) => new Promise((ok, no) => {
    const n = ++id;
    waiting.set(n, m => m.error ? no(new Error(`${method}: ${JSON.stringify(m.error)}`)) : ok(m.result));
    socket.send(JSON.stringify({id: n, method, params, sessionId}));
  });
  const {targetId} = await browser('Target.createTarget', {url: 'about:blank'});
  const {sessionId} = await browser('Target.attachToTarget', {targetId, flatten: true});
  return {browser, page: (method, params) => browser(method, params, sessionId)};
}
