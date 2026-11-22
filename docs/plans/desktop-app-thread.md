# Desktop application thread

Status: proposal, 2026-11-22. The architecture in "Decided" is Tom's
(2026-11-22); everything else is a plan, and nothing in it is built.
The open choices in section 4 come before any change to how the code
is structured, and before design 01, design 02 or `threading.md` are
rewritten. Read from vapors main at ba482514 (T2 merged, #599).

Every option in section 4, the proposed one included, is an
alternative for Tom to choose between; none is decided.

## The smallest decision that unblocks work

Starting L1 (section 5) needs AT2 to AT5 and does not need AT1. The
smallest decision is one answer: whether L1 may start on the proposed
options, (a) for each of AT2 to AT5. Any other option on any of them
changes L1's shape, and section 5 is revised before it starts. AT1,
what crosses between the threads, is the larger decision; it blocks
L2 and the dashboard and can wait for L1's measurements.

## 1. Decided (Tom, 2026-11-22)

- On desktop, all app-authored code runs on one dedicated application
  thread. That includes handlers and every continuation after an
  `.await`.
- That thread has Tokio for timers and IO.
- Winit owns the UI thread.
- Winit deadlines (`ControlFlow::WaitUntil`, `Next::WaitUntil`, or a
  planned `Platform::request_frame_at`) are not used to time
  background application tasks.
- Web: a Web Worker as the application thread is proposed and not
  validated.
- Mobile comes after desktop and web parity.

## 2. What this overrides

| Where | What it says today | Status now |
|---|---|---|
| `docs/design/02-scheduler-frame-threading.md`, "Threading" | The UI thread owns the graph, the tree, the scene and the GPU queue | Overridden on desktop for app-authored code. What else moves depends on AT1 |
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
over. Whether they are removed after that is decided with AT1.

## 4. Decisions for Tom

**AT1. What crosses between the two threads.** The decision says where
app code runs. It does not say where the document, layout, paint and
GPU work run, and every later slice depends on that.

- (a) **Everything but winit on the application thread.** The graph,
  the document, layout, paint and GPU submission all move. The UI
  thread forwards input as owned messages and carries out the
  platform's main-thread requests (IME, clipboard, cursor, window
  operations, the a11y adapter). This is the closest to today's code:
  `ElementsApp` moves as one unit, and design 02's single owner
  remains, on a different thread. Risks, not verified: wgpu surface
  present and reconfigure from a thread that is not winit's, during a
  live resize on macOS and Windows. Winit is believed to require its
  event loop on the main thread on macOS (not checked against winit
  0.30.13 here). If so, that fits this option, since winit keeps the
  UI thread.
- (b) **Graph and document on the application thread; GPU on the UI
  thread.** The application thread sends an owned draw list per frame,
  which is `threading.md` section 2.5's render-thread shape. This
  needs the paint slabs changed from `Rc` to `Arc`, and a `DrawList`
  copy every frame.
- (c) **Only the graph and app state on the application thread.** DOM
  operations cross as owned messages (the `Dom` trait already has a
  recording implementation, `crates/dom/src/recording.rs`), and the UI
  thread owns the document, layout and paint. This changes event
  semantics: a handler's `preventDefault` and focus decisions become
  asynchronous, against design 03.

Proposed, not decided: (a), with a spike before any slice relies on it. The
spike would measure the input-to-present latency over the hop and
exercise a live resize. I am not confident about the resize behaviour
on macOS and Windows; this machine can only check Linux under Xvfb.

**AT2. The executor on the application thread.** (a) Keep
`vapors_signals::spawn` and `run_ready` (owner-scoped, deadlines
carried), driven by the application thread's loop inside Tokio's
runtime context. The wake hook wakes that loop, for example through
`tokio::sync::Notify`, and futures use `tokio::time` because they are
polled inside the runtime. (b) Use Tokio's `LocalSet::spawn_local`
for app tasks, with owner disposal aborting through an `AbortHandle`.
Proposed, not decided: (a). It keeps T1's ownership and disposal tests, and
it keeps one executor that the web shares, while Tokio supplies only
timers and IO.

**AT3. The Tokio runtime's flavour.** (a) `current_thread`, so the one
application thread is the whole runtime and every continuation runs
on it. (b) `multi_thread`, with app code confined to a `LocalSet`, so
IO could complete on worker threads. Proposed, not decided: (a). It is the
literal reading of "a dedicated application thread", and IO can move
off later by message.

