# 8. Remote functions and routing

Status: design, 2026-10-25. Nothing is built in vapors; touché has
`Transport` (R3) and a key-store with three states (R2). Tom's
direction: remote functions like SvelteKit's `query` and `command`,
similar in shape to Leptos `#[server]`, with three outcomes (resolved,
absent, unreachable) and unknown argument shapes refused; routing and
load functions later and smaller. Uses in touché: the model key held
server-side for paid users (the broker), `/events`, licence checks,
later accounts and sync.

## The model

**A remote function is a Rust function with an attribute, callable from
a component as a future.** One definition gives the client stub, the
server handler, the argument and result schemas, and the transport
encoding. The server side compiles only into the server binary; the
client side only into the app. The compiler (a proc macro here, since
it is Rust in a `.rs` file, not a `.wisp` template) is the one place
that knows both sides, so the shapes cannot drift.

```rust
#[vapors::query]
async fn allowance(user: UserId) -> Remote<Allowance> { .. }

#[vapors::command]
async fn record_events(batch: EventBatch) -> Remote<Accepted> { .. }
```

| Kind | Semantics | From |
|---|---|---|
| `query` | Read. Idempotent; the client caches by arguments and refreshes on demand (`.refresh()`), or when a `command` says it changed; several queries in one frame are sent as one batched request | SvelteKit `query` and `query.batch` |
| `command` | Write. Never cached; runs once per call; may name the queries it invalidates so they refresh in the same round trip ("single-flight") | SvelteKit `command` with `updates` |
| `prerender` | Read whose result is baked into the build | SvelteKit `prerender`; not needed by touché, listed so it is a deliberate omission |
| `form` | A command bound to a form element with progressive enhancement | Not taken: vapors has no HTML form submission to enhance |
| live query | A read that keeps delivering values over one connection, reconnecting with backoff and keeping only the latest pending value | SvelteKit `query.live` (an async generator over a shared connection); vapors' streaming query (F4) is the same idea over SSE |

**Three outcomes, as a type.**

```rust
pub enum Remote<T> {
    Resolved(T),
    Absent,                 // the server answered: no such thing (404-shaped)
    Unreachable(Reason),    // no answer: offline, DNS, TLS, timeout, 5xx, a shape the client could not parse
}
```

`Absent` is only ever a server's answer; a client never infers it. A
component matches on all three; a `<boundary>` receives `Unreachable`
as a failure with the reason. touché's rule from DESIGN.md ("after
grace, viewing, editing and export stay available while paid features
pause") is written against `Unreachable`. The same three states are the
platform services' shape (area 7), so one `match` idiom serves both.

**Unknown argument shapes are refused.** Arguments and results are
`serde` types with `schemars` schemas; the server validates the body
against the schema before deserialising and refuses with the field
named (the broker's rule in touché's cost and security notes). The
client refuses a result that fails its schema as `Unreachable(Shape)`
rather than guessing, and reports the field. Unknown fields are
refused, not ignored, on both sides: a client one version ahead of its
server gets a named refusal, not a silently dropped field.

