# Incremental contracts: signals, setters, GPU surfaces, 3D

Status: record of the source at `8b3326f` (2026-11-10), slice IU0 of
`docs/briefs/INCREMENTAL_UI_PERFORMANCE_BRIEF.md`. It changes no
behaviour. Sections a to d say what the code does today, each statement
pointing at the code and at a test that pins it; tests added by this
slice are marked (IU0). Section e lists the API choices left to Tom;
section f states 3D slice 1c's acceptance as counters.

## a. Signals today

| Behaviour | Code | Pinned by |
|---|---|---|
| `set` and `update` notify every reader, also when the new value equals the old: `write_cell` notifies whenever `notify` is set, with no comparison | `crates/signals/src/access.rs:63-86`, `signal.rs:120-131` | `semantics_writes.rs` `setting_an_equal_value_reruns_the_readers`, `an_update_that_changes_nothing_reruns_the_readers` (IU0) |
| `Trigger::notify` is `set(())`, so it depends on equal writes notifying | `trigger.rs:38-40` | through the row above |
| A `Memo` notifies its readers only when its value changes by `PartialEq` | `memo.rs:63` | `ported_memo_effects.rs` `effect_doesnt_rerun_if_memo_didnt_change` |
| A memo whose value is NaN notifies on every recompute (NaN != NaN) | `memo.rs:63` | `semantics_writes.rs` `a_memo_of_nan_notifies_its_readers_on_every_recompute` (IU0) |
| Outside a batch, effects run synchronously inside the `set` call, before it returns | `flush.rs:34-39` | `ported_effect.rs` `test_batch` |
| Two related writes outside a batch: an effect reading both sees the state between them | `flush.rs:34-39` | `test_batch` logs `(1, 0)` between `(0, 0)` and `(1, 1)` |
| Inside `batch`, effects wait for the outermost batch to end; nested batches flush once; an effect sees only the final state | `flush.rs:18-30` | `test_batch` (`(3, 3)` only), `semantics_batching.rs` `effects_run_after_the_batch_in_creation_order` |
| A memo reading several signals written in one batch recomputes once | `propagate.rs:68-84` (a memo recomputes when read or when its effect runs, not at the write) | `semantics_batching.rs:13` `memo_reading_two_signals_set_in_one_batch_runs_once` |
| Effects in a flush run in creation order | `flush.rs:86-100` | `effects_run_after_the_batch_in_creation_order` |
| A write inside an effect is picked up after the running pass, never inside the writing effect | `flush.rs:34-39`, `effect.rs:62-64` | `signal_set_inside_an_effect_is_deferred_until_the_pass_ends`, `semantics_first_run.rs` |
| More than 100 passes in one flush panics. The message names `MAX_FLUSH_PASSES` (100 passes) and each effect still scheduled by kind, optional name and creation site, e.g. `effect created at <file>:<line>` | `flush.rs:16`, `52-54`, `102-114`, `error.rs:39-46` | `semantics_writes.rs` `an_effect_that_rewrites_its_own_source_forever_panics_naming_the_effect` (IU0); `ported_effect.rs` `recursive_effect_runs_recursively` converges in 2 and does not reach it |
| Before a re-run, an effect's previous run is disposed: the cleanups run as `Released::finish` orders them: every memo's compute is dropped first, then the effect's own cleanups in registration order, then those of nested nodes, newest first | `propagate.rs:101-111`, `owner.rs:83-89`, `104-122` | `semantics_writes.rs` `a_rerun_runs_the_effects_cleanups_in_order_then_its_nested_effects_newest_first` (IU0) |
| Disposing an effect runs its last run's cleanups and stops it | `effect.rs:75-77` | `semantics_refusals.rs` `disposing_an_effect_stops_it_and_runs_its_cleanups` |

There is no transaction API other than `batch`. A component that writes
two related signals outside a batch exposes the intermediate state to
every effect reading both.

## b. Document setters today

Generated bindings call the setters from an effect with no comparison
(`crates/compiler/src/codegen/text.rs:37`, `element.rs:73-76`,
`element_bind.rs:58`), so an equal signal write reaches the setter.
What a setter does when given the value the node already holds, pinned
by `crates/elements/tests/same_value_writes.rs` (IU0), on ten text
cells (the first also has a child element) and a checkbox:

| Setter, same value | `take_changed` | Elements restyled | Boxes laid out | Text layouts built |
|---|---|---|---|---|
| `set_text` (`dom_impl.rs:47-61`) | true | 0 (the parent's subtree only when a sheet uses `:empty`) | 0 | 0 |
| `set_attribute` (`dom_impl.rs:35-45`) | true | the element's subtree, wider with sibling combinators or `:has()` (`restyle.rs:78-88`) | 0 | 0 |
| `set_property("checked")` (`dom_impl.rs:71`) | true | as the sheets' `:checked` selectors scope it (`restyle.rs:94-105`); 1 here | 0 | 0 |
| `set_property("value")` (`dom_impl.rs:72-75`) | true | as `:invalid` scopes it | not pinned | not pinned |
| any other property (`dom_impl.rs:76`) | true | 0 | 0 | 0 |

`set_attribute` and `set_property("value")` also call
`editor_from_text` (`text_field.rs:166`), which leaves a field's editor
alone when its text is unchanged, so an equal write keeps the caret,
selection and undo history.

`take_changed` true is what `ElementsApp::next_after_frame` turns into
another frame (`crates/app/src/elements_app.rs:124-132`), and every
frame runs the whole `Document::update`: tree check, restyle of the
marked elements, box build, layout with its cache, scroll settle and a
new `Painted` (`crates/elements/src/update.rs:58-111`), then
`ScenePainter::draw` of the whole display list
(`crates/app/src/elements_app_frame.rs:47-87`). The restyle, layout and
text counters measure cache misses; none counts the traversal around
them.

## c. The gpu-surface contract today

`docs/gpu-surface.md` is the contract's user-facing description. Against
the brief's "GPU surfaces" list:

| Item | What exists | What is missing | What 1c needs |
|---|---|---|---|
| Explicit content invalidation | `SurfaceHandle::invalidate` marks the surface stale and wakes the loop, answering `Woke`, `AtFirstFrame`, `NotMounted` or `LoopClosed` (`elements_app_surface_handle.rs:100-123`); the next frame renders it once (`painter_surface_backing.rs:61-63`, `92-94`). Pinned: `elements_app_surfaces.rs`, `gpu_app_surfaces.rs` `gpu_an_invalidate_through_the_handle_renders_the_surface_once_more` | An invalidate re-renders the surface but the frame also runs the whole document update and UI paint (b); surface content and surrounding UI share one invalidation path | Nothing more for a correct 1c; IU5 separates the paths |
| Animation under the shared scheduler | `SurfaceRenderer::animating()` (`surface.rs:114-116`) is polled after each frame; while true `wants_frame` holds and the app returns `Next::Continue` (`painter_surface_backing.rs:65-69`, `elements_app.rs:124-132`), on the same platform loop as the UI. Requests from several surfaces coalesce into one frame: `next_after_frame` reduces them to one `surfaces_want` bool (`elements_app.rs:125-126`). Pinned: `gpu_an_animating_renderer_asks_for_every_frame`, `gpu_surface_lifecycle.rs` `gpu_an_animating_surface_renders_every_frame_and_wants_the_next` | Starting: a renderer that turns `animating` on while the loop waits is not seen until something wakes the loop, and the renderer does not hold its `SurfaceHandle` (`mount` returns it). `SurfaceFrame` carries no frame time (`surface.rs:40-54`); the platform `Frame` has `now` | A way to start animating from idle and a frame time for the racer's simulation and camera (question e3) |
| Resource ownership | The renderer owns its resources; the painter owns the backing texture, the encoder and submission (`surface.rs:1-5`, `40-54`); device and queue are shared | Inactive surfaces release the backing but the renderer is not told (no `deactivate`, `surface.rs:67-71`) | Nothing: the racer keeps its pipelines while mounted |
| Resize | `resize(device, size)` on allocation, size or scale change, and re-activation (`surface.rs:59-62`, `painter_surface_backing.rs:129`). Pinned: `gpu_resizing_the_surface_renders_a_new_backing_at_the_new_size_once`, `gpu_surface.rs` `gpu_a_scale_change_renders_a_new_backing_at_the_new_device_size` | Nothing found | The racer's depth texture is reallocated in `resize` |
| Unmount | `ElementsApp::unmount` removes the name, its pending invalidation, and hands the renderer back (`elements_app_surfaces.rs:54-65`); teardown is `Drop`. Pinned: `gpu_a_renderer_unmounted_after_the_first_frame_is_handed_back`, `gpu_unmounting_drops_the_renderer_and_the_surface_is_then_refused` | Nothing found | Nothing |
| Device loss | Reported by the platform frame loop, which stops the app (`crates/platform/src/device_lost.rs:31-99`); surfaces attempt no recovery (`docs/gpu-surface.md`) | No surface-level transition; by design for the MVP | Nothing |

The brief asked whether surfaces animate outside the scheduler: they do
not. `animating` feeds the same `Next` the document does, so 1c builds on
the shared loop.

## d. 3D: the brief's requirements against slices 2 to 4

The brief's "Requirements for the proposed 3D layer", checked against
the 3D lane's slices as reconciled with Tom on 2026-11-09 (touche
`docs/decisions/2026-11-09-3d-surface-first.md`):

