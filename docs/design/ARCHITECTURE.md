# Architecture: the invariants every area keeps

Status: design, 2026-10-25. The area documents (`01` to `11`) each
design one part. This page states what holds across all of them, how
each pair that interacts fits together, and every conflict found while
writing them with its resolution, or the note that it is open.

## Invariants

1. **One reactive graph.** `vapors-signals`: one thread-local runtime,
   no locks, Leptos semantics. Everything reactive in an app, including
   remote query results (8) and scene object props (9), is a node in it.
   Animated values (6) and scene bulk data (9) are deliberately outside
   it and never write to it per frame.
2. **One scheduler and frame loop.** Area 2's eight phases in a fixed
   order on the UI thread: input, tasks, animation tick, update, a11y,
   draw, record, next. Every area's work is a phase or runs inside one;
   nothing runs between frames except the platform's event delivery.
   One clock per frame, injectable.
3. **One event dispatch.** Area 3's capture-target-bubble function, used
   by pointer, keyboard, focus, wheel, input, synthetic input from tests
   and scenario replays (10 C4), accessibility actions (5) and 3D
   picking (9); the deferred automation API (Q13) will inject here too.
   A handler cannot tell them apart. Recording replay (10 C3) is the one
   consumer of input that does not dispatch: it redraws recorded
   mutations, so dispatching would apply default actions twice.
4. **One element tree.** `Document` implements `Dom`, `css::Element`,
   Taffy's traits, the hit test, the a11y projection and the recorder's
   source. A scene (9) is a node kind in it, not a second tree.
5. **One GPU device.** Created by the platform, shared by the painter,
   the text atlas, scenes and the app's engine. Web devices cannot be
   shared across workers, so render-in-a-worker moves the whole frame
   to the worker and its device (2). GPU work is never offloaded (1).
6. **The threading rule.** The UI thread owns the graph, the tree, the
   scene and queue submission order. Other threads and workers take
   owned data in and give owned data out through `offload` (1) or a
   channel, and wake the loop through `Waker`. The graph's types are
   `!Send`, asserted.
7. **The error model.** Three outcomes, one type: `Outcome<T>` is
   `Resolved(T)`, `Absent`, or `Unreachable(Reason)`, returned by
   platform services (7) and remote functions (8). A caller never turns
   `Unreachable` into `Absent`. Everything else is a refusal: an update
   that fails stops the loop and names the fix; a future that fails
   outside a boundary stops the app; an input shape not recognised
   (CSS, template, remote body, recording record) is refused naming the
   file and line or the field. Nothing is approximated, skipped or
   defaulted.
8. **The web's semantics, Chromium as the oracle.** Where a thing has a
   web definition (layout, paint, cascade, events, focus, roles and
   names, transitions, 3D transforms), vapors implements that definition
   and compares with headless Chromium. Where it has none (a boundary, a
   wgpu scene, momentum), the area document says what the oracle is
   instead. A deviation from the web is written down with its reason
   (D5 monospace, the button look, the hold rule in area 1).
9. **Refusal lists are visible.** Every deferred CSS property, skipped
   WPT test, unimplemented tag and unfired event is in a table or a
   committed list with the slice or reason, so the gap is a diff, not a
   silence.
10. **App code is platform-agnostic.** No `cfg(wasm)`, no `web_sys`, no
    native crate outside platform modules; checked by a lint with named
    exceptions. Services, futures, offload and remote calls look the
    same on both targets.
11. **Semantics come from the tree, not the app.** Roles and names (5),
    redaction tags (10), default actions (3) and animation of value
    controls (6) are properties of elements, so a component author
    cannot omit them by accident, and a missing one is loud.

## How the areas fit together