**Transport.** HTTP over area 7's `Transport`: one POST per batch to
`/_vapors/<function path>`; JSON body with the function's version hash
in a header so a mismatched client and server refuse each other by
name; `Accept: application/json`. Streaming results (the agent's turn)
are a `query` that returns `Stream<T>` over SSE, which touché's R3
already reads. Every call has a deadline from measurement, named at
the call site; reaching it is `Unreachable(Timeout)`.

**Authentication** is a header the client's session supplies; the
remote function's handler receives a `Context` with the caller's
identity or anonymity and refuses what the plan does not allow. The
key never reaches the client (Tom's rule).

**Offline desktop.** The desktop binary can embed the server (touché's
"in-process" deployment): the stub calls the handler directly with no
network, and `Unreachable` can still occur when the handler's own
dependency (a licence server) is away. One code path, two bindings,
decided at start-up.

### Caching and consistency

A `query`'s result is a signal: reading it in a template subscribes,
and a refresh is a signal write, so area 1's held updates apply. The
cache key is the argument's canonical JSON. A `command` that names
`updates(allowance(user))` makes the server run the query after the
command and return both, so the client's cache is set in one round
trip and the UI never shows a stale allowance between the two. There
is no optimistic override in this slice (area 1 records why).

### Routing and load functions, later

- A route is a component with a path pattern; `<Router>` picks one from
  the platform's location (the URL on the web, a saved value natively),
  and `navigate(to)` sets it. Deep links on the web are the reason to
  have it; natively it is window state.
- A `load` function is a `query` the route awaits before showing,
  through the same `<boundary>`.
- Nested layouts, route groups, param matchers and server-side
  rendering are not taken.

touché's routes: editor, library, settings. Not needed before accounts.

## Taken from where

- **SvelteKit remote functions** (experimental since 2.27, checked
  2026-10-25): query, `query.batch` (one macrotask's calls in one
  request), command, `updates` single-flight (the server opts in with
  `requested(query, limit)`), schema validation of arguments by Standard
  Schema with a generic 400 on failure, `*.remote.ts` files split by the
  build. Differs: no `form`, no `prerender`, no `withOverride`, Rust
  types with `schemars` instead of Standard Schema, a refusal naming the
  field instead of a generic 400, a three-state result type instead of
  thrown errors, versions in a header.
- **Leptos `#[server]`**: the attribute on an async function, the
  server-only body, the generated client stub and registered handler,
  `ServerFnError` as the error type. Differs: Leptos's error type folds
  "no such thing" and "unreachable" into one; vapors separates them and
  refuses unknown shapes where Leptos's default codecs ignore unknown
  fields.
- **tRPC and Connect**: the batch request shape and the idea that the
  schema is the API; not taken as a dependency.
- **gRPC-style versioning** by a hash header rather than URL versions.

## Deliberately left out

- Server-side rendering and hydration: vapors paints a canvas; there is
  no HTML to hydrate.
- Form actions and progressive enhancement.
- WebSockets as a transport: SSE covers streaming reads; a bidirectional
  need (voice) is touché's realtime path, outside remote functions.
- Optimistic updates.
- A generated TypeScript client. touché's skills get `.d.ts` from the
  same schemas by their own route (DESIGN.md).

## Oracles

| Part | Oracle |
|---|---|
| Shape refusal | A table of malformed bodies per function (missing field, extra field, wrong type, wrong version) each refused naming the field; proven by relaxing the validator and watching each fail |
| Three outcomes | touché's stand-in server (`check_ask` has one): 200, 404, connection refused, TLS failure, timeout past the deadline, a 200 with an unparseable body; each mapped to its outcome in a test on both targets |
| Batching | A test that two queries in one frame make one request; proven by a counter on the stand-in |
| Single-flight | A command with `updates` returns the query's new value and the cache is set before the next frame |
| Offline desktop | The embedded binding runs the same test table with no socket |
| Versioning | A client built with a changed type against the old server refuses by name |
| Routing | Chromium's `history` after `navigate`, and the page reloading at a deep link showing the route; natively a saved state round trip |

## Slices

| Slice | Delivers | touché use |
|---|---|---|
| F1 `Remote<T>` and `Transport` moved in | The type, the deadline rule, the stand-in server as a test crate | touché's key-endpoint check and events upload use one shape |
| F2 `#[query]` and `#[command]` | The attribute macro, stubs, handlers, schema validation, version header | `/events` upload as a command; the licence check as a query |
| F3 batching and `updates` | | The broker's allowance and the turn in one round trip |
| F4 streaming query | SSE results as `Stream<T>` | The agent's turn through the broker |
| F5 embedded binding | Desktop in-process server | The paid desktop offline |
| F6 router and load | Routes, `navigate`, deep links | Library and settings pages |
