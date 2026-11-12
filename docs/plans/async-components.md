# Async in `.wisp` components

Scheduled after the performance finish line
(`docs/plans/perf-finish-line.md`): no row here starts until every
property there holds, measured, unless Tom moves it. Plan only; nothing
here is built. Read from the code at 90d1c8a (vapors main, 2026-11-12)
and touche main at ea2bb08. Line counts per PR are estimates. No build
was run for this plan.

What this plan adds to `docs/design/01-reactivity-async.md` (2026-10-25,
decided with `QUESTIONS.md` Q3, Q5 and Q6) and
`docs/design/02-scheduler-frame-threading.md`: the shapes as code
(`spawn`, the executor, `{await}`, `Resource`), what each costs idle, a
deadline on every task, what the compiler sees, and a PR stack. Where
this plan departs from those pages it says so (section 14, D1 and D7).

Binding rules, from the brief for this plan:

- No API whose effect the compiler's analysis cannot see
  (`docs/decisions/2026-11-11-analysable-sources.md`).
- Generated code calls a small shared runtime, never a per-use copy
  (as `vapors_dom::blocks::conditional` serves every `{if}`,
  `crates/dom/src/blocks/conditional.rs`).
- Idle does nothing: a pending future draws no frames and polls on no
  timer (`perf-finish-line.md` property 6; `idle_fps` and
  `scripts/web-idle-frames.sh`).
- Every failure loud: an error from an await is shown or refused,
  never swallowed.
- No `unsafe`.

## 1. Where things stand

Decided (design 01, 02, `QUESTIONS.md`):

- `await` in three places: the script's top level, a template hole,
  and an async memo (01, "1. `await` in three places").
- The narrow held-update rule, Q3 (a): a write that restarts an async
  derivation holds that derivation and its downstream; synchronous
  readers apply at once (01, rule 2).
- Drop to cancel, plus a `Cancel` token for off-thread work (01, rule
  3; Q5 (a) for `offload`).
- A `.get()` after an `await` in a tracked scope is refused by the
  compiler, Q6 (a).
- `<boundary>` with `pending` and `failed` snippets (01, rule 5).
- Futures run on the UI thread's executor in frame phase 2, "Tasks",
  woken through the platform `Waker` (01, rule 6; 02, "The frame").
- Every wait that is not the user's has a deadline from a
  measurement, named in the code (`docs/design/07-platform-services.md`,
  "Deadlines").

Built:

- The frame's phases, with `Phase::Tasks` a no-op:
  `crates/app/src/frame.rs` (`Phases::tasks` has a default empty body;
  `crates/app/src/elements_app_frame.rs` does not override it).
- Redraw on demand: `Next::Wait`, `Next::WaitUntil(Instant)`
  (`crates/platform/src/window.rs`), `Platform::request_redraw` and
  `Platform::waker` (`crates/platform/src/platform.rs`). A `Waker::wake`
  posts `Message::Wake`; the handler turns it into a redraw request
  (`crates/platform/src/handler_events.rs:57`,
  `crates/platform/src/handler.rs` `redraw_if_asked`). Every wake draws
  one frame today.
- A `std::task::Waker` over the platform waker, for the GPU error
  scopes: `crates/app/src/elements_app_scope_wake.rs` (`LoopWake`,
  wrapped in `send_wrapper` on the web because the platform waker is
  not `Send` there). The scope watch on the web runs as a
  `wasm_bindgen_futures::spawn_local` task racing a page timer
  (`crates/app/src/painter_surface_scope_watch.rs`,
  `crates/platform/src/timer_web.rs` `Timer`); natively the scope is
  polled once with a noop waker and resolves on pop
  (`painter_surface_scope_watch.rs` `Watched::new`).
- A deadline with a visible-time rule for those scopes:
  `crates/app/src/painter_surface_scope.rs` `DEADLINE` (1000 ms) and
  `painter_surface_scope_deadline.rs`.
- The signal graph: one thread-local runtime, `Owner`, `on_cleanup`,
  `Effect`, `Memo` (`crates/signals/src/`). No async node, no context
  on an owner.
- Snippets with memo parameters, `{render expr()}` through
  `vapors_dom::blocks::dynamic` (R1a to R1d; `docs/components.md`).
- Async dependencies in vapors: `pollster` (workspace, for setup and
  tests), `wasm-bindgen-futures` (platform and app). No `tokio`, no
  `futures`.

Not built: 2-S2 executor, 1-R2 async memo, 1-R3 held updates, 1-R4
`await` in script and template, 1-R5 `<boundary>`, 1-R6 `offload`,
11-Q3's manual executor. `ORDER.md` placed 2-S2 and 1-R2 at step 3.

## 2. The use cases, from touche

Every async thing touche does today, and what it would become:

| touche today | Where | Becomes |
|---|---|---|
| A handler starts work; the result lands in a mailbox; the mailbox wakes the loop; the next frame takes it | `crates/web/src/page_app.rs` (`spawn_local` at lines 69 and 152), `ask_mailbox.rs` (`post_pixels` calls `wake::wake()`), `wake.rs` | `spawn` from the handler; the task writes a signal; the executor wakes the loop (section 3) |
| The profile in IndexedDB, read before the learned-scales sheet can show | `crates/web/src/profile_web.rs` `load`, `profile_sheet.rs` `touche_profile_summary` | `{await profile::load()}` with a pending branch (section 5). Today that read has no deadline; it gets one (section 4) |
| A model turn: a stream of chunks over `fetch` or a websocket, with a deadline per wait | `crates/runtime/src/timer.rs` `within` (tokio's sleep natively, `gloo_timers` on the web), `agent.rs` `DEFAULT_DEADLINE` 300 s | A task spawned with a deadline; each chunk is a signal write, as design 01 "Deliberately left out" says |
| A GPU readback with a measured deadline | `crates/engine/src/readback.rs` `MEASURED_READBACK` 110 ms, `READBACK_DEADLINE` 15 s | A task; the deadline moves to the `spawn` |
| Export: encode in a worker, then save | `crates/web/src/busy.rs`, `export_pipeline.rs` | A task now; `offload` (1-R6) later for the encoder |
| Native CLI commands block on futures | `pollster::block_on` in `crates/app/src/measure_run.rs`, `engine_blocking.rs`; the agent loop on a tokio current-thread runtime, `crates/runtime/src/agent.rs` `run_blocking` | Unchanged: no window, no frame loop, so no executor of ours |

The first two rows are the whole of what a component needs: start work
from a handler and show a value that is not there yet. Neither needs a
top-level `await` or a boundary; section 14 D1 recommends building
those last.

## 3. Tasks: `spawn`, the executor, waking, idle

### 3.1 The API

```rust
// in a .wisp <script>
let answer: Signal<Option<String>> = Signal::new(None);
let ask = move |_: &mut Event| {
    spawn(Deadline::Measured(ASK_DEADLINE), async move {
        let text = model::ask(prompt.get()).await?;
        answer.set(Some(text));
        Ok(())
    })
};
```

- `spawn(deadline, future)` in `vapors_signals` (the graph's owner is
  what a task belongs to; see 3.4 for why not a new crate). The future
  is `Future<Output = Result<(), E>> + 'static` with `E: Display`, the
  same contract as a handler (`docs/components.md`, "Events"). It is
  not `Send`: it is polled on the UI thread only.
- It returns a `Task` handle. Dropping the handle does not stop the
  task (as dropping an `Effect` does not, `crates/signals/src/effect.rs`);
  the owner current at `spawn` does, by disposal, and `Task::cancel`
  does explicitly.
- `deadline` is section 4's.

### 3.2 The executor

One thread-local executor beside the signal runtime
(`crates/signals/src/runtime.rs` has the pattern: a `RefCell` in a
`thread_local!`, borrowed briefly, released before user code runs).

- Tasks: an arena of `Pin<Box<dyn Future<Output = ()>>>` (the
  `Result` is consumed by the task's own wrapper, section 8), each with
  its owner, its deadline, its name (the `spawn` call site, through
  `#[track_caller]` as `Effect::new` does) and a `woken` flag.
- Ready list: `Arc<Mutex<Vec<TaskId>>>`. A task's `std::task::Waker`
  pushes its id and calls the wake hook. The mutex is held for one
  push or one drain; it is not on the signal graph, which stays
  lock-free. Natively a waker may be called from another thread (a
  channel's receiver, wgpu's map callback), which is why it is a mutex
  and not a `RefCell`. On the web there is one thread, and the
  `Arc<Mutex>` costs nothing it would not cost natively; `send_wrapper`
  is not needed for the list, only for the platform waker inside the
  hook, as `elements_app_scope_wake.rs` already does.
- Wake hook: `set_wake_hook(impl Fn() + Send + Sync)`, installed once
  by `ElementsApp` with the platform waker (as it installs the error
  report, `crates/app/src/elements_app.rs:92`). A wake with no hook
  installed is a refusal naming `set_wake_hook`, the same shape as a
  listener error with no report (`crates/dom/src/listener_result.rs`).
- `run_ready(now) -> Ran`: drains the ready list, polls each listed
  task once with its waker, with the signal runtime's observer stack
  empty (so a `.get()` inside a future subscribes nothing; section
  10). A task that completes is removed and its owner's registration
  dropped. Returns how many were polled and how many completed, for
  the frame's tasks line.
- `run_ready` refuses to run inside a batch or a flush
  (`Runtime::batch_depth`, `flushing`): a task polled from inside an
  effect would write signals mid-flush. The refusal names the effect
  being computed (`Runtime::computing`).

### 3.3 Waking and idle

Today a `Message::Wake` draws a frame
(`crates/platform/src/handler_events.rs:57`). With tasks that is one
frame per wake even when the poll changes nothing (a future that
wakes to find its channel still empty). The rule "idle does nothing"
needs the poll before the frame:

1. A future's waker pushes the task and calls the hook; the hook
   calls `Platform`'s `Waker::wake`, which posts `Message::Wake`.
2. The handler delivers it as a new `App` method, `fn woken(&mut self,
   platform: &Platform)`, default body `platform.request_redraw()` (what
   happens today, so every existing app is unchanged). `ElementsApp`
   overrides it: `run_ready(now)`, then `request_redraw` only if
   `Document::take_changed()` (`crates/elements/src/document.rs:112`)
   says the tree changed, or a task completed or failed (its refusal
   is shown by a frame).
3. The frame's `Phase::Tasks` (`crates/app/src/frame.rs`) does not poll;
   it reports the time polls took since the last frame, as
   `Phases::input_time` does for handlers (2-S1 dispatches input on
   arrival, not at the frame's start; tasks follow that precedent).

Cost of a pending future: zero frames, zero polls, until its waker
fires. Cost of a wake: one poll, and a frame only when something to
show changed. The oracles in section 12 pin both.

Why not `wasm_bindgen_futures::spawn_local` on the web, as touche and
the scope watch do: that is the browser's executor, and native has no
counterpart, so there would be two executors with two orders of
events. One executor of ours, driven by the loop on both targets, is
the smaller thing to reason about; `JsFuture` still works inside it
(its promise callback calls our waker, which posts through winit's
proxy: one macrotask hop, measured in W1).

### 3.4 Where the code lives

`vapors_signals` gains `task.rs` (spawn, `Task`, the arena),
`executor.rs` (ready list, `run_ready`, the hook) and `deadline.rs`.
The graph crate is the right place because a task is owned by an
`Owner` and disposed with it, and `run_ready` must read the runtime's
batch and flush state. The crate depends on `std` only; no `futures`,
no `tokio`. `vapors_dom::blocks` gains `awaiting.rs` for section 5;
the compiler gains the `{await}` node and the `Resource` field kind.

## 4. Deadlines

Tom's rule (`07-platform-services.md`, "Deadlines", and touche's
`runtime/timer.rs`): every wait that is not the user's has a deadline
from a measurement; the user's own waits have none and say so.

```rust
pub enum Deadline {
    /// The task is refused at `spawn + d`; `d` from a measurement
    /// named where the constant is declared.
    Measured(Duration),
    /// The user decides when it ends (a picker, a permission prompt).
    /// Declared here, where it runs, as the relaxed guard it is.
    User,
}
```

- The executor keeps each task's deadline as an `Instant`. The
  earliest is `vapors_signals::earliest_deadline()`.
- Judging: when a task is polled and is still pending, and `now` is
  past its deadline, it is dropped and refused: "task spawned at
  `file:line` did not finish within `d`; ..." through the error report
  (section 8). The poll comes first, so work that is done at the
  deadline wins, touche's `within` rule (`runtime/timer.rs`: "The work
  is polled first, so work that is already done wins over a zero
  limit").
- Arming: a deadline needs a frame at its time if nothing else
  happens. `ElementsApp::next_after_frame`
  (`crates/app/src/elements_app_decision.rs`) already returns
  `Next::WaitUntil` for the tooltip's deadline; it takes the earlier of
  that and `earliest_deadline()`. A task spawned from a handler that
  changed nothing has no frame to return `WaitUntil` from, so the
  platform gains `Platform::request_frame_at(Instant)`, taken by the
  handler after each event beside `take_redraw_request`
  (`handler.rs` `redraw_if_asked`), applied as
  `ControlFlow::WaitUntil`. On the web that is winit's timer, as
  `Next::WaitUntil` is today (`window.rs` doc on `WaitUntil`).
- The frame at the deadline runs `run_ready` first (the task may have
  been woken but not yet polled), then judges.
- A hidden page draws no frames and throttles timers
  (`timer_web.rs` module doc). The GPU scope's deadline counts visible
  time only because a hidden page cannot resolve a scope
  (`painter_surface_scope_deadline.rs`). A task's work (a fetch, a
  worker) goes on while hidden, so a task's deadline is wall time,
  judged at the first frame after; the message says the time it was
  judged at. D4 in section 14.
- Numbers: each use names its own. From touche: `ASK` 300 s
  (`agent.rs` `DEFAULT_DEADLINE`), readback 15 s with 110 ms measured
  (`readback.rs`), dev keys 5 s (`ask_dev_keys.rs`), IndexedDB
  unmeasured today (`profile_web.rs` has no deadline): measured in the
  first PR that uses it.

## 5. `{await}` in the template

```html
{await profile::load()}
  <Skeleton rows={3} />
{then profile}
  {for scale in profile.scales key=scale.id}<ScaleRow {scale} />{/for}
{catch error}
  <ErrorRow message={error.to_string()} />
{/await}
```

Spelling: wisp's blocks have no sigils (`{if}`, `{else}`, `{/if}`;
`docs/components.md`), so `{await expr}`, `{then name}`, `{catch
name}`, `{/await}`. Svelte's shorthands (`{#await p then v}`) are not
taken in the first slice.