| Pair | How they fit |
|---|---|
| 1 async and 2 scheduler | Futures run on the executor in phase 2; a resolved future's writes are applied before the tick and the update, so a frame never shows a half-applied version. Waking is `Waker`, the existing redraw request |
| 1 async and 6 animation | A held update settles in one batch; the computed-value changes it makes start transitions in that frame's restyle (phase 4) and are sampled from the next tick, so an agent edit's controls animate from the settled frame. Animated values never enter the graph |
| 1 async and 8 remote | A `query` result is a signal; a remote call is a future returning `Outcome<T>`; a boundary's `failed` receives `Unreachable` |
| 1 async and 7 services | Every service call that can wait is a future with a deadline; the three-state type is shared |
| 2 scheduler and 3 events | Phase 1 dispatches all events since the last frame; coalesced pointer moves are one dispatch per frame with the samples attached |
| 2 scheduler and 5 a11y | Phase 5 diffs and pushes the tree once per frame; live announcements are coalesced separately (at most one per second); actions arrive as platform events in phase 1 |
| 2 scheduler and 9 3D | Scene bulk tasks run in the tick; the scene is drawn in paint order in phase 6 |
| 2 scheduler and 10 recording | Phase 7 writes the frame's records from the other phases' outputs; timings are the phases' spans |
| 3 events and 4 layout | Hit testing walks paint order over the last update's boxes; transforms (6, 9) map through the inverse; scroll containers (S9) own touch pans by `touch-action` |
| 3 events and 5 a11y | A reader's action is dispatched as the equivalent event; focus is one state read by both, and on the web DOM focus is a projection of it onto the mirror element, which also hosts the IME; `:focus-visible` follows the HTML heuristic |
| 3 events and 6 animation | A user drag sets a value control's display directly (no transition); a programmatic set animates the display and fires no `input` |
| 4 layout and 6 animation | A transition on a layout property relayouts per frame; the `honoured` table classifies each property as layout, paint or text, which decides the cost |
| 4 layout and 9 3D | Transforms do not affect layout; a 3D-transformed element is a layer composited by matrix; a `scene` node is a replaced box with intrinsic size from its props |
| 5 a11y and 9 3D | Scene objects with listeners are a11y nodes (a layer in the 3D view is a `button` or `option` with a name); picking is dispatch |
| 5 a11y and 10 recording | The a11y projection is recorded by reference to the element tree; names computed from user content are redacted by the same tags |
| 6 animation and 9 3D | Per-object presentational CSS transitions through the same transition machinery; bulk motion is a frame task writing arrays, not transitions |
| 7 services and 8 remote | `Transport` is the service remote functions call; both return `Outcome<T>`; the version header and deadlines are set at one place |
| 7 services and 10 recording | Recordings are written through the storage service and uploaded through a `command`; the IME path and the shared-memory choice are `Meta` records |
| 8 remote and 10 recording | A remote call's outcome by name is an `Async` record; bodies never are |
| 9 3D and 10 recording | A scene's objects are recorded as props (not textures); textures from user content are tagged `image` |
| 11 testing and every area | Each area names its oracle in its own table; the oracle runner, the stepped clock, the manual executor and the fakes are shared |

## Conflicts found, and how they were resolved

1. **Held updates versus direct manipulation (1, 3, 6).** Svelte holds
   every update while an `await` it caused is in flight. In an editor
   the slider under the finger must move in the same frame while the
   engine's render is in flight. Resolved by narrowing the hold to the
   async derivation and its downstream; consistency between a sync and
   an async value is the author's job. Recorded as a deviation from
   Svelte; QUESTIONS Q3, decided 2026-10-25.
2. **Reads after `await` (1).** Svelte tracks them; they make a
   future's restart depend on timing. Resolved: the compiler refuses a
   `.get()` on a known signal after an `await` in a tracked scope.
3. **Closures across web workers (1, 2).** The brief's "one off-thread
   call" read as a closure; a closure cannot cross to a worker.
   Resolved: the unit is a named `#[offload]` function with
   serialisable input; the same call on both targets.
4. **One device versus render in a worker (2, 5, 7).** A device cannot
   be shared across threads on the web. Resolved: the whole frame moves
   to the worker with its own device; the a11y mirror, IME and
   clipboard stay on the main thread and talk by messages. Open: the
   hop's latency, to be measured before the option is built.
5. **Programmatic value set and `input` (6, 3).** The first draft fired
   `input` at the end of an animated set; browsers fire none for a
   programmatic set. Resolved: none fires; the document changed at the
   write, the display catches up.
6. **Two three-state types (7, 8).** Services had
   `Found/Absent/Unavailable`, remote functions
   `Resolved/Absent/Unreachable`. Resolved: one `Outcome<T>`; touché's
   `KeyStore` variants are renamed when it moves.
7. **"Unknown properties refused" versus CSS extensions for 3D (4, 9).**
   Custom properties (`--x`) are valid CSS anywhere, so the cascade
   cannot refuse `--vapors-emissive`; a scene object refuses a
   `--vapors-*` property not in its list at the elements layer instead.
   Resolved by placing the check where the vocabulary is known.
