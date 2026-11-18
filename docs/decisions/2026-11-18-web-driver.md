# A vapors-owned web driver in place of winit's web backend

Date: 2026-11-18. Decided by: Tom, with the lead. Execution deferred
until property 9 holds (`docs/plans/perf-finish-line.md`). Companion to
the platforms plan (`docs/plans/platforms.md`), where phones leave
winit for thin Kotlin and Swift hosts.

## Decision

1. **The browser gets its own driver of `App`, written with `web_sys`,
   and winit's web backend leaves the web build.** The driver owns the
   frame loop, input, resize and scale; the app sees the same `App`
   trait, `Frame`, `Next` and `Event` it sees today, so no app or page
   changes.
2. **Desktop stays on winit.** Linux (X11 and Wayland), macOS and
   Windows windows, input, IME and high-DPI are what winit does well,
   and `accesskit_winit` attaches accessibility there. Replacing it
   would mean three backends for little gain.
3. **So `App` has three drivers:** winit on desktop, the web driver, and
   the phone hosts' embedding API from the platforms plan. Each driver
   translates its platform into the same events and frame contract;
   none of them branches inside the app.
4. **Nothing is built now.** Until the work starts, each new problem
   that comes from winit's web backend is filed with the `winit-web`
   label and linked here, so the case and the driver's acceptance tests
   grow together.

## Why

On the web, winit is a thin layer and vapors already works around it
more than it uses it:

- **Keys arrive only on the canvas.** winit listens for keyboard events
  on the canvas element, so a page whose focus lands on `<body>` hears
  no keys. `text_bridge_web.rs` hands focus back to the canvas after a
  text field closes, and a further slice (web focus, 2026-11-18) does
  the same at start-up and after clicks on page elements. A driver can
  listen where the page needs it.
- **Text input and accessibility are already vapors' own.** The hidden
  textarea bridge (`text_bridge_web.rs`, `text_bridge_web_view.rs`) and
  the DOM accessibility mirror (`a11y_mirror_web.rs`) are `web_sys`
  code; winit's `Ime` events are not used on the web.
- **Other workarounds sit beside winit:** the wheel position read by a
  capture-phase listener on the window before winit's own
  (`wheel_position_web.rs`), the page timer for deadlines that must end
  without a frame (`timer_web.rs`), and the click fallback by browser
  platform (`clicks_web.rs`).
- **Frame timing.** winit does not pass on the `requestAnimationFrame`
  timestamp, so `Frame::now` is a separate `performance.now()` read
  (`window.rs`), and `Next::WaitUntil` runs on a timer outside the
  animation-frame loop. A driver runs the loop on
  `requestAnimationFrame` itself and gives the frame its timestamp.
- **Canvas sizing.** #510: at `devicePixelRatio` 2 in headless Chromium
  153 the canvas was backed at its CSS size while touches came in
  device pixels. The cause is likely winit's canvas sizing; it is not
  confirmed. A driver sizes the backing from a `ResizeObserver`'s
  device-pixel box, where the browser gives it.

## What the driver covers

- The loop: `requestAnimationFrame` while the app returns `Continue` or
  something asked for a redraw; nothing scheduled while it returns
  `Wait` (property 6: idle draws nothing); `WaitUntil` as one timer that
  requests a frame, cleared when the next frame's answer changes it.
  The timestamp `requestAnimationFrame` gives is `Frame::now`.
- Input: pointer events (mouse, pen, touch through one path), wheel,
  keys and focus, with listeners on the canvas or the window as each
  event needs; keys reach the app whatever element of the page holds
  focus, except a focusable page element that owns them (an `<input>`
  the page has).
- Resize and scale: a `ResizeObserver` on the canvas with the
  device-pixel content box, the backing size set from it, and
  `devicePixelRatio` changes (zoom, moving between screens) as
  `ScaleFactorChanged` (#120 is the native half).
- Lifecycle: `visibilitychange` (a hidden page draws nothing), and the
  `webgpu.rs` surface set-up moved as it is.
- It keeps what is already vapors' own: the text bridge, the
  accessibility mirror, the picker, the page contract
  (`docs/web.md`).

## Acceptance

- Every check the web build has today passes unchanged:
  `scripts/check-page-contract.sh`, the screenshot and pixel checks,
  touche's `check_idle`, `check_zoom_status`, `check_dev_keys` and
  `check_desktop`.
- An idle page asks for no frames (property 6's `--scaling Idle`
  measured in the browser where the harness allows).
- Keys reach the app with focus on `<body>`; the canvas backing equals
  its CSS size times `devicePixelRatio` at 1, 2 and 3; a tap at a CSS
  point lands at that point times the scale (#510's case).
- `Frame::now` equals the `requestAnimationFrame` timestamp of its
  frame.
- The wasm size does not grow (winit's web backend leaves the build).
- Each `winit-web` issue filed by then has a test that fails on the old
  driver, or a note saying why it cannot be tested.

## Order

After property 9 holds, and after the `agent-friction` fixes in flight.
The driver is written beside the winit path behind the web build's
choice of driver, switched over once every acceptance check passes on
both, and the winit web path is then removed in its own PR.

## Risks

- **Browser differences** winit has absorbed (Safari's pointer and
  keyboard quirks, Firefox's wheel deltas) come back to vapors. The
  check pages run in Chromium only today; Safari 26 and Firefox 141 need
  a manual pass before the switch.
- **Two input translations to keep equal** (winit's and the driver's)
  until the switch. The acceptance checks run on both.
