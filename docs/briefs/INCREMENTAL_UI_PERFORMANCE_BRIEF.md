<!-- Source: untracked INCREMENTAL_UI_PERFORMANCE_BRIEF.md at the vapors main checkout root, written 2026-11-09; copied unchanged 2026-11-10. -->

# Brief: incremental UI performance with the existing Vapors API

Status: approved investigation and improvement direction. API names below are
proposals, not existing functionality. Written 2026-11-09.

## Objective

Make sparse changes cost approximately the work required by their affected
dependencies, while preserving compiled `.wisp` templates, signals, props,
bindings, scoped styles and keyed loops.

Keep output and interaction behavior equivalent to a full update. Do not optimize
by dropping required layout, clipping, accessibility or input work. Local edits
can legitimately have broad dependencies; the engine must identify those rather
than assume every change is local.

This complements `GPU_SURFACE_3D_BRIEF.md` in this clone root. The proposed surface
and declarative scene APIs should preserve the same incremental-update contracts.

## Evidence and starting point

The review examined `f25556a` in `/home/tom/projects/vapors-racer`, branch
`demo/top-down-racer`, PR https://github.com/tomyan/vapors/pull/132.
Inspect current source before implementation: other framework work may have
advanced. Coordinate integration and do not overwrite another active checkout.

At that revision:

- Components mount once. Template expressions compile into effects that update
  persistent nodes (`crates/compiler/src/codegen/text.rs`).
- Signal reads track dependencies. `Signal::set` notifies readers without an
  equality guard (`crates/signals/src/signal.rs`).
- Document text, attribute and property setters write/invalidate even when given
  the same value (`crates/elements/src/dom_impl.rs`).
- Selective restyling, layout reuse and text-layout caching already exist.
- `Document::update` still checks/enumerates the tree, builds layout structures,
  places boxes and constructs a new painted result (`update.rs`, `paint.rs`).
  Cached recomputation does not eliminate traversal.
- Keyed loops preserve rows, but collection reconciliation still examines their
  contents (`crates/dom/src/blocks/keyed.rs`).
- The racer writes several unchanged HUD values each frame. Its smooth custom GPU
  scene and small HUD do not establish large-tree UI scalability.

The architecture permits improvements behind the existing component API. The work
is primarily change detection, dependency-aware invalidation and retained engine
structures, not replacement of the authoring model.

## API compatibility

Preserve component construction, event ordering, keyed identity, bindings, focus,
pointer capture and lifecycle cleanup. Keep failures visible and actionable.

Do not silently change `Signal::set` to suppress notifications: consumers may use
a write deliberately as a notification. Propose an additive equality-aware
operation such as `set_if_changed` for `T: PartialEq`, while retaining explicit
notification. Define behavior for unusual equality, including NaN and types with
interior mutability.

Document mutation guards may skip identical updates only when setter semantics
are preserved. Check text-editor state, selection, validation, control state and
observers. Do not suppress user events merely because values match.

Retained layout/paint changes should be internal. Collection and virtualization
APIs may be additive; present their design choices to the user when measurements
establish the need.

## Recommendation: decide contracts early, optimize against measurements

Preserve the `.wisp` authoring model, but do not treat every existing API decision
as fixed. Prefer additive changes where they express the required semantics.
If an existing contract prevents correct or efficient behavior, document the
trade-off and migration options for user approval before changing it.

Settle these contracts before the GPU-surface and 3D APIs accumulate consumers:

- **GPU surfaces:** explicit content invalidation, animation requests under the
  shared scheduler, resource ownership, resize, unmount and device-loss behavior.
  Custom renderers must be able to express when their output changes without
  invalidating unrelated UI.
- **3D components:** persistent node identity, shared geometry/material ownership,
  keyed instance identity and explicit instancing. The API must permit local
  transform and instance updates without reconstructing resources. Do not promise
  automatic batching of arbitrary scene components.