Semantics:

- `expr` is a Rust expression whose value is `Future<Output =
  Result<T, E>> + 'static`, `E: Display`. Always a `Result`: one shape,
  no trait that hides which branch a value takes (D2).
- The expression is evaluated in the block's effect, tracked, as an
  `{if}` condition is (`blocks/conditional.rs`). When a signal it read
  changes, the future in flight is dropped (01, rule 3) and the new one
  spawned under the block's owner with the block's deadline.
- The block's deadline: `{await expr within=CONST}`, an attribute on
  the opening hole, a Rust expression of `Duration`; or `within=user`
  for a wait that is the user's. Omitting it is refused: "every
  `{await}` names its deadline: `within=PROFILE_DEADLINE` from a
  measurement, or `within=user`".
- Branches: `pending` (the nodes before `{then}`), `then` (with the
  value), `catch` (with the error). `{then}` is required; `{catch}` is
  optional, and without it an `Err` goes to the error report (section
  8) and the block shows nothing. The pending nodes may be empty.
- The value's branch is built when the future settles, the value moved
  into its builder as a plain binding (no `PartialEq`, no `Clone`);
  when a later future settles the branch is removed and built again
  (D3).
- What shows while a later future is in flight: the branch shown now
  (then or catch). The pending nodes show only before the first
  settle. This is 01's rule 2 (narrow hold) applied to the block, and
  it is where the block differs from Svelte's `{#await}` (section 11).
