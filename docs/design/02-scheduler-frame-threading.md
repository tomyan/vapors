# 2. Scheduler, frame loop and threading

Status: design, 2026-10-25. The platform has a frame loop with redraw
on demand (`Next::Wait`), a `Waker` for work started elsewhere, and
measured idle frames of zero. This page adds the order of work inside a
frame, the task queue that area 1's futures run on, animation ticks
(area 6), the frame budget, and the threading rule, including the
web-only render-in-worker option.

## The frame

One frame, on the UI thread, in this order:

1. **Input.** Every platform event since the last frame is dispatched
   (area 3). Handlers write signals; effects run at once, as today.
2. **Tasks.** The executor polls every future that was woken since the
   last frame, until none is ready. Resolved futures write signals; a
   pending version that settles flushes its queued effects (area 1).
3. **Animation tick.** Running transitions and animations are sampled
   at this frame's time and written as used values (area 6). A tick
   that reads signals is a bug; it only writes computed values. Scene
   animations (area 9) run their bulk tasks here.
4. **Update.** `Document::update`: restyle, layout, paint to a `Scene`.
   An update that moves content under a still pointer re-hovers and
   asks for another frame (F2c D7).
5. **Accessibility.** The AccessKit tree update is diffed from the
   element tree and pushed (area 5).
6. **Draw.** The painter submits the scene. The frame is presented.
7. **Record.** Area 10 writes the frame's snapshot delta and timings.
8. **Next.** `Wait` unless something asked for a frame: a signal
   changed during the update, a transition is running, a future is
   ready, or a scene animation is active.

Why this order: input before tasks so a click's effects are visible in
the same frame as a future that resolved just before it; tasks before
the tick so a resolved future can start a transition that is sampled
at its first frame; the tick before the update so sampled values are
laid out, not applied a frame late. Chromium's event loop has the same
shape (input, then tasks and microtasks, then `requestAnimationFrame`
callbacks, then style and layout, then paint), and the ordering tests
below can be written against it.

## Time

One clock, `platform::Instant`, read once per frame and handed to
every phase as `frame.now`. A transition started by a handler in phase
1 uses the frame's time, not the wall clock at the write, so two
transitions started in one frame are in step. Tests supply the clock,
so an animation test steps time by hand and asserts sampled values; no
test sleeps.

The web's `requestAnimationFrame` timestamp is the frame's time there;
natively it is `Instant::now()` at the start of the frame. The
`stress` demo's measured update time is reported by this clock.

## Redraw on demand, as built

`Next::Wait` draws again only when asked: `Platform::request_redraw`
from inside `event`, `Waker::wake` from outside, or the window
(resize, scale, shown). The idle measurement is zero frames in three
seconds on both targets, held by `idle_fps` and `web-idle-frames.sh`.
This page keeps that rule and adds the three new reasons to ask: a
ready future, a running animation, a scene with an active animation.
Each is one `request_redraw`, so the frame cost of an idle app with a
pending future is still zero.

## The budget

The brief sets 4 ms for the framework's own work per frame on the
development machine, to be replaced by a measurement. As built, S7
measured 2.6 to 3.1 ms for a 1,000-element update on this box's CPU.
The budget is split per phase, each measured by `tracing` spans and
shown in the debug overlay (area 11): input and effects, tasks, tick,
restyle, layout, paint, a11y, draw. A phase over its budget is a
report in the overlay and a line in the recording (area 10), not a
refusal: dropping a frame is a defect to measure, not an error to
stop on. The one exception is a `Document::update` that fails, which
stops the loop today and continues to.

Phone numbers come from the Pixel through area 10's recordings, not
from this box.

## Threading

**The rule: the UI thread owns the graph, the tree, the scene and the
GPU queue's submission order.** Nothing else touches them. Everything
else is a worker that takes owned data and returns owned data through
`offload` (area 1) or a channel the platform provides, and wakes the
loop through `Waker`.

| Work | Where | How it reaches the UI thread |
|---|---|---|
| Decode, encode, hashing, measurement maths on the CPU | A thread pool natively (rayon or a fixed pool sized to the cores, decided by measurement); a worker pool on the web | `offload`'s future; the result is a signal write in phase 2 |
| GPU passes (the engine's render graph) | Submitted from the UI thread on one device (ARCHITECTURE.md); natively the engine may build command buffers on its own thread and hand them over, since wgpu's device is `Send + Sync` there | A "texture N ready for version V" message; the canvas element draws the latest ready texture and never waits |
| Network | The platform's transport (tokio natively, `fetch` on the web) | A future |
| File dialogs, keychain | A platform thread natively; a promise on the web | A future, or the existing event |
| Font loading | The UI thread today; a worker when the web fallback fetch arrives (F6) | A future that registers the font then asks for a frame |

**Web: no shared memory by default.** Workers get data by structured
clone or transferred `ArrayBuffer`s. Shared wasm memory
(wasm-bindgen-rayon) needs the cross-origin isolation headers (COOP
`same-origin`, COEP `require-corp`), which the tailnet server and a
static host can set but a third-party embed cannot, and as of
2026-10-25 a nightly toolchain with `build-std`, `+atomics` and
`+bulk-memory`; wgpu's `fragile-send-sync-non-atomic-wasm` feature is
off under atomics. The toolchain cost is why the shared path is a
separate, optional slice. touché's page is first-party, so the headers
are a start-up option checked at load, and `offload` uses shared memory
when they are present and the build has it, and copies otherwise. The
choice is reported once in the status line and recorded (area 10).

## Render in a worker (web only)

A start-up option, built only if area 10's timings show main-thread
frame drops (Tom, 2026-10-24). Shape, so the frame loop is ready for
it:

- The page's main thread keeps input, the DOM shims (clipboard, IME,
  pickers, the a11y mirror) and the `<canvas>` element. It transfers
  the canvas to a worker with `transferControlToOffscreen`.
