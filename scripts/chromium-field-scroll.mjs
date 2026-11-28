// Chromium's long text field, for crates/elements/tests/field_scroll.rs
// and field_scroll_input.rs (entry 44): the gui-bench search input's
// box at (676, 14), typed into with "W" 60 times (Input.insertText),
// deleted with Backspace, blurred, focused, clicked at x 800 and
// wheel-scrolled. Each step prints the text length, the field's
// scrollLeft, the caret's x as measured (the content box's left, 689,
// plus the text's width before the caret, less scrollLeft) and the
// columns its caret is painted in (caret-color red, from a screenshot;
// "-" where none shows, as after blur), one line per step.
//
// With `scrollbar` (tests/field_scrollbar.rs), scrollbars are shown
// and the same input with a green scrollbar-color and no
// scrollbar-width, then auto, then thin, is typed into 60 times and
// wheel-scrolled; scrolling divs styled the same, auto and thin, are
// the controls that a thumb would be seen. Each step prints the case,
// the computed scrollbar-width, the scrollLeft and the columns painted
// green ("-" for none).
//
// With `overflow` (tests/field_overflow.rs), the same input under each
// author overflow (none given, hidden, overflow-x: hidden, clip,
// visible) is typed into 60 times, wheeled by -100 on x, blurred, and
// focused again. Each step prints the case, the computed overflow-x,
// the scrollLeft, the red caret's columns and the text's ink columns
// (dark pixels, the text colour) in the field's rows.
//
// With `centre` (tests/field_text_centre.rs), a text and a number
// input, each also under an author flex start, a wrapped reversed row
// packed to its end, and a grid aligned to its end, show the placeholder "Search" while empty, then are
// typed into ("Ada", "123"). Each step prints the case, the rows of the
// text's ink (dark or placeholder grey) and of the red caret, in the
// field's columns, and the caret's columns ("-" for none).
//
// Usage: node scripts/chromium-field-scroll.mjs [scrollbar | overflow | centre]
// Needs node 22 or later and the pinned Chromium (.config/tools.toml
// `chromium.version`, installed by `cargo xtask setup` under
// ~/.local/share/factory); $CHROME overrides. Exits 1 naming the step
// that failed.
import {spawn} from 'node:child_process';
import {mkdtempSync, readFileSync, rmSync} from 'node:fs';
import {homedir, tmpdir} from 'node:os';
import {dirname, join} from 'node:path';
import {fileURLToPath} from 'node:url';
import {inflateSync} from 'node:zlib';

