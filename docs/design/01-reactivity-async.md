# 1. Reactivity and async

Status: design, 2026-10-25. The signal graph exists (`vapors-signals`,
Leptos semantics); the compiler emits one effect per hole. This page
designs what Tom asked for on 2026-10-24: Svelte-style `await` in
scripts and templates, a boundary with pending content, updates held
until consistent, drop-to-cancel with a token for off-thread work, and
one off-thread call that takes owned data. App code stays agnostic to
where work runs.

## The reactive graph as built

`Signal<T>`, `Memo<T>`, `Effect`, `batch`, `Owner`, `on_cleanup`,
`Trigger`, `untrack`. A write marks readers dirty; memos recompute when
read and only when a source changed; effects run after the write or
the outermost batch, in creation order; a diamond runs once. The graph
is one thread-local runtime with no locks, because the UI thread owns
it. Refusals name the node and where it was created.

This stays. Everything below is built on it, and nothing below changes
the synchronous semantics `docs/components.md` documents.

## Async: the semantics chosen

### 1. `await` in three places

| Place | Meaning |
|---|---|
| `<script>` top level: `let image = decode(bytes).await;` | The component's mount is asynchronous. Statements after the `await` run when it resolves. Until the component's first `await`s settle, the nearest `<boundary>` shows its `pending` content |
| A template hole or attribute: `{summary(image).await}` | The hole is a derived value whose computation is a future. While it is in flight the hole keeps its last value; before the first value, the boundary is pending |
| A memo: `let stats: Memo<Stats> = Memo::async(move || async { measure(layer.get()).await });` | A derived value that recomputes, as a memo does, when a signal it read before its first `await` changes. (Reads after an `await` are not tracked; the compiler refuses one it can see.) |

Why these three and no more: they are Svelte's (`await` in the script,
in the template, in `$derived`), and they cover touché: decoding and
measuring are futures, the agent's turn is a stream of futures, remote
functions (area 8) are futures.

### 2. Updates are held until consistent

When a signal write makes an async derivation recompute, the UI keeps
showing the old value of **that derivation and everything downstream
of it** until it has settled. Synchronous readers of the written
signal apply at once, as today. So a drag writes `value`, the slider's
thumb follows the finger in the same frame, and the readout that
derives from the engine's render changes when the render lands. Two
values that must change together derive from one async result.

Svelte's rule (experimental `await`, 5.36+) is wider: when state read
by an `await` changes, every reader of that state is held until the
async work settles, synchronous readers included, so a label and the
number it describes cannot disagree; updates to unrelated state are not
held, and a later fast update may show while an earlier slow one is
still running. SolidJS 2.0 (unreleased; release candidate, 2026-10) drops explicit
transitions: a derivation may return a promise, a reader keeps showing
the last resolved value while a newer one is pending, and
`isPending(() => expr)` reports it; a `Loading` boundary covers first
readiness only. Leptos's `Transition` holds only the Suspense subtree
and its `Resource` documents nothing about stale or out-of-order
responses. vapors takes the narrow rule because touché is an
editor: a control under the finger must move at once, and a frame that
holds the thumb until the GPU renders is the worse inconsistency. The
cost is that consistency between a synchronous and an asynchronous
value is the component author's job, by deriving both from the async
value. This is QUESTIONS.md Q3, decided 2026-10-25 (Tom): the narrow
rule.

Mechanism: a write that reaches an async node opens a **pending
version**. Effects downstream of it are queued, not run. When the last
future of that version resolves, the queued effects run in one batch.
A second write while a version is pending joins it (the futures that
depend on the new value restart; see 3). The version's age is reported:
`pending()` on the boundary returns the count of unsettled futures and
the time since the version opened, for the busy state touché's
Feedback pattern needs within 100 ms.

### 3. Drop to cancel, plus a token