- Unmount: the block's owner is disposed with its fragment
  (`blocks/mod.rs` `Fragment::remove` disposes the owner first), so
  the task is dropped before its nodes are removed.

Runtime: `vapors_dom::blocks::awaiting(dom, parent, anchor, deadline,
future_of, pending, then, catch)`, one function, generic over `T` and
`E` as `conditional` is over its builders; generated code is one call
per block, like the `{if}` call in the counter snapshot
(`crates/compiler/tests/snapshots/codegen_snapshot__counter_component_generates_readable_rust.snap`
shows the shape for a hole).

Compiler: `Kind::Await`, `Kind::Then`, `Kind::Catch` in
`crates/compiler/src/template/holes.rs`; `Node::Await(AwaitBlock)` in
`crates/compiler/src/ast/mod.rs`; parsing beside `parse_if` in
`template/control_flow.rs`; emission beside `emit_if` in
`codegen/blocks.rs`; `expressions_in` collects the expression and the
branches' reads for the builders' clones. Refusals: `{then}` or
`{catch}` with no open `{await}`, two `{then}`, `{catch}` before
`{then}`, a `{then}` without a name, a name that shadows a script
binding (the snippet-parameter rule, `components.md` "snippets"), a
missing `within=`.

## 6. `Resource<T, E>` in the script

A derived async value a script can read and other code can share:

```rust
let stats: Resource<Stats, String> =
    Resource::new(Deadline::Measured(MEASURE_DEADLINE), move || measure(layer.get()));
```

- A struct field, as `Signal` and `Memo` are (`codegen/script.rs`
  `reactive_kind` gains `Resource`; `classify_field` names it with
  `Resource::named`).
- The closure is synchronous and tracked: it reads its sources and
  returns the future. The future is polled untracked. So there is no
  "read after an await" to refuse in a resource: the shape has no
  tracked code after the await (section 10).
- `stats.get()` returns `Async<T, E>`: `Pending`, `Ready(T)`,
  `Failed(E)`; `T: Clone`. Reading subscribes, as a memo read does; a
  settle is a change. `Ready` is kept while a new future is in flight
  (rule 2); `Pending` is only ever the first state.
- `{await stats}` in a template takes a resource as well as a future:
  the block's effect reads `stats.get()` and shows the branch for the
  state, with no spawn of its own. The runtime function has two entry
  points, one per source; generated code picks by the expression's
  type through a trait with two impls (`AwaitSource` for `F: Future`
  and for `Resource`). This is the one trait in the plan, and it
  chooses an entry point, not a branch.
- A resource is 1-R2's `Memo::async` with the closure made
  synchronous. D7 records the change.