// a cold Chromium took 58 s to its first page here
// (COLD_START in crates/text/src/comparison/chromium_warm.rs)
const START_MS = 60000;
// each step's wait for a wheel scroll to land: 500 ms showed it every run
const WHEEL_MS = 500;
// the caret blinks, so a screenshot can miss it: up to 6 taken 200 ms
// apart cover more than one blink period
const SHOTS = 6, SHOT_GAP_MS = 200;
const MODE = process.argv[2];
if (process.argv.length > 3 || ![undefined, 'scrollbar', 'overflow', 'centre'].includes(MODE)) {
  console.error(`chromium-field-scroll: unknown arguments ${process.argv.slice(2).join(' ')}; fix: give none, \`scrollbar\`, \`overflow\` or \`centre\``);
  process.exit(1);
}
const SCROLLBAR = MODE === 'scrollbar';
const STYLE = `<!doctype html><style>html,body{margin:0;background:#fff}
#s{position:absolute;left:676px;top:14px;box-sizing:border-box;width:280px;height:36px;margin:0;padding:0 12px;
border:1px solid #d1d5db;border-radius:8px;background:#f9fafb;color:#111827;font:14px sans-serif;outline:none;caret-color:#ff0000}</style>`;
const PAGE = `${STYLE}<input id=s type=text>`;
// the scrollbar cases: [name, page]
const BAR = (width, element) => `${STYLE}<style>#s{${width ? `scrollbar-width:${width};` : ''}scrollbar-color:#00ff00 transparent}</style>${element}`;
const INPUT = '<input id=s type=text>', DIV = `<div id=s style="overflow-x:auto;white-space:nowrap">${'W'.repeat(60)}</div>`;
const BAR_CASES = [
  ['input, none given', BAR(null, INPUT)],
  ['input, auto', BAR('auto', INPUT)],
  ['input, thin', BAR('thin', INPUT)],
  ['div, auto (control)', BAR('auto', DIV)],
  ['div, thin (control)', BAR('thin', DIV)],
];
// the overflow cases: [name, the author's declarations]
const OVERFLOW_CASES = [
  ['none given', ''],
  ['overflow: hidden', 'overflow:hidden'],
  ['overflow-x: hidden', 'overflow-x:hidden'],
  ['overflow: clip', 'overflow:clip'],
  ['overflow: visible', 'overflow:visible'],
];
// the centring cases: [name, type, author declarations, typed text]
const CENTRE_CASES = [
  ['text', 'text', '', 'Ada'],
  ['number', 'number', '', '123'],
  ['text, author flex start', 'text', 'display:flex;align-items:flex-start', 'Ada'],
  ['number, author flex start', 'number', 'display:flex;align-items:flex-start', '123'],
  ['text, author wrapped row end', 'text', 'display:flex;flex-direction:row-reverse;flex-wrap:wrap;justify-content:flex-end', 'Ada'],
  ['number, author wrapped row end', 'number', 'display:flex;flex-direction:row-reverse;flex-wrap:wrap;justify-content:flex-end', '123'],
  ['text, author grid end', 'text', 'display:grid;align-items:end', 'Ada'],
  ['number, author grid end', 'number', 'display:grid;align-items:end', '123'],
];

function pinned() {
  const root = join(dirname(fileURLToPath(import.meta.url)), '..');
  const tools = readFileSync(join(root, '.config/tools.toml'), 'utf8');
  const version = tools.match(/^chromium\.version = "([^"]+)"/m);
  if (!version) throw new Error('no chromium.version in .config/tools.toml');
  return join(homedir(), `.local/share/factory/chromium-${version[1]}/chrome-linux/chrome`);
}

// The x range of pixels `is` picks in rows 15..48 of an 8-bit RGB(A),
// non-interlaced PNG: red for the caret, green for a thumb.
const RED = (r, g, b) => r > 0xc0 && g < 0x40 && b < 0x40;
const GREEN = (r, g, b) => r < 0x40 && g > 0xc0 && b < 0x40;
// the text's colour, #111827, and its antialiased edge
const INK = (r, g, b) => r < 0x80 && g < 0x80 && b < 0x90;
function columns(png, is) {
  return extent(png, is, 0);
}

// The y range of pixels `is` picks in the field's columns 677..955.
function rows(png, is) {
  return extent(png, is, 1);
}

// The x (axis 0, rows 15..48) or y (axis 1, columns 677..955) range of
// the pixels `is` picks.
function extent(png, is, axis) {
  let p = 8, w, h, ct;
  const idat = [];
  while (p < png.length) {
    const len = png.readUInt32BE(p), type = png.toString('ascii', p + 4, p + 8), data = png.subarray(p + 8, p + 8 + len);
    if (type === 'IHDR') {
      [w, h, ct] = [data.readUInt32BE(0), data.readUInt32BE(4), data[9]];
      if (data[8] !== 8 || data[12]) throw new Error('screenshot: not an 8-bit non-interlaced PNG');
    }
    if (type === 'IDAT') idat.push(data);
    p += 12 + len;
  }
  const bpp = {6: 4, 2: 3}[ct];
  if (!bpp) throw new Error(`screenshot: PNG colour type ${ct}`);
  const raw = inflateSync(Buffer.concat(idat)), stride = w * bpp, img = Buffer.alloc(h * stride);
  for (let y = 0; y < h; y++) {
    const f = raw[y * (stride + 1)], row = raw.subarray(y * (stride + 1) + 1, (y + 1) * (stride + 1));
    for (let x = 0; x < stride; x++) {
      const a = x >= bpp ? img[y * stride + x - bpp] : 0, b = y ? img[(y - 1) * stride + x] : 0;
      const c = x >= bpp && y ? img[(y - 1) * stride + x - bpp] : 0;
      const paeth = () => { const q = a + b - c, pa = Math.abs(q - a), pb = Math.abs(q - b), pc = Math.abs(q - c); return pa <= pb && pa <= pc ? a : pb <= pc ? b : c; };
      const pr = [0, a, b, (a + b) >> 1][f] ?? paeth();
      img[y * stride + x] = (row[x] + pr) & 255;
    }
  }
  const found = [];
  const [y0, y1, x0, x1] = axis ? [0, h, 677, 956] : [15, 49, 0, w];
  for (let y = y0; y < y1; y++) for (let x = x0; x < x1; x++) {
    const o = y * stride + x * bpp;
    if (is(img[o], img[o + 1], img[o + 2])) found.push(axis ? y : x);
  }
  return found.length ? `${Math.min(...found)}..${Math.max(...found)}` : '-';
}

