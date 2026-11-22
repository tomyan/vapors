# Desktop application thread

Status: proposal, 2026-11-22. The architecture in "Decided" is Tom's
(2026-11-22); everything else is a plan, and nothing in it is built.
The open choices in section 4 come before any change to the code that
depends on them, and before design 01, design 02 or `threading.md` are
rewritten. Read from vapors main at ba482514 (T2 merged, #599).

Section 4 marks each decision as answered by Tom (AT1, AT3, AT5) or
open (AT1c, AT2, AT4, AT6, AT7). In an open decision, every option,
the proposed one included, is an alternative for Tom to choose between.

## The smallest decision that unblocks work

L1a, the application thread with no window, is authorised (Tom,
2026-11-22) and in progress. It is built on the proposed options of
AT2 (the `vapors_signals` executor driven inside Tokio) and AT4 (in
`vapors-app`). AT2 is the one genuine open choice it rests on; see
AT2 for why (a) follows from the async plan's earlier decisions. The
next decisions are AT6 and AT7, which F1 needs. None of them changes
L1a or L1b.

## 1. Decided (Tom, 2026-11-22)

- On desktop, all app-authored code runs on one dedicated application
  thread. That includes handlers and every continuation after an
  `.await`.
- That thread has Tokio for timers and IO.
- Winit owns the UI thread.
- Winit deadlines (`ControlFlow::WaitUntil`, `Next::WaitUntil`, or a
  planned `Platform::request_frame_at`) are not used to time
  background application tasks.
- Web (Tom, 2026-11-22): by default, app code, browser timers,
  network continuations and the UI all run on the browser's main
  thread. A dedicated Web Worker is optional, and is built only if
  measurements justify it. Desktop keeps the dedicated Tokio
  application thread and the winit UI thread.
- Mobile comes after desktop and web parity.
- Rendering default (Tom, 2026-11-22): frames are prepared, submitted
  and presented on the Tokio application thread, and winit handles
  window events. Submission and presentation on the UI thread stay
  available through an owned, transferable frame description, without
  a mandatory copy. No runtime toggle or second production backend is
  needed now (section 4).
- This is a chosen default awaiting measurement, not a measured
  advantage. It is judged after parity on sustained scrolling of a
  million orders, p99 input-to-visible latency, and missed frames.
  Live resize is a platform correctness check that must pass, not a
  comparison.
- The desktop application thread, its async tasks and timers come
  first, on a dedicated `current_thread` Tokio runtime (Tom,
  2026-11-22).

## 2. What this overrides

| Where | What it says today | Status now |
|---|---|---|
| `docs/design/02-scheduler-frame-threading.md`, "Threading" | The UI thread owns the graph, the tree, the scene and the GPU queue | Overridden on desktop for app-authored code. Frame preparation moves with it, and by default so do submission and presentation (section 4) |
| `docs/design/01-reactivity-async.md`, rule 6 | Futures run on the UI thread's executor in phase 2 | Overridden on desktop |
| `docs/plans/threading.md` | Keeps design 02's rule. Section 2.4 rejects moving the update off the UI thread because it costs a hop of input latency per event | Its rule is overridden. Its hand-off estimate (20 to 100 µs, not measured, section 1.3) feeds AT1 |
| `docs/plans/async-components.md` section 3.3 (T2, built) | The task wake reaches the winit thread, which polls `run_ready` | Built and correct for one thread. On desktop, task wakes move to the application thread (section 3) |
| `docs/plans/async-components.md` section 4 and T3 | Deadlines armed through `Platform::request_frame_at` and `ControlFlow::WaitUntil` | Withdrawn for tasks. Deadlines are judged on the application thread with a Tokio timer (L3) |
| `crates/gui-bench-app/src/live.rs` | The Live tick and spinner are derived from the frame clock, so the loop draws every frame while Live is on | To be replaced by a timer task on the application thread (L2) |

## 3. What T2 becomes

T2 kept two kinds of wake apart: `Wake::Frame` asks the UI thread for
a frame, and `Wake::Tasks` reaches `App::tasks_woken`, which polls the
executor and asks for a frame only when the document changed or a task
ended. The rule "a frame only when something visible changed" carries
over unchanged.

What changes is where the task wake goes. With an application thread,
the executor's wake hook wakes that thread, not winit. The UI thread
then only sees frame wakes, sent by the application thread when its
poll changed something to show. `App::tasks_woken` and `Wake::Tasks`
stay in use on the web, and on desktop until L2 moves the dashboard
over. Whether they are removed on desktop after that is decided when
L2 lands.

## 4. Decisions for Tom

**Rendering placement: the chosen default (Tom, 2026-11-22).** Frame
preparation is separated from submission and presentation by an
owned, transferable frame description (section 4a). By default the
application thread consumes its own description: it submits and
presents, and the UI thread runs winit, forwards input as owned
messages and carries out the platform's main-thread requests (IME,
clipboard, cursor, window operations, the a11y adapter). Handing the
description to the winit thread to submit and present stays
available, without a mandatory copy, because of the type boundary
rather than a second implementation. Nothing toggles between the two
at run time.