Not in this plan: `refetch`, `mutate` (Solid's), a resource's own
pending count on a boundary. Each waits for a use.

## 7. Top-level `await` and `<boundary>`

Design 01 says a script may `await` at its top level and the nearest
`<boundary>` shows `pending` until the component's first awaits
settle. The shape that fits the compiler as built:

- The compiler finds the first statement containing a `syn::Expr::Await`
  (the script is a `syn::Block`, `codegen/script.rs` `analyse`). The
  statements before it run in `mount_before` as today; it and the rest,
  with the template build, run in a mount task spawned under the
  component's owner. `mount_before` returns the component with an
  anchor as its only root until the task builds the template before
  the anchor.
- A `Signal`, `Memo`, `Resource` or `#[dispatcher]` `let` after the
  first await is refused: "declare `name` before the first `.await`;
  the component struct is built at mount. Give it a default and write
  it after the await". Plain bindings after the await are fine: the
  template's closures are built after them.
- The mount task's deadline is the script's `#[deadline(CONST)]`
  attribute on the first awaiting statement, or `#[deadline(user)]`;
  missing is refused as in section 5.
- The task needs a boundary to show pending and to receive a failure.
  The boundary is found through the owner chain, which needs one new
  thing in `vapors_signals`: a value an owner provides and a descendant
  finds (`Owner::provide::<K>(value)`, `Owner::find::<K>()`, keyed by
  type, walking parents through `OwnerNode::parent` in
  `crates/signals/src/runtime.rs`). A component built later under a
  boundary (inside an `{if}` branch) finds it through the chain, where
  a stack kept only while the boundary builds its children would not.
- No boundary above: the mount is refused before the task starts,
  naming the component and the fix ("wrap `<Profile>` in a
  `<boundary>`, or move the await into `{await}`"), through the error
  report. The component shows nothing. Design 01 says the same for a
  failure outside a boundary.
- `<boundary>` is a component tag the compiler knows
  (`component_tag.rs` emits a call into `vapors_dom::blocks::boundary`
  instead of a `mount_before`): `pending` and `failed(error, reset)`
  snippets, children. It keeps a count of mount tasks pending under it;
  the count's transitions switch what is shown; `reset` disposes and
  rebuilds the children.

Recommendation (D1): build sections 3 to 6 first and this section
last, after a touche component shows a need no `{await}` block covers.
Section 2 found none today.

## 8. Errors

One rule, three places, one sink:

| Where | `Err(e)` goes to | Panic |
|---|---|---|
| A `spawn`ed task | The error report (`vapors_dom::set_error_report`; `ElementsApp` installs `report_error`, which prints natively and writes the page's `#error` element, `crates/platform/src/report.rs`). The message names the task's `spawn` site and `e` | Propagates on the UI thread: the app stops, as a panic in a handler does |
| An `{await}` block | The `{catch}` branch when there is one; else the error report, naming the block's file and line, and the block shows nothing | as above |
| A `Resource` | `Async::Failed(e)` to its readers; also the error report when no reader is subscribed at the time, so a failure nobody shows is still seen | as above |
| A mount task (section 7) | The boundary's `failed`; with no boundary the mount was refused earlier | as above |
| A deadline reached | As the task's `Err`, with the deadline and the time judged in the message | n/a |

`Async::Failed` and `{catch}` carry `E` as given; the report carries
`e.to_string()`. The error report is the one sink the app already has
for a handler's `Err`; nothing new to install, and a report of another
name cannot replace it silently (`listener_result.rs`
`set_error_report` panics on a second name).

Design 01 names the three-state `Outcome<T>` from area 8 for remote
calls. That is the value inside `Ok`, matched by the component;
`Unreachable` reaches `{catch}` only through an explicit
`outcome.required()?`, as 01 says.

## 9. Cancellation and unmount

- A task is registered with the owner current at `spawn`: the
  component's (`shell.rs` wraps the script and the template in
  `owner.with`), a block fragment's (`Fragment::build` makes a child
  owner), or an effect's scope. `on_cleanup` is not used for it: the
  task id is on the `OwnerNode` as nodes are, so `collect_owner`
  (`crates/signals/src/owner.rs`) takes it out, and the future is
  dropped after the runtime borrow is released, in `Released::finish`,
  where user `Drop` code may run.
- Dropping the future is the cancellation (01, rule 3). What the
  future owns is dropped with it: a `JsFuture`'s promise is not
  cancelled by that (the browser's fetch runs on; its result is
  dropped on arrival by `JsFuture`'s closure finding no receiver), and
  a native thread started by the future runs on. The `Cancel` token of
  1-R6 is the answer for work that left the future; this plan does not
  build it, and a task that starts such work says so in its doc.
- A task spawned inside an effect's scope is dropped when the effect
  re-runs (each run disposes its scope, `effect.rs` `create`). That is
  the intended semantics of a derivation restarting, and a surprise
  for a task meant to outlive the effect: the doc says to spawn from a
  handler or the script's top level.
- Order at unmount: the owner is disposed before nodes are removed
  (`shell.rs` `unmount`, `blocks/mod.rs` `Fragment::remove`), so a task
  never sees a removed node's signal; a `bind:this` signal is cleared
  by the same rule (`components.md`, "Element references").
- `Task::cancel()` for an explicit stop (a Cancel button): the same
  drop, from the handler.

## 10. Signals and the compiler's analysis

What the analysis must see is every place a tree can change. A task
changes the tree only by writing a signal a template reads, which is
how a handler changes it today; neither holds `dom`, which generated
code never hands to script code. So `spawn` adds no path the analysis
cannot see. The rules that keep it so:

- Tracked reads happen only in synchronous closures: an `{await}`
  block's expression, a `Resource`'s closure, an effect. The executor
  polls with no observer on the stack (section 3.2), so a `.get()`
  inside a future subscribes nothing and restarts nothing. Design 01
  and Q6 (a) wanted a compiler refusal for a read after an await in a
  tracked scope; with these shapes there is no tracked code after an
  await, so the refusal narrows to one case the compiler can see with
  `syn`: a `.get()` inside an `async` block or an `async move` closure
  that is the `{await}` expression or a `Resource` closure's return.
  Message: "read `layer` before the `async` block and move the value
  in: a read inside it is not tracked".
- `spawn_local`, `block_on` and `spawn` by other names in a script are
  refused by last path segment (`script_uses.rs` sees `use` items;
  `syn` sees the call), naming `vapors_signals::spawn`: an executor the
  loop does not drive would run tasks the frame order does not see.
- A write from a task goes through `Signal::set`, which runs effects
  at once when not batched (`flush.rs`). `run_ready` wraps each poll in
  a `batch`, so a task that writes two signals before yielding runs
  their effects once, as a handler's writes in one `batch` would.
- Held updates (1-R3, pending versions queuing downstream effects) are
  not in this plan. With `Async::Ready` kept while a new future is in
  flight, a reader of a resource never sees a gap, which is what rule 2
  asks for at the resource. Holding the readers of the written signal
  until the resource settles is the wider rule Tom rejected (Q3). What
  1-R3 adds beyond this plan is a `pending()` count on a boundary; it
  waits for section 7.

## 11. Comparison

What was checked: Svelte's docs in `~/projects/svelte` at 67c4a38
(2026-02-08), `documentation/docs/03-template-syntax/05-await.md` and
`19-await-expressions.md`. The other four are from memory of their
docs and are marked so; a PR that borrows a shape from one reads the
source first.

| | Svelte `{#await}` (checked) | Svelte `await` expressions, 5.36+ experimental (checked) | SolidJS `createResource` (memory) | React Suspense (memory) | Leptos `Resource` (memory) | Dioxus `use_resource` (memory) | This plan |
|---|---|---|---|---|---|---|---|
| Unit | A block over a promise | `await` in script, `$derived`, markup | A signal source and a fetcher | A component that throws a promise or calls `use()`; the boundary catches | A source signal and an async fetcher; `Suspense` and `Transition` | A reactive closure returning a future; `suspend()` for a boundary | `spawn`, `{await}` block, `Resource` |
| New input while in flight | The block shows pending again | The UI keeps the old value until the work completes; a fast update may show before a slow one (synchronized updates) | The old value stays (`latest`); `loading` is true; the old promise is not cancelled | Shows the fallback unless inside a transition | Reruns; stale results discarded by version, I believe; not checked | Reruns and drops the previous future, I believe; not checked | The old future is dropped; the shown branch stays; pending only before the first settle (rule 2, narrow) |
| Cancellation | None (a promise cannot be cancelled) | None | None | None | Not checked | Drop | Drop; a token for off-thread work later (1-R6) |
| Errors | `{:catch}` | Nearest `<svelte:boundary>` | `error` accessor; `ErrorBoundary` | `ErrorBoundary` | `ErrorBoundary` | The `Result` the future returns | `{catch}`, `Async::Failed`, else the error report; never dropped |
| Deadline | None | None | None | None | None | None | Required on every task; `Deadline::User` declared as the exception |
| Idle cost | The browser's microtask queue; no frames unless the DOM changes | Same | Same | Same | Same (wasm) | Same (wasm) | Zero frames and zero polls until a waker fires; a frame only when the tree changed |
| Loading UI | The pending branch | `pending` snippet of a boundary on first load; `$effect.pending()` after | `<Suspense fallback>` | `<Suspense fallback>` | `<Suspense fallback>` | `<SuspenseBoundary>` | The pending nodes; a boundary later (section 7) |
| Reads after an await | Tracked (markup and `$derived`) | Tracked | Untracked | n/a | Untracked after the first await | Untracked after the first await | None exist in a tracked scope; a `.get()` inside an `async` block is refused |

Degree of similarity, stated plainly: the `{await}` block is Svelte's
`{#await}` in spelling and branches, differing in the deadline, the
`Result` requirement, and what shows on a second future (Svelte shows
pending again; this plan keeps the branch). `Resource` is Solid's
`createResource` in shape (source closure, async fetcher, three
states, old value kept) with cancellation by drop, which Solid does
not have. `spawn` is Dioxus's `spawn` in shape (an owner-scoped task
dropped with its scope); the deadline and the `Result` contract are
this plan's. React's model (throw a promise) is not taken at all.

## 12. Tests and oracles

| Part | Oracle |
|---|---|
| Executor | Unit tests in `vapors_signals` with the manual executor (11-Q3): a task polled only when woken; a task dropped at owner disposal is never polled again (a future that records polls); a wake with no hook refuses naming the fix; `run_ready` inside an effect refuses naming the effect; each proven by breaking |
| Idle | `idle_fps` (`crates/app/examples/idle_fps.rs`) gains a case with a pending task that never wakes: 0 frames in 3 s; `scripts/web-idle-frames.sh` the same on the page (`?demo=task`); a woken task that changes nothing: 0 frames; one that writes a shown signal: 1 frame, through `scripts/web-wake-frames.sh`'s counting |
| Deadline | A stepped clock (`crates/app/src/frame.rs` `Clock`): a task past its deadline is refused at the next poll naming the site and the duration; a task that completes at the deadline wins; the frame at the deadline comes from `WaitUntil`, pinned as the tooltip's is (`crates/app/src/elements_app_give_tooltip_tests.rs`) |
| `{await}` block | `RecordingDom` (`crates/dom/src/recording.rs`): the sequence of calls for pending, then, catch, a second future (branch kept, then rebuilt once), unmount while pending (owner disposed before nodes removed); the compiler's snapshot of the generated call; every refusal in section 5 with its position |
| `Resource` | A signal change drops the old future (a future that records polls after the change: none) and the state stays `Ready` until the new one settles |
| Errors | An `Err` from a task reaches the installed report with the site; a `Resource` failing with no reader reaches it; a task's panic stops the app (a `should_panic` test) |
| Web | The page contract (`scripts/check-page-contract.sh`) gains the `task` demo: a `JsFuture` over `setTimeout` resolved through our executor, the frame count 1, and the hop's latency (promise resolution to frame) recorded in `docs/web.md` |
| Compiler refusals | `wisp-check` cases for each message in sections 5, 7 and 10 |

## 13. PR stack

Each row under about 400 lines (estimate), built by a fresh agent, each
check proven by breaking what it guards. After each row the next is
checked against what it found.

| PR | Content | Accepted when | Depends on | Parallel with |
|---|---|---|---|---|
| T1 | `vapors_signals`: `spawn`, `Task`, the arena, the ready list, `run_ready`, the wake hook; tasks on `OwnerNode`, dropped in `Released::finish`; `Deadline` as a type carried but not yet judged | The executor tests in section 12; a task's drop at disposal proven by a future that would have written a signal; the `!Send` of the task future asserted with a negative impl check, which design 02's threading oracle asks for the graph's types and is not built yet | none | C1 |
| T2 | `App::woken`, `ElementsApp` installing the hook, `run_ready` on a wake, a frame only on change; `Phase::Tasks` time; `idle_fps` case | 0 frames in 3 s with a pending task natively; 1 frame per task that changes the tree; the wake-without-change case 0 frames | T1 | C1 |
| T3 | Deadlines judged: `earliest_deadline`, `Platform::request_frame_at`, the handler's `WaitUntil` after an event, the refusal message | The stepped-clock tests; the frame at the deadline pinned; the work-first rule proven by a task that completes exactly at the deadline | T2 | C1, W1 |
| W1 | The `task` demo on the web page and its page-contract case; `JsFuture` through the executor; hop latency recorded | `web-idle-frames.sh` 0 with a pending task; 1 frame per resolution; latency in `docs/web.md` | T2 | T3 |
| C1 | Compiler: parse `{await}`, `{then}`, `{catch}`, `within=`; the AST node; refusals with positions; no emission yet (a refusal "await blocks are parsed and not yet built" at generation, so nothing half-works) | `wisp-check` cases for every refusal; the parse pinned by AST tests | none | T1 to T3 |
| C2 | `vapors_dom::blocks::awaiting` and the emission; `expressions_in` for the block; the counter-style snapshot | The `RecordingDom` sequences in section 12; unmount while pending; the second-future rule | T3, C1 | R1 |
| R1 | `Resource<T, E>`, `Async<T, E>`, the field kind in the compiler, `{await resource}` through `AwaitSource` | The resource tests in section 12; a `Resource` failing unread reaches the report | T3, C2 | A1 |
| A1 | The analysis refusals: `.get()` inside an `async` block in an `{await}` expression or `Resource` closure; `spawn_local` and `block_on` in a script by name | Each refusal's `wisp-check` case, and a passing case for a read moved before the block | C2, R1 | R1 |
| P1 | touche: the profile sheet's load through `{await}` with a measured IndexedDB deadline; `page_app.rs`'s `spawn_local` sites through `spawn` | `check_ask`'s learn and loud checks pass unchanged; the deadline named in the component | R1, a vapors pin bump in touche | none |
| B1 | `Owner::provide` and `Owner::find`; `<boundary>` with `pending`, `failed(error, reset)`, the pending count | `RecordingDom` tests for pending, settled, failed, reset; `find` through an `{if}` branch built later | R1, D1 answered | none |
| S1 | Top-level `await`: the script split, the mount task, the field-after-await refusal, `#[deadline]` | A component awaiting at mount shows the boundary's pending then its template; the refusals; no boundary refused naming the fix | B1 | none |

Critical path to touche's first use: T1, T2, T3, C1, C2, R1, P1. W1
carries the web check; A1 the analysis rule; B1 and S1 wait on D1.

## 14. Decisions for Tom

Each with options, a recommendation, and what it changes.

**D1. Top-level `await` and `<boundary>`: now or on a use?** (a) Last,
after a touche component needs an await before its whole template
(section 2 found none: the profile sheet and the model call are an
`{await}` block and a `spawn`). (b) In this stack's order as design 01
has it, R4 and R5 after R2. Recommendation: (a). It moves two PRs (B1,
S1) and the owner-context feature behind a use, and nothing in T1 to
P1 needs them; design 01's three places become two until then.

**D2. The `{await}` expression's type.** (a) `Future<Output =
Result<T, E>>` only; an infallible future wraps its value in `Ok`. (b)
A trait over `T` and `Result<T, E>` so either is accepted.
Recommendation: (a): one shape, and the `{catch}` branch's meaning is
visible in the template, not chosen by a type. touche's futures all
return `Result<_, String>` already.

**D3. What a second settle does to the `{then}` branch.** (a) Remove
and build the branch again with the new value moved in; `value` is a
plain binding; focus inside the branch is lost on refresh. (b) Keep
the branch and update `value` as a memo parameter, which needs `T:
PartialEq + Clone` (an image in an `Rc`). Recommendation: (a) first,
measured; (b) when a refresh that costs a remount shows up, since the
runtime function's signature allows either and the parse is the same.

**D4. A deadline on a hidden page.** (a) Wall time, judged at the first
frame after the page shows, the task polled first, the judged time in
the message. (b) Visible time only, as the GPU scope's deadline counts
it (`painter_surface_scope_deadline.rs`). Recommendation: (a): a fetch
or a worker goes on while the page is hidden, unlike a GPU scope,
which only resolves with frames; and (a) has no timer of its own.

**D5. Is `Deadline::User` allowed on an `{await}` block?** (a) Yes,
`within=user`, for a block awaiting a picker's answer. (b) No: a
block's wait is never the user's; a picker is a `spawn` from a handler.
Recommendation: (a), since 07's rule names the kind at the call and a
block is a call; refusing it would push a picker's result through a
signal and a second block for no gain.

**D6. The executor's crate.** (a) In `vapors_signals`, beside the
runtime, since tasks belong to owners and `run_ready` reads the batch
state. (b) A new `vapors_tasks` crate depending on `vapors_signals`,
with the owner registration through a public hook. Recommendation:
(a): the hook in (b) is a public API that exists only to let a second
crate reach into the first.

**D7. `Memo::async` becomes `Resource`.** Design 01 wrote `Memo::async(
move || async { measure(layer.get()).await })`, a closure whose
tracked part is before its first await. (a) `Resource::new(deadline,
move || measure(layer.get()))`: a synchronous tracked closure returning
a future, no tracked code after an await, and a deadline. (b) As 01
wrote it, with the compiler's refusal of reads after the await.
Recommendation: (a): it makes Q6's rule hold by shape rather than by a
lint, and it carries the deadline. 01 is updated by the PR that builds
R1.

**D8. The `App::woken` method.** (a) A new trait method with the
default `request_redraw`, so every app keeps today's one-frame-per-wake
and `ElementsApp` opts into the poll-first path. (b) The handler polls
through a hook the platform owns, no trait change. Recommendation:
(a): the platform crate stays ignorant of the executor, as it is of
the document today.

## 15. Risks

- **winit's web proxy hop.** `Waker::wake` posts through
  `EventLoopProxy::send_event` (`crates/platform/src/waker.rs`); on the
  web winit schedules the delivery itself. One hop per wake between a
  promise resolving and the poll. W1 measures it; if it is over a
  frame, the fallback is a `Message::Wake` posted through a
  `MessageChannel` of ours, in the platform crate, no app change.
- **A `JsFuture` is not cancelled by a drop.** The browser's work runs
  on; only its result is dropped. A fetch that must stop needs an
  `AbortController` in the future's own `Drop`; touche's transport
  (`crates/runtime/src/transport_web.rs`) is where that goes, not the
  framework.
- **`Send` on the web.** The task's `std::task::Waker` must be `Send +
  Sync`; the platform waker is not `Send` on the web.
  `elements_app_scope_wake.rs` solves it with `send_wrapper` and a
  `compile_error!` under `target_feature = "atomics"`; T1 reuses that
  pattern, with the same guard, so a threaded wasm build cannot reach
  the panic.
- **A task that never wakes and has no deadline.** `Deadline::User` is
  that case by declaration. A `Measured` task that is never woken is
  judged at its deadline because the executor arms a frame for it
  (section 4); without T3 a T1-only build could hang silently, so T3
  is on the critical path before any product use.
- **Thread-local executor and parallel tests.** nextest runs each test
  in its own process (`.config/nextest.toml` sets its profile), so one
  executor per test; a test that uses `cargo test` instead shares a
  thread per test thread and must not assume an empty executor.
- **Re-entrancy.** A task polled from inside a handler's `batch` would
  write mid-flush; `run_ready` refuses inside a batch or flush
  (section 3.2). The one caller is `App::woken`, which runs between
  events; the refusal is for a future mistake, proven by a test that
  calls it inside an effect.
- **The hidden page.** A deadline judged late (D4 (a)) means a user who
  returns to a tab sees the refusal then, not at the deadline. The
  message carries both times.
- **A remount on every settle (D3 (a)).** A `{then}` branch holding a
  focused control loses focus when a new value settles. Known until a
  use asks for (b).
- **Dependence on the finish line.** The frame-order and idle rules
  here are measured by the harness `perf-finish-line.md` builds
  (`vapors-perf --scaling`, `idle_fps`). If that harness changes shape
  before this plan starts, section 12's rows are re-read against it.