async function main() {
  const chrome = process.env.CHROME || pinned();
  const profile = mkdtempSync(join(tmpdir(), 'field-scroll-'));
  const chromium = {proc: undefined};
  closeOnExit(chromium, profile);
  chromium.proc = start(chrome, profile);
  const deadline = setTimeout(() => { console.error(`no Chromium page within ${START_MS} ms; fix: check ${chrome} starts headless`); process.exit(1); }, START_MS);
  await run(chromium.proc, deadline);
}

// in its own process group, so its zygotes and renderers go with it
function start(chrome, profile) {
  try {
    return spawn(chrome, ['--headless=new', '--remote-debugging-port=0', `--user-data-dir=${profile}`,
      '--window-size=1280,800', ...(SCROLLBAR ? [] : ['--hide-scrollbars']), '--force-device-scale-factor=1', 'about:blank'],
      {detached: true, stdio: ['ignore', 'ignore', 'pipe']});
  } catch (error) {
    // most spawn failures throw here; ENOENT, EACCES and EAGAIN come as 'error' in run
    throw cannotStart(error);
  }
}

const cannotStart = error => new Error(`cannot start Chromium: ${error.message}; fix: run \`cargo xtask setup\` or set $CHROME to a Chromium`);

// Chromium and its profile go on every exit: the end of a run, a
// failed step, process.exit and SIGINT, SIGTERM or SIGHUP. SIGKILL,
// SIGQUIT and other signals that end node without its exit handlers
// leave both
function closeOnExit(chromium, profile) {
  let closed = false;
  process.on('exit', () => {
    if (closed) return;
    closed = true;
    killGroup(chromium.proc);
    removeProfile(profile);
  });
  for (const [signal, code] of [['SIGINT', 130], ['SIGTERM', 143], ['SIGHUP', 129]]) {
    process.on(signal, () => { console.error(`chromium-field-scroll: stopped by ${signal}; closing Chromium and removing ${profile}`); process.exit(code); });
  }
}

// a failure here prints its fix and sets exit 1 rather than throwing,
// so the profile is still removed
function killGroup(proc) {
  if (proc?.pid === undefined) return; // it never started
  try { process.kill(-proc.pid, 'SIGKILL'); } catch (error) {
    if (error.code === 'ESRCH') return;
    console.error(`chromium-field-scroll: cannot kill Chromium's process group ${proc.pid}: ${error.message}; fix: kill -KILL -- -${proc.pid}`);
    process.exitCode = 1;
  }
}

function removeProfile(profile) {
  // Chromium may still be writing the profile as it exits
  try { rmSync(profile, {recursive: true, force: true, maxRetries: 10, retryDelay: 100}); } catch (error) {
    console.error(`chromium-field-scroll: cannot remove ${profile}: ${error.message}; fix: rm -rf '${profile}'`);
    process.exitCode = 1;
  }
}