- **Signals and transactions:** document equality, explicit notification, batching,
  effect timing and whether observers can see intermediate states. Audit current
  behavior first. Prefer additive `set_if_changed` semantics; propose a transaction
  API only if existing batching cannot express coherent updates. Do not silently
  change the behavior applications already observe.

These are contract decisions, not a requirement to implement every optimization
immediately. Validate them with small examples: a surface animating beside static
controls, two meshes sharing geometry, a keyed instance update and two related
state writes observed by an effect. Present unresolved API choices to the user.

Defer specialized collection mutation and virtualization APIs until representative
large-list or tree workloads show the need. Keyed insert/remove/move operations
can avoid whole-collection comparison, but introduce ordering and notification
contracts. Virtualization also needs measurement, scroll anchoring, focus and
accessibility semantics. Review these together before publishing those APIs.

Proceed with internal layout and paint optimizations in measured slices. Public
API compatibility makes migration less urgent; it does not make implementation
cost independent of timing. Stable identity, explicit ownership and dependency
invalidation should be designed early so later optimizations do not require
replacing substantial engine structures.

Recommended order:

1. Record the current signal/batching semantics and agree the surface/3D contracts
   alongside `GPU_SURFACE_3D_BRIEF.md`, before those APIs are implemented publicly.
2. Establish the baseline and full-recomputation oracle described below.
3. Implement safe unchanged-write suppression and measure the result.
4. Choose retained layout, retained paint or surface-isolation work according to
   measured cost and the active surface implementation. Reassess after each slice.
5. Introduce collection or virtualization APIs when measured application needs
   justify their added contracts.

This recommendation authorizes documenting the design direction. Implementation
still requires the agreed slice plan; unresolved semantic or architectural choices
must be reviewed with the user.

## Baseline and instrumentation

Use the real component-to-pixel pipeline. Test at least 100, 1,000 and 3,000 mounted
rows, plus a larger case if useful. Record revision, profile, hardware/backend,
scale, warmup and sampling procedure. Include virtualized workloads separately.

Measure initial mount/teardown, one fixed-size label edit, a size-changing label
edit, an equal write, hover/focus, typing, paint-only and inherited-style changes,
keyed insertion/removal/reorder/filtering, scrolling, viewport resize and an
animated GPU region beside unchanged UI. Compare fine-grained item signals with
a deliberately coarse collection signal without conflating their workloads.

Instrument:

- effects executed and nodes visited in each phase;
- elements restyled, boxes recomputed and text layouts rebuilt;
- paint records rebuilt, GPU upload bytes and draw calls;
- allocations, retained memory and idle frame scheduling;
- CPU update/encoding costs and GPU time where supported.

Existing `elements_restyled`, `boxes_laid_out` and `text_layouts_built` counters are
useful but insufficient: a cache hit can still require a full-tree walk. Report
distributions and scaling, not only averages or FPS. State excluded costs. Do not
introduce synchronous GPU waits into the normal path to collect measurements.

## Implementation slices

### 1. Baseline and correctness oracle

Build a deterministic sparse-update workload and counters before optimizing.
Compare incremental results with forced full recomputation of equivalent state.
Extend existing cache-discard facilities where necessary.

Acceptance: deliberately break invalidation and prove the correctness oracle
fails. Record baseline scaling and identify dominant phases. Keep structural
counter assertions separate from noisy timing thresholds.

### 2. Eliminate unnecessary writes

Add safe guards for unchanged text, attributes and properties. Add an explicit
equality-aware signal operation and use it where appropriate in the racer's HUD.
Batch related updates only where event semantics remain correct.

Acceptance: equal updates preserve state/output without avoidable work; changed
values still notify every dependent binding. Cover controls, validation, focus,
selection, reset epochs and deliberate notifications. Prove no-op regression
checks fail without their guards.

### 3. Retain layout structures and dependencies

Preserve stable layout nodes and update relationships as the element tree changes.
Avoid walking unaffected branches just to rediscover cached results.