When a dependency of an in-flight async derivation changes, the old
future is **dropped** and a new one started; the old result is never
applied. Dropping a Rust future cancels everything it owns. Work that
has left the future (a thread, a worker, a GPU submission) cannot be
dropped, so every off-thread call receives a `Cancel` token that
observes the drop; the worker checks it between steps and stops. On the
web without shared memory a running wasm function cannot observe the
token (there is no shared cell to read), so cancellation there is
`worker.terminate()` and a replacement worker from the pool; with shared
memory the token is an atomic the function polls, as natively. A
result that arrives for a dropped future is discarded with a debug log
naming the derivation; it is never a refusal, because it is the
expected path.

Taken from: Rust's own cancellation model (drop) and Svelte's "a
stale result is discarded" rule. Svelte and Solid have no token because
JavaScript cannot cancel a promise; Leptos resources have the same gap.
The token is what makes the rule hold for work that runs elsewhere.

### 4. One off-thread call

```rust
#[vapors::offload]
fn encode_png(image: RgbaImage, cancel: &Cancel) -> Result<Vec<u8>, EncodeError> { .. }

let bytes = encode_png.run(image).await?;
```

An offloaded function is a named, free function taking owned input
(`Send + 'static`, and `serde` types on the web) and the token. `run`
sends the input to a thread pool natively, or to a worker pool on the
web, and returns a future for the result. App code sees one call on
both targets. The attribute registers the function by name in a table
the worker's copy of the wasm module also has, so the worker receives
the name and the input (by structured clone, or in a
`SharedArrayBuffer` when the page has the isolation headers) and calls
the function. A closure cannot cross to a web worker, which is why the
unit is a named function and not a closure; an input type that is not
serialisable is refused at compile time on the web target, naming the
type.

GPU work is not offloaded: there is one device (ARCHITECTURE.md) and
its queue is thread-safe natively but main-thread only on the web.
Work that needs the GPU is a future on the UI thread that awaits the
queue's completion; the brief's engine already does this.

### 5. `<boundary>`

```html
<boundary>
  {#snippet pending()}<Spinner />{/snippet}
  {#snippet failed(error, reset)}<ErrorCard {error} onretry={reset} />{/snippet}
  <LayerCard {layer} />
</boundary>
```

The nearest boundary shows `pending` while any component inside it
has not finished its first `await`; after that, it shows the content
and later updates are held per rule 2 (no flash back to `pending`). A
future that fails (`Result::Err`, or a panic caught at the task
boundary) shows `failed` with the error and a `reset` closure that
unmounts and remounts the content. A failure with no boundary above it
is a refusal of the update, reported as any other, and stops the loop.

Taken from Svelte's `<svelte:boundary>` with `pending` and `failed`
snippets; React's Suspense and ErrorBoundary are the same shape. The
difference: a failure outside every boundary stops the app rather than
unmounting the root, because the rule here is that failure is loud.

### 6. Where futures run

All futures that touch signals run on the UI thread's executor, which
is part of the scheduler (area 2): one task queue drained in the frame
loop between input and update. There is no multi-threaded executor
behind signals, so the graph keeps its no-lock rule. A future wakes by
posting to the platform's `Waker`, which is already how work started
elsewhere asks for a frame.

## Snippets and children

Svelte's snippets are the mechanism the boundary needs
(`{#snippet pending()}`), and children inside a component tag are what
touché's cards need (`<Card title="Curves">controls here</Card>`). Both
parse already and are refused at code generation. This design makes
them the first slice: a snippet compiles to a closure that builds its
nodes through `Dom`, a `children` snippet is the default one, and a
component renders a snippet with `{@render children()}`. A snippet's
nodes belong to the owner that renders it.

## Deliberately left out

- **Tracking reads after an `await`.** Svelte tracks them; Solid 2 and
  this design do not. A read after an await is a hidden dependency that
  may or may not restart the future depending on timing. The compiler
  refuses a `.get()` it can see after an `await` in a tracked scope and
  asks for it to be read before.