async function run(proc, deadline) {
  const wsUrl = await new Promise((ok, no) => {
    // before stderr, which is null when the spawn failed (EMFILE)
    proc.on('error', e => no(cannotStart(e)));
    let buf = '';
    proc.stderr?.on('data', d => { buf += d; const m = buf.match(/ws:\/\/[^\s]+/); if (m) ok(m[0]); });
    proc.on('exit', c => no(new Error(`chrome exited ${c}: ${buf}`)));
  });
  const browser = new WebSocket(wsUrl);
  await new Promise(r => browser.addEventListener('open', r));
  clearTimeout(deadline);
  let id = 0;
  const waiting = new Map();
  browser.addEventListener('message', e => { const m = JSON.parse(e.data); waiting.get(m.id)?.(m); waiting.delete(m.id); });
  const send = (method, params = {}, sessionId) => new Promise((ok, no) => {
    const n = ++id;
    waiting.set(n, m => m.error ? no(new Error(`${method}: ${JSON.stringify(m.error)}`)) : ok(m.result));
    browser.send(JSON.stringify({id: n, method, params, sessionId}));
  });
  const {targetId} = await send('Target.createTarget', {url: 'about:blank'});
  const {sessionId} = await send('Target.attachToTarget', {targetId, flatten: true});
  const s = (m, p) => send(m, p, sessionId);
  const {frameTree} = await s('Page.getFrameTree');
  const load = html => s('Page.setDocumentContent', {frameId: frameTree.frame.id, html});
  const value = async expr => (await s('Runtime.evaluate', {expression: expr, returnByValue: true})).result.value;
  const step = async op => {
    const [len, left, x] = await value(`(() => { const i = document.getElementById('s');
      const c = document.createElement('canvas').getContext('2d'); c.font = getComputedStyle(i).font;
      const before = c.measureText(i.value.slice(0, i.selectionStart)).width;
      return [i.value.length, i.scrollLeft, (689 + before - i.scrollLeft).toFixed(1)]; })()`);
    let caret = '-';
    for (let k = 0; k < SHOTS && caret === '-'; k++) {
      if (k) await new Promise(r => setTimeout(r, SHOT_GAP_MS));
      const shot = await s('Page.captureScreenshot', {format: 'png'});
      caret = columns(Buffer.from(shot.data, 'base64'), RED);
    }
    console.log(`${op}\t${len}\t${left}\t${x}\t${caret}`);
  };
  const wheel = async (deltaX, deltaY) => {
    await s('Input.dispatchMouseEvent', {type: 'mouseWheel', x: 800, y: 32, deltaX, deltaY});
    await new Promise(r => setTimeout(r, WHEEL_MS));
  };
  if (SCROLLBAR) return bars(s, load, value, wheel, await send('Browser.getVersion'));
  if (MODE === 'overflow') return overflows(s, load, value, wheel, await send('Browser.getVersion'));
  if (MODE === 'centre') return centres(s, load, value, await send('Browser.getVersion'));
  await load(PAGE);
  console.log(`# ${(await send('Browser.getVersion')).product}\nop\tlength\tscrollLeft\tcaret x\tcaret columns`);
  await value("document.getElementById('s').focus()");
  for (let k = 1; k <= 60; k++) {
    await s('Input.insertText', {text: 'W'});
    if ([18, 19, 20, 21, 30, 44, 59, 60].includes(k)) await step('typed');
  }
  for (let k = 59; k >= 19; k--) {
    for (const type of ['rawKeyDown', 'keyUp']) await s('Input.dispatchKeyEvent', {type, key: 'Backspace', code: 'Backspace', windowsVirtualKeyCode: 8});
    if ([59, 44, 30, 21, 20, 19].includes(k)) await step('deleted to');
  }
  await value("document.getElementById('s').value = 'W'.repeat(60); document.getElementById('s').blur()");
  await step('60 set, blur()');
  await value("document.getElementById('s').focus()");
  await step('focus()');
  await value("document.getElementById('s').blur()");
  for (const type of ['mousePressed', 'mouseReleased']) await s('Input.dispatchMouseEvent', {type, x: 800, y: 32, button: 'left', clickCount: 1});
  await step('blur(), click at x 800');
  await value("document.getElementById('s').blur()");
  await wheel(0, 100);
  await step('blurred, wheel deltaY 100');
  await wheel(100, 0);
  await step('blurred, wheel deltaX 100');
}