Distinguish intrinsic-size, layout-style, paint-only and interaction changes.
Track parent sizing, sibling placement, text wrapping, inheritance, scrolling,
clipping and selectors that affect other nodes. Viewport and stylesheet changes
may legitimately invalidate broad regions.

Acceptance: sparse fixed-size changes visit bounded affected regions. Edits with
broader dependencies update them correctly. Test insertion, removal, reparenting
and recovery after refused updates against full recomputation.

Validation must remain correct: cache validation only with explicit invalidation
for every input it depends on. Unsupported input must still fail loudly.

### 4. Retain paint and interaction data

Preserve unchanged paint records, text resources and GPU buffers. Update affected
records while keeping stacking, clips, transforms, hit targets and input geometry
consistent. Define deletion and resource lifetime explicitly.

Retained paint data does not require partial framebuffer presentation. An animated
scene may redraw a complete frame while reusing unchanged UI preparation and GPU
data. Damage-region rendering is not an unmeasured prerequisite.

Acceptance: paint-only edits avoid unrelated layout. Drawing unchanged UI does
not rebuild its paint records. Deletion leaves no stale draw or hit records.
Compare both GPU output and interaction targets with the full-update reference.

### 5. Independent GPU-surface invalidation

Integrate with the panel-capable surface brief. Surface content, surface geometry
and surrounding UI require distinct invalidation paths. Camera animation must not
rebuild unrelated HUD layout or text.

Use the shared scheduler, coalesce requests, preserve composition order and stop
frames when neither UI nor surfaces require animation. Resource readiness, resize
and device loss need explicit transitions.

Acceptance: animate a surface beside static controls and measure skipped UI work.
Test two surfaces, overlays, clipping, focus/capture, pause/resume and unmount.
Preserve correctness even when presenting the entire frame.

### 6. Collections and virtualization, if justified

Keyed identity does not make vector replacement incremental. Consider per-item
signals and keyed insert/remove/move operations that express local changes without
replacing an entire collection.

Virtualization needs an explicit visible-range and measurement contract, with
scroll position, focus, keyboard navigation and accessibility. Do not assume fixed
row heights or silently discard focused off-screen state.

Acceptance: editing one item does not rebuild other rows. Structural changes
preserve identity, resources and input state. Measure creation/filtering costs
and memory against the simpler API. Review API alternatives with the user first.

## Requirements for the proposed 3D layer

Scene components create persistent nodes. Transform updates do not recreate
geometry. Geometry/material handles share resources; keyed instance changes update
affected instance records. Measure uploads and draw calls rather than inferring
efficiency from declarative syntax.

Custom renderers must declare invalidation. Vapors cannot infer arbitrary writes
inside application Rust or GPU resources. Make that boundary explicit.

## Verification and delivery

Keep full recomputation as a test reference. Compare bounds, clipping, ordering,
hit targets, focus, selection and lifecycle state as well as pixels. Exercise
deterministic edit sequences, negative cases and structural changes. Where pixel
comparisons need a tolerance, document the reason where the check runs.

Follow red/green/refactor, required repository checks, commit and push. Prove new
checks fail before relying on them. Update the plan after each slice using the
measurements to choose the next valuable step. If findings require changing the
agreed architecture, stop and present choices to the user.

First deliverables: baseline harness/results, a documented dependency/invalidation
model and the smallest validated optimization. Do not treat this as permission
for an undifferentiated engine rewrite.

## Limits and references

This brief does not promise constant-time edits or superiority to another
framework. Mounting, retained resources, dependency graphs and caches have costs;
track creation-time and memory regressions alongside update improvements.

The motivating article separates fine-grained reactivity from engine traversal.
Its numbers are not Vapors measurements and must not become Vapors claims:
https://blog.zortax.de/posts/a-critique-of-gpui

User-supplied discussion:
https://www.reddit.com/r/rust/comments/1xrg224/a_critique_of_gpui/

Related brief: `GPU_SURFACE_3D_BRIEF.md`.