**AT1 (answered: the default above, awaiting measurement).** It is
judged after parity on sustained scrolling of a million orders, p99
input-to-visible latency and missed frames. A live resize on each
desktop must stay correct whichever thread presents; not verified
here, and this machine can only check Linux under Xvfb. Winit is
believed to require its event loop on the main thread on macOS (not
checked against winit 0.30.13 here); the default keeps winit there.

**AT1c. Only the graph and app state on the application thread.** An
alternative to the direction above, kept so it is dropped explicitly
rather than silently. It is not compatible with the default above,
which prepares frames on the application thread. DOM operations would cross as owned messages
(the `Dom` trait already has a recording implementation,
`crates/dom/src/recording.rs`), and the UI thread would own the
document, layout and paint. A handler's `preventDefault` and focus
decisions would become asynchronous, against design 03. Proposed, not
decided: drop it, since frame preparation then sits on the UI thread
and the frame description has no role.

**AT6. What the frame description owns.** (a) The painted state for
the frame, moved to the consumer. The application thread still needs
one for layout and hit testing, so this means two `Painted` values,
and the second is a real copy or a second build each frame. (b) Only what the consumer lacks: the batches and uploads
that changed, plus ids for what the consumer already holds from
earlier frames, with the painter's retained state (`KeptGpu`,
`painter.rs:33`) keyed by those ids. Proposed, not decided: (b),
because the painter already keeps state between frames, so a frame
that changes little carries little. To be measured against `--scaling`
in F1.

**AT7. Shared immutable paint data** (text layouts, images, icons;
`slab.rs:71-73`). (a) `Arc` in the slabs: transferring one is a
reference count change, with no copy of the pixels or glyph runs. (b)
Each value moved to the consumer the first time a frame names it, and
named by id after that, so the application thread no longer holds it
for drawing. Proposed, not decided: (a), as the smaller change.

**AT2. The executor on the application thread.** (a) Keep
`vapors_signals::spawn` and `run_ready` (owner-scoped, deadlines
carried), driven by the application thread's loop inside Tokio's
runtime context. The wake hook wakes that loop, for example through
`tokio::sync::Notify`, and futures use `tokio::time` because they are
polled inside the runtime. (b) Use Tokio's `LocalSet::spawn_local`
for app tasks, with owner disposal aborting through an `AbortHandle`.
Proposed, not decided: (a). It keeps T1's ownership and disposal
tests, and one executor that the web shares, while Tokio supplies only
timers and IO. L1a is built on (a), because the async plan already
places the executor in `vapors_signals` (its D6) and refuses
`spawn_local` in scripts by name (its section 10). (b) would reverse
those, so it stays open for Tom.

**AT3. The Tokio runtime's flavour.** (a) `current_thread`, so the one
application thread is the whole runtime and every continuation runs
on it. (b) `multi_thread`, with app code confined to a `LocalSet`, so
IO could complete on worker threads. Answered (Tom, 2026-11-22): (a),
a dedicated `current_thread` runtime.