async function bars(s, load, value, wheel, version) {
  console.log(`# ${version.product}\ncase\top\tscrollbar-width\tscrollLeft\tgreen columns`);
  for (const [name, html] of BAR_CASES) {
    await load(html);
    await value("document.getElementById('s').focus()");
    if (name.startsWith('input')) for (let k = 1; k <= 60; k++) await s('Input.insertText', {text: 'W'});
    // a div's text starts at scrollLeft 0, so its control wheels right
    const delta = name.startsWith('div') ? 100 : -100;
    for (const op of ['typed 60', `wheel deltaX ${delta}`]) {
      if (op !== 'typed 60') await wheel(delta, 0);
      const [width, left] = await value("[getComputedStyle(document.getElementById('s')).scrollbarWidth, document.getElementById('s').scrollLeft]");
      const shot = await s('Page.captureScreenshot', {format: 'png'});
      console.log(`${name}\t${op}\t${width}\t${left}\t${columns(Buffer.from(shot.data, 'base64'), GREEN)}`);
    }
  }
}

async function centres(s, load, value, version) {
  console.log(`# ${version.product}\ncase\top\tink rows\tcaret rows\tcaret columns`);
  const at = async (name, op) => {
    let ink = '-', caret = '-', across = '-';
    for (let k = 0; k < SHOTS && caret === '-'; k++) {
      if (k) await new Promise(r => setTimeout(r, SHOT_GAP_MS));
      const png = Buffer.from((await s('Page.captureScreenshot', {format: 'png'})).data, 'base64');
      [ink, caret, across] = [rows(png, INK), rows(png, RED), columns(png, RED)];
    }
    console.log(`${name}\t${op}\t${ink}\t${caret}\t${across}`);
  };
  for (const [name, type, css, text] of CENTRE_CASES) {
    await load(`${STYLE}<style>#s{${css}}</style><input id=s type=${type} placeholder=Search>`);
    await value("document.getElementById('s').focus()");
    await at(name, 'empty, placeholder');
    await s('Input.insertText', {text});
    await at(name, `typed ${text}`);
  }
}

async function overflows(s, load, value, wheel, version) {
  console.log(`# ${version.product}\ncase\top\toverflow-x\tscrollLeft\tcaret columns\tink columns`);
  const at = async (name, op) => {
    const [x, left] = await value("[getComputedStyle(document.getElementById('s')).overflowX, document.getElementById('s').scrollLeft]");
    let caret = '-', ink = '-';
    for (let k = 0; k < SHOTS && caret === '-'; k++) {
      if (k) await new Promise(r => setTimeout(r, SHOT_GAP_MS));
      const png = Buffer.from((await s('Page.captureScreenshot', {format: 'png'})).data, 'base64');
      [caret, ink] = [columns(png, RED), columns(png, INK)];
    }
    console.log(`${name}\t${op}\t${x}\t${left}\t${caret}\t${ink}`);
  };
  for (const [name, css] of OVERFLOW_CASES) {
    await load(`${STYLE}<style>#s{${css}}</style>${INPUT}`);
    await value("document.getElementById('s').focus()");
    for (let k = 1; k <= 60; k++) await s('Input.insertText', {text: 'W'});
    await at(name, 'typed 60');
    await wheel(-100, 0);
    await at(name, 'wheel deltaX -100');
    await value("document.getElementById('s').blur()");
    await at(name, 'blur()');
    await value("document.getElementById('s').focus()");
    await at(name, 'focus()');
  }
}

main().then(() => process.exit(0), error => {
  console.error(`chromium-field-scroll: ${error.message}; rerun: node scripts/chromium-field-scroll.mjs${MODE ? ` ${MODE}` : ''}`);
  process.exit(1);
});