- **Optimistic updates as a framework feature** (SvelteKit's
  `withOverride`): touché's store already applies the user's edit at
  once and the engine follows; nothing is waiting on a server to show
  a slider move. Area 8 can add it when a remote write exists.
- **A `Transition` API** that an app calls to say "this update may be
  slow": every async update is held; there is no fast path that shows
  pending content for updates after the first.
- **Streaming values as a first-class signal kind.** A stream is a
  future per item; the chat panel appends through a signal write per
  chunk. If measurement shows the per-chunk relayout cost, a
  `Stream<T>` signal that batches chunks per frame is the slice.
- **Multi-threaded signals.** Leptos's `reactive_graph` has `Send`
  variants with locks; vapors does not and will not. Off-thread work
  takes owned data in and sends owned data out.
- **Svelte's `fork()`** (5.42+), which starts expected async work early
  and later commits or discards it, for preloading on hover. Area 8's
  query cache gives a `prefetch` for the one use touché has (a route's
  load); a general fork of the graph is not taken.
- **Svelte's waterfall warning** (`await_waterfall`, for sequential
  awaits that could run in parallel) is worth copying as a compiler
  lint once R4 exists; listed here so it is not forgotten, not designed.

## Interaction with other areas

- Area 2 owns the executor and when the task queue runs relative to
  input and the update. Rule: futures resolve before the update, so a
  frame never shows a half-applied version.
- Area 8's remote functions are futures that return the three-state
  `Outcome<T>` as a value, which a component matches on. Only an
  explicit `outcome.required()?` turns `Unreachable` into the boundary's
  `failed`; `Absent` is never converted, since absence is an answer.
- Area 11 tests async with a stepped clock and a manual executor: a
  test resolves futures by hand and asserts the held values.
- Area 10 records version open and settle events with their durations,
  which is where touché's "waiting time" measure comes from.

## Oracles

| Part | Oracle |
|---|---|
| Synchronous graph | Ported `reactive_graph` tests (done); Svelte 5's `runtime-runes` samples for signals, derived and effects ported where they test ordering (MIT) |
| Snippets and children | Svelte 5's 56 `runtime-runes/samples/snippet-*` cases (checked 2026-10-25 in `~/projects/svelte`) ported to `RecordingDom`, less those that test HTML-only behaviour, each skip named |
| Held updates and pending | Svelte's `await` test samples (`runtime-runes/samples/async-*`) ported to `RecordingDom`; each asserts the sequence of DOM writes with futures resolved by hand |
| Drop to cancel | A test with a future that records whether it was polled after the dependency changed; a worker test with a token that stops a loop and reports the step count |
| Offload on the web | A wasm test in headless Chromium (the page-contract script) that offloads a sum and gets it back; the same test natively; identical results pinned |
| Boundary | `RecordingDom` tests for pending, settled, failed, reset; no browser oracle, since the HTML has no boundary element |
| Performance | The stress demo gains an async counter; a held update must settle within one frame of its future resolving, measured and recorded |

## Slices

| Slice | Delivers | touché use |
|---|---|---|
| R1 snippets and children | `{#snippet}`, `{@render}`, children inside a component tag | Cards with slotted controls; the layer stack's card component |
| R2 async memo and the executor | `Memo::async`, futures on the UI thread, drop to cancel, the scheduler hook | Measurements shown in a card without blocking a frame |
| R3 held updates | Pending versions, effects queued and flushed together, `pending()` | A slider move and its readout change in the same frame after the engine renders |
| R4 `await` in script and template | Compiler support; the mount future | A card that decodes its thumbnail before showing |
| R5 `<boundary>` | `pending`, `failed`, `reset` | The history timeline's first load; a failed decode shown in place |
| R6 `offload` | Native thread pool; web worker pool with the entry-point registry; the token | Export encoding leaves the UI thread on both targets through one call |