| Requirement | Slices | Match |
|---|---|---|
| Scene components create persistent nodes; transform updates do not recreate geometry | 2 (persistent scene and camera); the lane's "Reactivity" section | Matches in intent. Slice 2's acceptance checks transforms, camera validation, refusals and cleanup, not upload counts; geometry-upload counts first appear in slice 4 |
| Geometry and material handles share resources | 3 (sharing, mount and unmount), 3b (one texture across meshes, stale handle refused) | Matches |
| Keyed instance changes update affected instance records | 4 (insert, remove, reorder, one transform; unchanged geometry uploads) | Matches |
| Explicit instancing, no automatic batching | 4 (`Instances` as an explicit batching contract) | Matches |
| Measure uploads and draw calls | 4 asserts geometry uploads; draw counts appear only in the lane's performance section | Differs: no slice asserts draw calls, and the painter has no upload or draw counter |
| Custom renderers declare invalidation | Layer 1 has `SurfaceHandle::invalidate` and `animating` (c); slice 5 says signal changes and tasks invalidate the surface | Differs: slices 2 to 4 do not say how a scene node's signal change reaches the surface's invalidation; `Scene3d` is itself a renderer and must call it |

## e. Open API choices for Tom

e1. **Equality-aware signal write.** (a) `set_if_changed(value) -> bool`
for `T: PartialEq`, notifying only when `!=`: NaN never compares equal,
so an `f64` signal holding NaN notifies on every write, as memos do
today (a); a type with interior mutability compares its current
contents. (b) The same, comparing by a caller-supplied function, which
lets an app treat NaN as equal or compare by bits. (c) Both. `set`
stays as it is in every option: `Trigger` and deliberate notifications
rely on it (a).

e2. **Transactions.** (a) `batch` as it is: it already hides
intermediate states from effects; the cost is that the caller must
remember it. (b) Generated event handlers wrap each handler in `batch`,
so two writes in one handler are never seen apart; handlers that rely
on an effect running between writes would change behaviour. (c) A new
transaction type that also defers memo reads; no consumer needs it yet.

e3. **Surface animation requests.** (a) Keep `animating()` polled, and
let a renderer reach its handle (pass a `SurfaceHandle` to `mount`) to
wake the loop when it starts. (b) Replace the flag with
`SurfaceHandle::request_animation()` / `stop_animation()`, owned by the
app; the renderer stays passive. (c) Frame tasks (slice 5) own
animation, and surfaces only invalidate. Separately: add `now` to
`SurfaceFrame`, or leave time to the app's `with_per_frame` hook.

e4. **Equal-value setters.** (a) Each setter returns early on an equal
value, after confirming no observer depends on the write (editor,
validation, `:checked`); (b) only the generated bindings compare, so a
direct `Dom` call still invalidates; (c) leave setters, rely on e1 in
apps.

## f. IU5 acceptance for 3D slice 1c, as counters

Claim: animating the surface does not restyle, lay out or rebuild text
for the HUD. Checked per animating frame after the HUD settles, with the
HUD's signals written as the racer writes them (`crates/racer/src/app/
drawing.rs:129-139`, every frame).

| Check | Counter | Exists |
|---|---|---|
| No HUD restyle | `Document::elements_restyled() == 0` | yes (`update.rs:122`) |
| No HUD layout | `Document::boxes_laid_out() == 0` | yes (`update.rs:129`) |
| No HUD text rebuilt | `Document::text_layouts_built() == 0` | yes (`update.rs:137`) |
| The surface renders each frame | a render count in the test renderer, as `surface_gradient --check` counts | yes, per renderer |
| No document change noted | `Document::take_changed()` false | yes (`document.rs:105`) |
| No document update or traversal at all | elements visited per update | missing |
| UI paint records reused | paint records rebuilt per frame | missing |
| GPU work | upload bytes and draw calls per frame | missing |

With today's signals and setters, two rows fail for a HUD that writes
equal values: every equal write marks the document changed (b), and an
equal write to an attribute-bound signal restyles its element. The first
is what the racer's `sync_hud` does today: it `set`s `speed`, `lap`,
`clock`, `best`, `message`, `compact`, `follow` and `paused`
unconditionally every frame (`drawing.rs:129-150`); the first five feed
text nodes through generated bindings, and `compact`, `follow` and
`paused` feed memos. Those signals reach no attribute: the HUD's
attribute bindings are the layout class (`hud.wisp:42`,
`race_stats.wisp:27`, `car_setup.wisp:22`, `camera_toggle.wisp:15`,
`race_menu.wisp:17`, `driving_controls.wisp:24`), fed by the
`size_class` memo (`hud.wisp:21`) that stops equal `compact` writes, and
`aria-disabled` (`hold_button.wisp:62`), whose signal `sync_hud` already
compares before writing (`drawing.rs:133-135`). So the restyle row
passes today for the racer's per-frame writes, and the changed row needs
a change: e1 is opt-in, so `sync_hud` must switch those calls to
`set_if_changed`; e4(b) needs no change to `sync_hud`, because the HUD's
bindings are generated and would compare before calling `set_text`. The
three missing counters belong to IU1's baseline; until then 1c can
assert the five that exist.