- The worker runs the whole frame above (graph, tree, scene, GPU) on
  its own `GPUDevice` from `navigator.gpu` in the worker. The device
  is created where it is used; a `GPUDevice`, buffer or texture cannot
  be posted or transferred between threads (gpuweb issue 354, deferred),
  which is why there is one device per thread of GPU work and so, since
  the whole frame moves, still one device. Worker WebGPU as of
  2026-10-25: Chrome 113+ (Linux from 144), Safari 26, Firefox 141+ on
  Windows and Apple-silicon macOS only; a WebGPU context on a
  transferred `OffscreenCanvas` has no per-browser compatibility entry
  and is checked by the page contract before the option is offered.
- Input events are forwarded as messages with the main thread's
  timestamp; IME and clipboard results likewise. Latency of the hop is
  measured and recorded.
- Platform services that need the main thread (area 7) already answer
  through futures, so app code does not change.

What it costs: every a11y update and every cursor change crosses the
hop too; the a11y mirror in the DOM is driven by messages. What it
does not change: app code, which sees the same `Platform` and the same
frame order.

## Taken from where

- **Frame phases and one clock per frame:** the HTML event loop's
  processing model; GPUI's frame (input, effects, layout, paint) has
  the same shape without a task phase.
- **Redraw on demand with a waker:** winit's `ControlFlow::Wait` and
  `EventLoopProxy`, which is what `Waker` wraps.
- **Owned data across threads:** Rust; and Bevy's rule that systems
  exchange components, not references. Bevy's multi-threaded scheduler
  is not taken: the graph is single-threaded by design (area 1).
- **Render in a worker:** the OffscreenCanvas pattern Chrome documents;
  Flutter's web renderer runs it; Makepad does not.

## Deliberately left out

- A multi-threaded reactive runtime (Leptos's `Send` signals). The
  cost would be a lock on every read.
- Damage tracking and partial redraw. The whole window is redrawn each
  frame; the scene is kept per subtree. Revisited when a measured frame
  is over budget for paint alone.
- A separate render thread natively. One thread submits; the engine
  may encode on its own thread and hand command buffers across.
- Frame pacing beyond vsync (triple buffering control, frame prediction).

## Oracles

| Part | Oracle |
|---|---|
| Phase order | A test that records the order of a handler's write, a resolved future, a transition's first sample and the update, and pins it; the same scenario in Chromium (a click handler, a resolved promise, a `transitionstart`, a `requestAnimationFrame`) recorded through the page-contract script as the reference order |
| Redraw on demand | `idle_fps` and `web-idle-frames.sh` (built): zero frames idle with a pending but unresolved future, and exactly one per resolution |
| Time | A stepped clock in tests; a transition sampled at the frame time, not the write time, pinned by a test that writes at two wall-clock times in one frame |
| Budget | `tracing` spans per phase, asserted under a stated ceiling in the `gpu` tier on this machine; the phone's numbers from recordings |
| Threading | A test that offloads work holding a `Cancel` token and drops the future; the worker's step count is pinned. Compile-time: the graph's types are `!Send`, and a test asserts it with a `static_assertions` negative impl check |
| Render in a worker | The page contract loads the page with `?worker=1` and checks the same pixels; input latency across the hop measured and recorded |

## Slices

| Slice | Delivers | touché use |
|---|---|---|
| S1 phases and clock | The frame order as a function, `frame.now`, spans per phase, the debug overlay line | The stress demo shows per-phase time; touché's P0 gets frame numbers from the same spans |
| S2 executor | The task queue, waking through `Waker`, drop to cancel | Area 1's R2 |
| S3 tick | The animation phase hook | Area 6's A1 |
| S4 offload | Native pool; web pool with copies | Export encoding and decode off the UI thread on both targets |
| S5 shared memory | The isolation headers checked at load; `SharedArrayBuffer` path | A 24 MP decode without a copy |
| S6 render in a worker | The start-up option | Only if P0 shows the need |