**AT4. Where the application thread's code lives.** (a) `vapors-app`,
native only, beside `ElementsApp`, which it runs. (b)
`vapors-platform`, native only, beside the event loop. (c) A new
crate. Proposed, not decided: (a). Tokio stays out of the platform
crate, which the web also builds. L1a puts its code in `vapors-app`
on (a); moving it to (b) or (c) later is a file move with no change
to its behaviour.

**AT5. What L1 shows.** (a) A standalone example with no document,
which proves the thread model, the Tokio timer and idle behaviour
without the frame description or a rendering placement. (b) Straight
to the dashboard, which needs F1 first. Answered by Tom's request for
the smallest independent slice: (a), split into L1a and L1b (section
5).

The dashboard's spinner is also derived from the frame clock. Whether
an animation's clock is app-authored code that belongs on the
application thread is taken up with L2.

## 4a. The frame description and what today's draw path holds

Moving an owned value to another thread transfers ownership: a `Vec`
or `Box` moves by its pointer, and nothing it owns is deep-copied.
`threading.md` section 2.5 counts "the `DrawList` copy per frame" as a
cost. That is a cost only where a list is built fresh each frame,
which AT6 (b) avoids; handing it over is a move. What does stand in
the way is data that is shared, borrowed or tied to a thread. In
today's code, read at ba482514:

| What draw uses today | Where | What it is | For an owned, transferable description |
|---|---|---|---|
| `Painted`, borrowed for the draw | `ScenePainter::draw(&mut self, target, painted: &Painted)`, `crates/app/src/painter.rs:118` | A borrow of the document's paint result | The description owns what the consumer reads (AT6) |
| Text layouts, images, icons | `Slabs`, `crates/elements/src/slab.rs:71-73` | `Rc<Layout>`, `Rc<RgbaImage>`, `Rc<Icon>`: shared and not `Send` | AT7 |
| The document's run-time shapes | `Painted::shapes`, `crates/elements/src/painted.rs:19` | `Rc<RefCell<DocumentShapes>>`: shared, mutable, not `Send` | The changed slots taken out as owned values per frame |
| Hit-test cache | `Painted::hits`, `painted.rs` | `OnceCell<KeptHits>`, filled on the first hit test | Stays with layout and hit testing on the application thread; not in the description |
| Pipelines, glyph atlas, retained GPU buffers | `ScenePainter`, `painter.rs:19-43` | GPU objects owned by whoever draws | Owned by the consumer. wgpu's `Device` and `Queue` are `Send + Sync` natively (`threading.md` 2.5) and not `Send` on the web |
| Surface renderers | `SurfaceRenderer`, `crates/app/src/surface.rs:92` | App-authored code that encodes GPU work, sharing its scene through `Rc<RefCell<..>>` | App code, so it runs while the frame is prepared, on the application thread (section 1). When the UI thread presents, its encoded output travels in the description; whether that is a `wgpu::CommandBuffer` or owned uploads is settled in F1 |
| The surface texture, acquired and presented | `crates/platform/src/surface_texture.rs:42` | Tied to wherever presentation runs; not `Send` on the web | Acquired and presented by the consumer |
| The winit window | `crates/platform/src/handler.rs` | Winit's, on the UI thread | Stays on the UI thread in both placements |

Platform constraints that stay: the web runs on the browser's main
thread by default (section 1), and wgpu's types are not `Send` there, so the web
consumes on the preparing thread. A compile-time check asserts that the
description is `Send` on native targets only.

## 5. L1: desktop Live timer on the application thread

L1 adds the application thread and nothing else: no `ElementsApp`
change, no dashboard change, no frame description. It is split in two
so the thread, its tasks and its timers can be judged before any
window is involved.

### L1a: the application thread, no window (authorised, in progress)