**AT4. Where the application thread's code lives.** (a) `vapors-app`,
native only, beside `ElementsApp`, which it runs. (b)
`vapors-platform`, native only, beside the event loop. (c) A new
crate. Proposed, not decided: (a). Tokio stays out of the platform crate,
which the web also builds.

**AT5. What L1 shows.** (a) A standalone example with no document,
which proves the thread model, the Tokio timer and idle behaviour
before AT1 is settled. (b) Straight to the dashboard, which needs AT1
first. Proposed, not decided: (a). It is the only slice that can start
before AT1 is answered.

The dashboard's spinner is also derived from the frame clock. Whether
an animation's clock is app-authored code that belongs on the
application thread is taken up with L2.

## 5. L1: desktop Live timer on the application thread (proposed)

The smallest slice: one example, one Tokio interval, and checks that
pin thread identity and idle behaviour. It adds the application
thread and nothing else: no `ElementsApp` change, no dashboard change.

**Shape.** A native-only `run_with_app_thread(options, app_side,
ui_side)` (name to be chosen), placed where AT4 decides. It starts one
application thread running a `current_thread` Tokio runtime with time
enabled (AT3), which drives `vapors_signals`' executor (AT2). Winit runs
on the calling thread. UI input reaches the application thread as an
owned message. The application thread sends the UI thread an owned
value plus a `Wake::Frame` when there is something to show. The
example, `live_timer`, has a Live switch (a key press) and shows the
tick: off at start, ticking every 100 ms while on.

**Red: acceptance tests written first, each failing before green.**

1. **Thread identity, unit.** With no window, start the application
   thread and send it a toggle. The handler and every continuation
   after `interval.tick().await` record `std::thread::current().id()`.
   All recorded ids are the application thread's, and none is the test
   thread's.
2. **Tokio timing, unit.** With a paused Tokio clock (`tokio::time::
   pause` and `advance`, from the `test-util` feature, dev only):
   advancing 100 ms gives exactly one tick. A 350 ms stall gives one
   tick showing the current value and the next tick a full period
   later (`MissedTickBehavior::Delay`), the policy proven in
   `crates/signals/tests/tasks_interval.rs`. Turning the switch off
   cancels the interval: no tick after any later advance, and the task
   is not polled again.
3. **Thread identity and frames, in a window (Xvfb).** `live_timer
   --check` records the UI thread's id in `App::frame` and the
   application thread's ids as in test 1, and exits 1 naming the code
   that ran on the wrong thread. Over 3 s, Live off gives 0 frames, 0
   application-thread wakes and 0 polls of the Live task. Live on gives
   one frame per tick, within one tick. Live off again gives 0 frames
   after the next period.
4. **No winit deadline.** While Live is on, every frame returns
   `Next::Wait`. The UI thread is woken only by the application
   thread's `Wake::Frame`; `live_timer --check` fails if a frame
   returns `Next::WaitUntil`.

**Green.** The application thread, its runtime and the two message
paths: the minimum that makes tests 1 to 4 pass.

**Mutations that must kill a test** (each to be recorded in the
commit):
- Run the handler on the UI thread: tests 1 and 3.
- Poll the Live task on the UI thread through T2's path: test 3.
- Time the tick with `Next::WaitUntil`: test 4.
- `MissedTickBehavior::Burst`: test 2.
- Leave the interval armed when Live is off: tests 2 and 3.

**Measured and recorded in the PR:** frames and wakes over 3 s for
off, on and off again, and the latency from a tick to its frame's
present over 50 ticks, median and maximum, under Xvfb with llvmpipe.

**Not in L1:** the document, the dashboard, IO, deadlines, the web and
mobile.

## 6. After L1

| Slice | Content | Needs |
|---|---|---|
| AT1 spike | Input-to-present latency over the hop, and a live resize, for AT1's chosen option | AT1 |
| L2 | The dashboard's Live switch on the application thread: the switch's handler and the tick there, a frame per tick, 0 frames with Live off, held to Chromium's page | L1, AT1, AT2 |
| L3 (replaces T3) | Task deadlines judged on the application thread with a Tokio timer for the earliest deadline, the work polled before the judgement; the refusal message; no `request_frame_at` | L1, AT2 |
| W0 | A Web Worker as the application thread: a spike that validates or rejects it (WebGPU, input forwarding, the hop's latency) | L1 |
| Mobile | After desktop and web parity | L2, W0 |