8. **`transform` on scene objects (9).** Elements place by CSS
   `transform`; scene objects would then have two ways to say where
   they are. Resolved: `transform` is refused on scene objects;
   `position`, `rotation`, `scale` props are the one answer.
9. **Touch ownership between a scroll container and the canvas (3, 4).**
   A pan starting in a list must not zoom the canvas and the reverse.
   Resolved (lead, 2026-10-25): a touch pan stays with one container for
   the gesture; the canvas claims touches inside it with
   `touch-action: none`.
10. **Enter and leave animation syntax (6).** Svelte's `in:`/`out:`
    directives have no browser oracle. Resolved: `@starting-style` and
    `transition-behavior: allow-discrete`, which Chromium implements;
    `animate:flip` is kept as a directive because FLIP has no CSS
    equivalent and its oracle is a counted test.
11. **Pointer events and winit's raw touches (3).** winit reports
    touches; the web reports pointers with implicit capture. Resolved:
    touches become pointers with `pointer_type = touch` and implicit
    capture, so one dispatch serves both; the platform's gestures remain
    for the canvas element only.
12. **AccessKit has no web adapter (5).** Resolved: a DOM mirror driven
    by the same `TreeUpdate` diff; the native projection and the mirror's
    tree are compared as one JSON.
13. **Recording user text in key events (10, 3).** Key events carry the
    typed character; recording them leaks the Composer. Resolved: in a
    text field a key record carries the character's class only.
14. **Budgets and loud failure (2).** A frame over budget is not an error
    to stop on, which looks like a relaxed guard. Resolved and declared:
    it is a measurement reported in the overlay and recorded; stopping
    would make a slow machine unusable. The one update that *fails*
    still stops the loop.
15. **Optimistic updates (1, 8).** SvelteKit has them; a held-update
    model makes them a second path for the same signal. Resolved: left
    out until a remote write exists in touché; recorded in both areas.
16. **Open: the web IME path.** EditContext is Chromium-only; the hidden
    textarea has known gaps. Not resolvable by design; the platform
    reports which is in use and the recording keeps it, so the gap is
    measured. QUESTIONS Q9, decided 2026-10-25: EditContext first, the
    hidden textarea in a later slice.
17. **Open: momentum has no oracle (3, 4).** Chromium's curve is
    platform-specific. Held to a stated curve and a recorded transcript
    from the Pixel.
18. **Open: `preserve-3d` with intersecting planes (9).** Not split,
    as Chromium without its compositor flag; recorded as a difference
    from the specification.

## Seams

Places where an app draws through vapors rather than handing it
content.

- **`ImagePainter` on `ImageView`** (`crates/canvas/src/image_view_painter.rs`).
  - **Who it's for:** an app that keeps an image's pixels on the GPU and
    draws them itself. touché renders a photo's visible region from its
    own textures and layer stack, and never uploads the pixels to the
    view.
  - **What the view keeps:** the image's size, the transform, fit, zoom,
    pan and the gestures.
  - **Each frame:** the view clears the frame to its background. It then
    calls `paint` with the frame, the target, the viewport and the
    transform it would have drawn pixels with.
  - **The painter's share:** only the drawing. Placement stays the
    view's, so there is one answer to where the image is.
  - **Replacing:** `set_painter` and `set_image` each replace what is
    shown, and bump the generation.
  - **No clip:** the painter owns the whole viewport. The view and its
    app draw their overlays (`ImageApp`'s label strip) after it, so
    those stay on top.
  - **Failure:** `paint` returns `Result<(), String>`, and
    `ImageView::draw` returns a failure as `image painter: <why>`.
    `ImageApp` reports it on the error row. A painter that cannot draw
    is never a silently blank frame.

## What would break these invariants

Named so a reviewer can check a pull request against them:

- A second place that answers where a node is (a `TaffyTree`, a scene
  graph with its own transforms, an a11y tree built by app code).
- A `Send` signal, a lock in the runtime, or a worker that holds a
  reference into the tree.
- A `cfg(target_family = "wasm")` outside a platform module.
- A default taken for a missing or malformed value; a dropped CSS
  declaration; a `.unwrap_or_default()` on an `Outcome`.
- An event delivered to a handler by any path but dispatch.
- A device created anywhere but the platform.
- A test that sleeps, retries, or compares against a golden vapors
  produced where a browser could have answered.