**Shape.** In `vapors-app`, native only (AT4 (a)): an `AppThread` that
starts one thread running a `current_thread` Tokio runtime with its
timer enabled (AT3). On that thread it builds the app side from a
closure, so the app's state, which is not `Send`, never crosses a
thread. It takes owned messages and hands each to the app side's
handler. It installs the executor's wake hook there and drives
`vapors_signals::run_ready` inside the runtime's context (AT2 (a)), so
a task can `.await` a Tokio timer. A task's `Err` goes to a report
callback, naming its spawn site. Wakes are coalesced, so a burst of
wakes before the thread runs costs one turn. It counts its turns, so a
test can see that it is idle. Stopping it has a deadline, and a
handler that panics stops the thread and says so on the next send and
at stop. Tokio's IO driver is not enabled until a slice uses IO.

**Red: tests written first, each seen failing before green.**

1. A handler runs on the application thread, not on the sender's.
2. A task spawned by a handler continues after `tokio::time::sleep`
   on the application thread.
3. With nothing pending, the thread takes no turns over 100 ms.
4. A running 20 ms interval wakes the thread about once per tick: at
   least 5 ticks in 200 ms, and no more than one turn per tick plus one.
5. Disposing the owner of the interval's task stops the ticks and the
   turns.
6. A task's `Err` reaches the report naming its spawn site.
7. A handler that panics stops the thread: a later send is refused,
   and stop returns the panic's message.

Timings use the real clock with wide bounds, because Tokio's paused
clock advances by itself whenever the runtime is idle, so a running
interval would tick as fast as the thread can poll it.

**Mutations that must kill a test** (recorded in the commit): run the
handler on the sending thread (1); a wake hook that does not wake the
thread (2, 4); drop the coalescing (4); skip `run_ready` after a
message (2); leave a disposed task's interval polled (5); swallow a
task's `Err` (6); continue after a handler's panic (7).

### L1b: the Live timer in a window (next)

The example `live_timer`: a Live switch (a key press) on the
application thread, a 100 ms `tokio::time::interval` with
`MissedTickBehavior::Delay` (the policy of
`crates/signals/tests/tasks_interval.rs`), and the tick shown by the
UI thread from an owned value with a `Wake::Frame`. `live_timer
--check` under Xvfb records the UI thread's id in `App::frame` and the
application thread's for the handler and every continuation, and
exits 1 naming code that ran on the wrong thread. Over 3 s, Live off
gives 0 frames, 0 application-thread turns and 0 polls of the Live
task; Live on gives one frame per tick, within one tick; Live off
again gives 0 frames after the next period. Every frame returns
`Next::Wait`, and the check fails on `Next::WaitUntil`. Measured and
recorded in its PR: frames and turns for off, on and off again, and
the latency from a tick to its frame's present over 50 ticks, median
and maximum. Mutations: the handler on the UI thread; the Live task
polled through T2's path; the tick timed by `Next::WaitUntil`;
`MissedTickBehavior::Burst`; the interval left armed with Live off.

**Not in L1:** the document, the frame description, the dashboard, IO,
deadlines, the web and mobile. L1b's UI side draws from an owned
value, so AT6, AT7 and the rendering default do not change it.

## 6. After L1

| Slice | Content | Needs |
|---|---|---|
| F1 | The frame description on today's single thread: `ElementsApp` prepares an owned description and its painter consumes it on the same thread. No thread change, no toggle; `--scaling` must not regress; a native-only compile-time check that the description is `Send`; the surface renderers' output settled (section 4a) | AT6, AT7 |
| L2 | The dashboard's Live switch on the application thread: the switch's handler and the tick there, frames prepared, submitted and presented there (the default), a frame per tick, 0 frames with Live off, held to Chromium's page; a live resize checked on each desktop available | L1, F1 |
| Default judged | After parity: sustained scrolling of a million orders, p99 input-to-visible latency and missed frames, with presentation on the application thread and on the UI thread through the same frame description; a live resize correct in both | Parity, F1 |
| L3 (replaces T3) | Task deadlines judged on the application thread with a Tokio timer for the earliest deadline, the work polled before the judgement; the refusal message; no `request_frame_at` | L1a |
| Web worker (optional) | Only if measurements on the browser's main thread justify it: a Web Worker for app code, with WebGPU, input forwarding and the hop's latency measured | Measurements that justify it |
| Mobile | After desktop and web parity | L2, W1 |
