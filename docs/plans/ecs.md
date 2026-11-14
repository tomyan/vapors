# An entity component system for vapors: whether, and what shape

Status: design, 2026-11-12. For after the performance finish line
(`docs/plans/perf-finish-line.md`); nothing here changes current work.
Read from the code at 90d1c8a (main), open PR #252
(`crates/scene3d-components/src/mesh.rs`), and the plans on branch
`docs/design-batch` (PR #256: `docs/plans/3d-engine-parity.md`,
`docs/plans/css-for-3d.md`). No build was run for this document; every
number is cited or marked "not measured" with the method that would
measure it.

Sections:

0. The rules this design is bound by
1. What an ECS is, part by part
2. What vapors has today for each part
3. Bevy ECS, hecs, flecs and Unity DOTS: the real differences
4. The options
5. How each option fits the rest of vapors
6. Costs: measured, estimated, and how to measure
7. Recommendation and slices
8. Decisions for Tom
9. Risks

## Summary

- "ECS" is four separable things: a data layout (components in
  columns), a scheduler (systems ordered and parallelised from their
  declared access), queries (typed iteration over entities with a
  component set), and change detection (per-component ticks). vapors
  needs the first and the fourth for a scene of thousands of nodes;
  it has the fourth in a different form, and its rules forbid the
  second (section 5).
- The scene store today is one array of structs with a `Vec<NodeId>`
  per node (`crates/scene3d/src/store.rs:10`, `node.rs:28`), and the
  renderer walks every node on every render
  (`crates/scene3d-gpu/src/draw.rs:17`), checks every node's ancestor
  chain (`crates/scene3d/src/world.rs:72`) and rebuilds a hash set of
  every node (`crates/scene3d-gpu/src/slot_table.rs:43`). Idle is 0;
  a frame that draws is O(nodes × depth) on the CPU, not O(changed).
  That is the gap an ECS layout would close, and it closes without
  one (section 7, slices E1 to E3).
- The 3D parity plan leaves an ECS out because "a scheduler is a
  second answer to ordering that the analysis cannot see"
  (`3d-engine-parity.md`, section 4). Agreed for the scheduler.
  Disagreed as a reason to leave the layout alone: design 09 itself
  asks for "flat ECS-like scene data" and "flat component arrays with
  transform propagation" taken from Bevy (`docs/design/09-3d.md:9`,
  `:142`), and the store built since does not have that shape yet.
- Recommendation: no ECS crate inside vapors. The scene store becomes
  columns with a kept draw list and a batched transform write; a game's
  simulation lives in the app's own data, which may be an ECS crate the
  app depends on (hecs or `bevy_ecs`), writing the scene through the
  batch once per frame from a frame task. vapors ships one example of
  that and no glue crate until a second app asks. Seven slices, seven
  decisions.

## 0. The rules this design is bound by

- **Analysable sources only**
  (`docs/decisions/2026-11-11-analysable-sources.md`): no API whose
  effect the compiler's build-time analysis cannot see; classes from
  booleans and enums, never strings. For a scene: a node's kind is a
  closed enum the template names (`<Mesh>`, `<Group>`), so the renderer
  can be told at mount what a scene can draw (`3d-engine-parity.md`,
  G3, `SceneUses`).
- **Generated code calls a small shared runtime** (Tom, 2026-11-10):
  no per-use monomorphised copies. A `.wisp` scene emits calls into
  `vapors-scene3d`; a generic ECS `Query<(&A, &mut B)>` is
  monomorphised per component set, which is per use.
- **Runtime performance first** (`perf-finish-line.md`): idle does no
  work and draws no frame (property 6); an edit's cost is flat in
  scene size (property 1's shape; a transform change writes one
  80-byte uniform, #252).
- **Every failure loud**: refused, naming the node, field and fix, as
  `Transform::new` (`crates/scene3d/src/transform.rs:35`) and
  `Scene::add` with a stale parent (`store.rs:74`) refuse today.
- **The threading rule** (`docs/design/02-scheduler-frame-threading.md`,
  "Threading"; `docs/design/ARCHITECTURE.md`, invariant 6): the UI
  thread owns the graph, the tree, the scene and queue submission.
  Other threads take owned data in and give owned data out through
  `offload`. Bevy's multi-threaded scheduler is named as not taken
  (`02`, "Taken from where"). The graph's types are `!Send`.
- **No `unsafe`** in vapors' own code (`CLAUDE.md`). A dependency's
  `unsafe` is its own, as wgpu's is, and passes `cargo xtask deny`.
- **Files about 200 lines, functions 10 to 15, PRs under about 400
  lines** (`CLAUDE.md`; every plan in `docs/plans/`).
- **Tom's direction** (`docs/plans/particles.md`, "Decided by Tom"):
  3D in vapors should eventually be suitable for games; nothing may
  rule that out. "Suitable for games" is read here as: a scene of
  thousands of moving things at 60 Hz, simulated in Rust the app owns,
  with the engine's per-frame cost proportional to what changed.

## 1. What an ECS is, part by part

The term covers four mechanisms that engines bundle but that are
separable. Each is named here once and referred to by letter below.

**(L) Data layout.** An entity is an integer id (index plus
generation, so a stale id misses). A component is a plain struct. The
world stores each component type in its own column; an archetype ECS
(Bevy, hecs, flecs, DOTS) groups entities with the same component set
into a table whose columns are contiguous arrays, so iterating one
component type over one archetype is a linear scan of one array.
Adding or removing a component moves the entity between tables (a
"structural change"). The benefit is cache behaviour on iteration and
memory per entity equal to the sum of its components; the cost is the
move on a structural change and a lookup (entity to table and row) on
random access.

**(S) Systems and a scheduler.** A system is a function whose
parameters declare what it reads and writes. The scheduler orders
systems from those declarations plus explicit `before`/`after`
constraints, reports ambiguities, and runs non-conflicting systems in
parallel on a thread pool. Structural changes are deferred to sync
points through a command buffer so systems never invalidate each
other's tables mid-run.

**(Q) Queries.** `Query<(&Transform, &mut Visible), With<Mesh>>`: a
typed iterator over every entity holding those components, compiled
per component set. A query is the only way to read the world from a
system, which is what makes (S)'s access analysis possible.

**(C) Change detection.** Each component slot carries the tick at
which it was last written; a mutable borrow of it sets the tick
whether or not the value changed (Bevy; DOTS tracks per chunk). A
query can filter `Changed<T>` against the tick the reading system last
ran at, so a system sees only what changed since it last looked, at
the cost of one tick per component slot and a compare per candidate.

An engine also bundles things that are not ECS: a hierarchy
(`ChildOf`/`Children` components), transform propagation (a system
over the hierarchy), and asset handles. These are what vapors' scene
store does today.

## 2. What vapors has today for each part

### 2.1 The scene store (`crates/scene3d`)

- **Layout.** `Scene { slots: Vec<Slot>, free: Vec<u32>, root, live,
  dirty: Vec<NodeId>, camera }` (`store.rs:18`); `Slot { generation,
  node: Option<Node> }`; `Node { kind: NodeKind, local: Transform,
  parent: Option<NodeId>, children: Vec<NodeId>, world: Mat4,
  world_version: u64, dirty: bool }` (`node.rs:28`). One array of
  structs; each node owns a heap `Vec` for its children; `NodeKind` is
  an enum whose largest variant holds a `Projection`. A `NodeId` is
  scene id, index and generation (`handle.rs:10`), the same shape as an
  ECS entity id and as `vapors-signals`' own `NodeId`
  (`crates/signals/src/node.rs:12`).
- **Propagation.** `set_transform` marks the node (`world.rs:14`);
  `update` filters the marked nodes to those with no marked ancestor
  (`is_top_mark`, an ancestor walk per marked node, `world.rs:108`),
  collects each top's subtree into a `Vec` (`store.rs:131`) and
  recomputes `parent_world * local` for each (`world.rs:122`). Work is
  O(marked × depth + changed subtree). This is the shape of Bevy's
  `propagate_parent_transforms` (dirty subtrees only) with a different
  dirty mechanism (a list, not ticks).
- **Change detection.** `world_version` per node, incremented by
  `recompute` and by `set_appearance` when the colour changes
  (`appearance.rs`; `set_colour` was removed in CSS for 3D C3c); a reader keeps
  the version it last read (`slot_table.rs`, `Slot::written`). This is
  (C) with the tick on the node rather than the component, and with
  the reader holding its own last-seen version per node rather than
  one tick per reader. It is exact, not "mutably borrowed": a version
  moves only when a value is written.
- **Queries.** None. The renderer walks the tree from the root with an
  explicit stack and filters `NodeKind::Mesh` (`draw.rs:17-25`), every
  render.
- **Kinds are closed.** `NodeKind::{Group, Mesh, Camera}`; the
  particles plan adds `Particles` (`particles.md`, section 4) and the
  parity plan `Instances`, `Light`, `SkinnedMesh`
  (`3d-engine-parity.md`, 3.2, 3.3, 3.9). The template names the kind;
  the analysis can see it (G3).

### 2.2 The renderer (`crates/scene3d-gpu`)

Per render (`renderer.rs:61`): `scene.update()`; `mesh_nodes` walks
every node (allocating two `Vec`s); `upload_missing` hashes every mesh
node's `GeometryId`; `write_slots` collects every node id into a
`Vec`, `SlotTable::assign` builds a `HashSet` of them and runs
`HashMap::retain` over the table (`slot_table.rs:43-54`), then for
each node `scene.world_version` walks the node's ancestor chain
checking `dirty` (`world.rs:72-82`, called from `:93`) and
`slots.holds` does a `HashMap` lookup; `draw` collects a `Vec` of
(offset, geometry) and records one draw per mesh. Nothing is kept
between renders except the uniform bytes and the slot table.

So: a render touches every node several times. Bytes written are
O(changed) (#252's 80-byte pin); CPU work is O(nodes × depth) with
about four allocations and about three hash operations per node. At
the racer's 11 planned scene draws (`one-scene.md`, 2.2) this is
nothing; the parity plan's C2 measures culling at 1,000 and 10,000
nodes and would meet this cost first.

### 2.3 Signals (`crates/signals`)

One thread-local runtime, a generational arena of nodes, effects
flushed in creation order after a write or batch (`flush.rs`). Each
`<Mesh>` holds a `Signal<Transform>` and a `Signal<Colour>`, each with
one `Effect` that borrows the scene, writes the node, and invalidates
the surface (`scene3d-components/src/node.rs:67`, `mesh.rs:76`). A
write to one signal costs: `write_cell` (a runtime borrow, the value
cell borrow), `notify_observers` (clone of the observer list, state
marking), the flush (`take_pending` sorts pending by `seq`), the
effect's recompute (`clear_sources`, `dispose_contents` of its scope,
`run_computing`), the scene borrow, `set_transform`, `mark`, and
`invalidate`. Design 09 and ARCHITECTURE invariant 1 say bulk motion
is outside the graph: "scene bulk data (9) are deliberately outside
it and never write to it per frame". Nothing enforces that today
beyond the author's choice, and `<Mesh transform={signal}>` is the
only authoring form built.

### 2.4 The frame and tasks

Phases are fixed: input, tasks, tick, update, accessibility, draw,
record (`crates/app/src/frame.rs:15`). `Phases::tick` and `tasks` are
empty defaults (`frame.rs:117-119`). The racer runs its simulation
before the app's frame, from its own `App::frame` (`racer/src/app/
drawing.rs`, `prepare`), at a fixed 1/120 s step with catch-up capped
at six steps (`advance_elapsed`), and copies a `Scene { race, camera }`
value into the renderer's cell compared by `PartialEq` each frame
(`racer/src/app/surface.rs:68`). The parity plan's T1 adds ordered
tasks on the scene with named stages and T2 a batched
`Scene::set_transforms` (`3d-engine-parity.md`, 3.12).

### 2.5 What design 09 actually says

`docs/design/09-3d.md` asks for: "flat ECS-like scene data, and bulk
animation outside per-object signals" (line 9); "Scene data is flat.
The scene's objects live in arrays indexed by object id (positions,
rotations, scales, material ids, geometry ids, flags), with a
world-matrix array computed per frame from the parent chain, the same
shape Bevy's `Transform`/`GlobalTransform` propagation has" ("Stage
2"); and under "Taken from where", Bevy: "flat component arrays with
transform propagation; instanced draws keyed by mesh and material;
picking as pointer events on entities with bubbling through observers.
Differs: no ECS systems scheduler; the frame has fixed phases."

Its "Deliberately left out" list names "a general scene graph with
arbitrary nesting depth and per-node signals: `<group>` nests, but
world matrices are computed in the flat arrays, not through per-node
memos". It does not list an ECS by name; the parity plan's section 4
cites it as if it did. The accurate reading: design 09 takes (L) and a
form of (C) from Bevy and refuses (S). The store built in 3D slice 2
(`3d-2.md`, PR 2) has per-node `Vec<NodeId>` children and an
array-of-structs layout, which is the general scene graph's shape with
the dirty-subtree propagation design 09 wanted. The layout question is
open, not settled.

## 3. Bevy ECS, hecs, flecs and Unity DOTS: the real differences

Described from their documentation and source as known here, not
fetched today; a slice that copies a behaviour checks the current
version first. Versions: `bevy_ecs` 0.16, `hecs` 0.10, flecs 4
(`flecs_ecs` Rust binding), Unity Entities 1.x.

| | Bevy ECS 0.16 | hecs 0.10 | flecs 4 (`flecs_ecs`) | Unity DOTS (Entities 1.x) | vapors today |
|---|---|---|---|---|---|
| (L) layout | Archetype tables, column per component; a `SparseSet` storage option per component for ones added and removed often; entities `index + generation` | Archetype tables, column per component; entities `id + generation` | Archetype tables ("tables"); entities with generation; relationships (`ChildOf`, user pairs) are components whose id encodes the pair | Archetype chunks of 16 KB, columns within a chunk; entities `index + version` | One `Vec<Slot>` of structs; children in a `Vec` per node; ids `index + generation` |
| Hierarchy | `ChildOf` and `Children` components (0.16's relationships); `propagate_parent_transforms` over `Changed<Transform>` | None; the app stores parent ids as a component | First-class: `ChildOf` relationship, queries up and down it, cascade ordering (`cascade` in queries for parent-before-child) | `Parent`/`Child` buffer components; `TransformSystemGroup` propagates `LocalToWorld` | `parent: Option<NodeId>`, `children: Vec<NodeId>`; `update` recomputes dirty subtrees |
| (S) scheduler | Yes: systems with declared access, `before`/`after`, system sets, ambiguity reporting, parallel executor on `bevy_tasks`; single-threaded executor on wasm; structural changes through `Commands` at sync points | None by design: the app calls plain functions; `CommandBuffer` for deferred spawns | Pipelines of phases (`OnUpdate`, custom); multithreaded with worker threads in the C core; `ecs_progress` runs a frame | `SystemGroup`s with update order; jobs scheduled on the job system with dependencies from declared access; Burst-compiled | Fixed phases; the parity plan's T1 ordered tasks on the scene; no parallelism (threading rule) |
| (Q) queries | `Query<(..), (With<..>, Changed<..>)>`, cached per system, archetype-matched | `world.query::<(&A, &mut B)>()`, archetype-matched; `query_one` per entity | Queries with terms, relationship traversal, cached and uncached; a query language for tools | `SystemAPI.Query<..>`, `IJobEntity`; chunk iteration | A hand-written tree walk per reader (`draw.rs:17`) |
| (C) change detection | Per component slot: `added` and `changed` ticks set on `DerefMut`, compared against the system's last run tick; ticks wrap and are periodically clamped (`check_change_ticks`) | None | Per table: a changed flag per query per table (`ecs_query_changed`), coarse; observers for `OnSet` | Per chunk: a change version per component per chunk; `WithChangeFilter` skips unchanged chunks | Per node: `world_version` moved on a real write; each reader holds the version it last read per node (`slot_table.rs`) |
| Threads | Parallel systems natively; one thread on wasm | None of its own | Worker threads in C; wasm through emscripten | Job system, Burst | UI thread only; `offload` with owned data |
| `unsafe`, language | Rust; `unsafe` in its core (`UnsafeWorldCell`, column storage) | Rust; `unsafe` in column storage | C core with a Rust FFI binding; needs a C toolchain, and the web target is emscripten, not `wasm32-unknown-unknown` | C#, a managed runtime; not applicable to Rust | No `unsafe` in vapors' code; a dependency's is its own |
| Authoring split | None: spawn from code; scenes as `.scn.ron` or glTF | None | None in the core; a JSON scene format | Baking: GameObject authoring in the editor baked to entities in subscenes at build time; the runtime never sees the authoring form | The template is the authoring form; the compiler emits calls that build the scene (3d-2 decision 1) |

Differences that matter for vapors:

- **Bevy ECS** is the closest in intent to what design 09 asks for in
  (L) and (C), and the furthest in (S): its scheduler is the product.
  Its change detection marks on mutable borrow, so a system that
  touches every `Transform` marks every one changed; vapors' version
  moves on a write. On wasm the executor is single-threaded, so the
  scheduler's parallelism gives the web page nothing. Its transitive
  dependencies include `bevy_tasks`, `bevy_utils`, `bevy_ptr`,
  `fixedbitset`, `petgraph` (the schedule graph) and, with the default
  `bevy_reflect` feature, `bevy_reflect`; size on the web page: not
  measured (section 6).
- **hecs** is (L) and (Q) alone, about 4,000 lines, no scheduler, no
  change detection and no hierarchy. It is what an app that wants an
  archetype store for its simulation and nothing else would take.
- **flecs** has the richest hierarchy and relationship model, which is
  what a scene graph wants, but it is a C library: `cargo xtask deny`
  and the `wasm32-unknown-unknown` target rule it out as a vapors
  dependency; an app building natively could use it for simulation.
- **Unity DOTS**'s baking is the one idea with a direct parallel: the
  authoring form (GameObjects there, `.wisp` here) is converted at
  build time into runtime data, and the runtime never holds the
  authoring form. vapors already does this for styles (`StyleKey`,
  `build-time-styles.md`) and plans it for shapes (`one-scene.md`,
  1.3) and glTF (`3d-engine-parity.md`, E4). Its chunk-level change
  filtering is coarser than vapors' per-node version.

## 4. The options

**A. No ECS: the scene store extended.** `vapors-scene3d` keeps its
`NodeId` API and closed `NodeKind`, and gains: columns for the
per-node fields (the (L) part), a change log or structural version
readers key kept results on, a kept draw list in the renderer (no tree
walk per render), a batched `set_transforms`, and ordered frame tasks.
No new dependency.

**B. An ECS for simulation only, beside the UI tree.** The app's game
state lives in a world the app owns (hecs, `bevy_ecs`, or a
struct-of-vectors by hand), stepped in a frame task, and written to
the scene through A's batch once per frame: the world holds a
`NodeId` per rendered entity. vapors holds no ECS; it holds the hook
and an example. The racer's `Race` struct is already this option with
one entity.

**C. An ECS as the scene store.** `Scene` becomes an ECS world (most
plausibly `bevy_ecs`): `Transform`, `GlobalTransform`, `Mesh`,
`Camera` are components; `<Mesh>` spawns an entity; the renderer is a
query over `(&GlobalTransform, &Mesh, Changed<GlobalTransform>)`; the
hierarchy is `ChildOf`. The template is one spawner among any number;
app code may add components the template never names.

**D. An optional crate integrating `bevy_ecs`.** A `vapors-scene3d-bevy`
crate that an app depends on if it wants: a system that copies a
world's `GlobalTransform` of entities carrying a `SceneNode(NodeId)`
component into the scene through A's batch each frame, and the
reverse for picking results. B with the glue shipped by vapors.

A is the base of B and D; C replaces A.

## 5. How each option fits the rest of vapors

### 5.1 `.wisp` and the compiler's analysis

- **A, B, D.** The template names every node kind a scene has
  (`<Mesh>`, `<Light>`); G3's `SceneUses` is computable per crate. A
  simulation under B creates nothing the renderer draws: it moves
  nodes the template made (through a handle out of the component,
  slice E4) or nodes it added under a template `<Group>` with
  `Scene::add`, which is the same visibility question M3 already
  faces for `Scene::add` from Rust (`3d-engine-parity.md`, risk 2) and
  gets the same answer: the app declares the kinds it adds, or adds
  through `<Instances>`/`{for}` where the template sees them.
- **C.** Which components an entity has is a run-time fact; an app
  can `insert` a `Light` the crate's templates never name. `SceneUses`
  then needs a registry of allowed component sets, a second source of
  truth, and a spawn outside it is refused at run time rather than at
  build time. The template becomes one of several descriptions of the
  scene, against `3d-engine-parity.md` section 4's "`.wisp` is the
  scene description; there is no second one". CSS reaches nodes
  through anchors (`css-for-3d.md`, section 1); an entity spawned from
  code has no anchor, so no cascade, no `:hover`, no a11y node
  (ARCHITECTURE, "5 a11y and 9 3D": scene objects with listeners are
  a11y nodes). C makes two kinds of scene object.
- **Generated code, shared runtime.** A, B, D: the compiler emits
  `Scene::add` and effects as today. C: a `Query` is monomorphised per
  component set, and the renderer and each picking or culling reader
  is its own instantiation; that is the library's code, not the
  template's, so the rule holds for generated code but the binary
  carries one copy per query shape.

### 5.2 Idle at zero

- vapors draws a frame only on an invalidation or an animation
  request (`crates/app/src/elements_app_decision.rs:57`). An ECS world
  does nothing on its own; it is the scheduler that runs systems
  every frame (Bevy's `Update` runs each frame unless `run_if`
  gates it). Under A, B and D nothing runs without a frame, and a
  simulation holds an animation guard while it runs (particles P6),
  dropping it when it stops, so the app returns to `Next::Wait`. Under
  C with `bevy_ecs`'s schedule used as the frame loop, idle would run
  every system's query matching per frame (cheap but not zero) unless
  the schedule is gated; under C with the world used as storage only
  (no schedule), idle is zero as under A.

### 5.3 The frame loop

- Design 02's phases are fixed; simulation belongs in the tick
  (ARCHITECTURE, "2 scheduler and 9 3D": "scene bulk tasks run in the
  tick"). A and B put the simulation task there, with
  `SurfaceFrame::now` (particles S1) and the step since the last
  frame. A fixed-step accumulator is the task's own (parity E8 (a)),
  as the racer's `step_fixed` is today.
- (S) under C would be a second ordering inside one phase. Bevy's
  `before`/`after` on named systems is what the parity plan calls the
  second answer the analysis cannot see. Note that T1 as the parity
  plan words it ("named stages before/after") is itself a small
  scheduler by name; decision E3 below asks whether the stage list
  should instead be a closed enum.

### 5.4 Threads

- The threading rule gives the scene to the UI thread. (S)'s
  parallelism over scene data is therefore unavailable under any
  option, and Bevy's executor is single-threaded on wasm anyway.
- What is available: a simulation that owns its data can run off the
  UI thread as owned data through `offload`
  (`docs/design/01-reactivity-async.md`, "One off-thread call") and
  return the frame's transforms as a `Vec<(NodeId, Transform)>`, which
  the tasks phase applies through the batch. That is Bevy's
  extract-to-render-world shape (the render world is a copy) and
  DOTS's job-to-main-thread sync, in vapors' terms. On the web without
  shared memory it is a structured clone per frame; on the UI thread
  it is a call. Under B the app decides per simulation; nothing in
  vapors changes for it except that the batch is the only write path
  a task uses.
- A `!Send` scene (through `Rc<RefCell<Scene>>`,
  `scene3d-gpu/src/renderer.rs:17`) stays `!Send`; under C a
  `bevy_ecs::World` is `Send + Sync`, which invites systems over it
  from other threads and would need a wrapper to refuse them.

### 5.5 The web build

- A, B (hand-written or hecs): no new dependency in vapors; the app's
  simulation crate is the app's size cost. hecs's size on wasm: not
  measured; it has no transitive dependencies beyond `hashbrown`.
- C, D (`bevy_ecs`): the crate and its transitive dependencies in
  every page that uses the scene (C) or the glue (D). Not measured.
  Method: `cargo xtask web` on `vapors-web-demo` with and without the
  crate, brotli bytes, as `complex-script-breaks` was measured
  (`crates/text/Cargo.toml:13-26`, `docs/design/text-stack-size.md`).
  touché's page budget is about 1.7 MB (`check_dist`,
  `3d-engine-parity.md`, risk 3); the racer page is 7.7 MB
  uncompressed today (same, 1.1).

## 6. Costs: measured, estimated, and how to measure

Nothing here was measured for this document. Each row says what was
read from the code and what would measure it.

| Cost | Today, from the code | Estimate | Method |
|---|---|---|---|
| CPU per render, scene unchanged except one node, N mesh nodes at depth d | `mesh_nodes` walk N, `assign` hashes N twice, `world_version` walks d ancestors per node, `holds` hashes N; about 4 `Vec` allocations per render (section 2.2) | O(N × d) with about 3 hash operations per node; at N = 1,000, d = 3, on the order of tens of microseconds; at N = 10,000, on the order of a millisecond. Not measured | A `vapors-perf` workload: a `<Scene>` of N meshes under groups of depth d, one `transform` signal written per frame; instruction counts at 1,000, 3,000 and 10,000 as `--scaling` reports for rows (`perf-finish-line.md`, "Properties 1 and 6"); the judge is the same 2 % and 5,000-instruction rule |
| CPU per render after E1 and E2 | A kept draw list keyed by the scene's structural version; `world_version` read without the ancestor walk | O(changed) plus one compare per draw-list entry, no allocation; the compare could go too with a changed list from `update` (E1 names it) | The same workload; the row must be flat within the judge's rule |
| Memory per node | `Node` holds `NodeKind` (its largest variant `Camera { handle, projection }`), `Transform` (40 bytes), `Mat4` (64), two ids, a `Vec` header (24) and flags; not summed here | Over 200 bytes per node plus the children `Vec`'s heap block. Not measured | `std::mem::size_of::<Node>()` in a unit test pinning the number, and the heap bytes of a 10,000-node scene from `vapors-perf`'s RSS report (`crates/perf/src/rss.rs`) |
| Memory per node under columns (E3) | | `Transform` 40 + `Mat4` 64 + version 8 + parent 4 + first child 4 + next sibling 4 + kind index 4 + flags 1, about 130 bytes, no per-node heap block. Not measured | The same pin after E3 |
| One signal-driven transform write | Section 2.3's list: runtime borrows, observer marking, a sort of pending effects, the effect's scope disposal, the scene borrow, mark, invalidate | A few hundred nanoseconds to a microsecond per node per frame; 1,000 nodes animated through signals would cost on the order of a millisecond per frame before the renderer runs. Not measured | The workload above with every node's signal written per frame, against a variant that writes the same transforms through the batch (E4); the difference is the graph's cost per node |
| `bevy_ecs` on the web page | | Not measured; the parity plan's method | `cargo xtask web` with and without, brotli bytes, in the PR that would add it (only under C or D) |
| Bevy's change detection versus vapors' | Bevy: a tick per component slot, set on `DerefMut`; vapors: a version per node, set on write | Equal in bytes (8 per node either way, 4 under Bevy's `u32` ticks); Bevy's marks on touch, so a system that iterates `&mut Transform` to read it marks everything; vapors' marks on write only | A unit test exists in spirit: `world_version_tests.rs` pins that an unchanged node's version does not move |

## 7. Recommendation and slices

**Recommendation: A, with B as the stated path for a game's
simulation, and D only when a second app asks for it.** Reasons, each
to a section above:

- The parts of an ECS vapors needs, (L) and (C), are reachable inside
  `vapors-scene3d` with no dependency and no API change (5.1, 6).
- (S) is refused by the threading rule and the analysable-sources
  rule, and gives nothing on the web (5.3, 5.4).
- C makes two kinds of scene object and a second scene description
  (5.1); its benefit over A is open composition, which the closed
  `NodeKind` deliberately does not offer.
- A game's own entities (enemies, bullets, pickups) have data the
  scene never needs (health, AI state); that data belongs in the app's
  world, and an app may choose an ECS crate for it today with no help
  from vapors beyond the batch write and a handle to the nodes it
  drives (B). One example proves the hook; a glue crate (D) would pin
  vapors to `bevy_ecs`'s release cadence for one consumer.

### Slices

Each row is under about 400 lines (estimate), built by a fresh agent,
each check proven by breaking what it guards. All after the finish
line, 3D PRs 4b and 5, and one-scene R3a2 (the racer on
`SceneRenderer`, the first scene with a real simulation). Where a row
overlaps the parity plan's, the parity row is named and this one
replaces its wording.

| PR | Content | Accepted when | Depends on |
|---|---|---|---|
| E1 | A kept draw list in `SceneRenderer`: `Scene` gains a structural version (moved by `add`, `remove`, `reparent` and a kind change) and `update` returns the changed nodes as today; the renderer rebuilds its mesh list only when the structural version moved, and writes slots from `update`'s returned list and `set_appearance`'s (a colour change joins the dirty list rather than only moving the version). `SlotTable::assign` runs only on a structural change | The `vapors-perf` scene workload (section 6, row 1) added, with N = 1,000, 3,000, 10,000; one moving node: instructions flat within the judge's rule; `FrameCounts` unchanged (#252's pins hold); a node added from Rust after mount is drawn on the next render (the structural version proven by removing the bump and watching the test fail) | 3D PR 5 |
| E2 | `Scene::world` and `world_version` without the ancestor walk: a scene-level `pending` count (the dirty list's length) answers `WorldPending` in O(1) after an update; the ancestor walk stays only for the error's message | `world_version_tests.rs` and `world_tests.rs` unchanged and green; the workload's `update`-plus-render instructions per node drop by the walk (recorded in `docs/perf/`); `WorldPending` still refused for a node changed since the last update (proven by breaking) | E1 |
| E3 | Columns: `Scene`'s per-node fields split into `Vec`s (`local`, `world`, `version`, `parent`, `first_child`, `next_sibling`, `kind`, `dirty`); children as a sibling chain, not a `Vec` per node; `NodeId` and every public method unchanged; a `size_of` pin per node | Every `vapors-scene3d` and `vapors-scene3d-gpu` test unchanged and green; the per-node bytes pinned in a unit test at the number section 6 estimates or the measured one; the 10,000-node mount's RSS recorded before and after | E1 |
| E4 | `Scene::set_transforms(&[(NodeId, Transform)])` (parity T2's batch) with the one-writer rule: a `<Mesh>` or `<Group>` whose `transform` prop is bound marks its node `bound` in the scene, and the batch refuses a bound node naming the node and the component; a node mounted without a binding is task-owned. Since a props struct fills every unset prop from `Default` (#251), `transform` becomes `Option<Signal<Transform>>`, `None` meaning task-owned; and a `node` out-prop (`bind:node={handle}`, the shape of `bind:this`) hands the `NodeId` to app code, since nothing does today (`mesh.rs`, `Mounted.placement` is private) | 1,000 nodes moved in one call recompute each world once (counted by `update`'s return); a bound node in the batch refused naming both; a template `<Mesh bind:node={..}>` gives the app the id that the batch then moves; proven by breaking the `bound` mark | E1, 3D PR 5 |
| E5 | Ordered frame tasks on the scene (parity T1), with the stage list shaped by decision E3 below: a task registered for a stage, given `now` and the step since the last frame, holding an animation guard (particles P6) while registered; tasks run in the tick phase in stage order, registration order within a stage | Two tasks in two stages run in stage order every frame (pinned); a task in one stage registered after another runs after it; a task dropping its guard returns the app to `Next::Wait` and 0 idle frames; a task writing a signal is refused in debug naming the task (design 02: "a tick that reads signals is a bug") | particles S1 and P6, E4 |
| E6 | The example of B: a demo scene (`crates/demos` or the racer's test page) of 1,000 cubes driven by a `hecs` world (decision E4) stepped in a simulation task, each entity holding its `NodeId`, written through `set_transforms`; `cargo xtask deny` passing with `hecs` pinned exactly; the frame's instructions and bytes recorded | Instructions per frame at 1,000 and 10,000 entities recorded in `docs/perf/`; bytes written per frame equal 80 × moved nodes (pinned); 0 frames after the task stops; the demo page's wasm size with and without `hecs` recorded in `docs/web.md` | E4, E5 |
| E7 | The off-thread variant of E6: the world stepped through `offload` as owned data, the result applied in the tasks phase; the same pins; the hop's cost per frame recorded on native and on the web (a copy) | The same demo passes with `?offload=1`; frame-to-idle (one-scene D6) recorded for both; the web copy's bytes per frame recorded | E6, design 02's S4 (`offload`) |

E1 to E3 serve every 3D row after them (culling's kept visible set,
instancing's per-node column, picking's visible list). E4 and E5 are
the parity plan's T2 and T1 with the changes decided below. E6 and E7
are the proof that B works and the numbers Tom's direction needs; they
are not product features.

## 8. Decisions for Tom

Sections 4 to 7 follow each recommendation.

**E1. Which option.** (a) A plus B: extend the store; the app's
simulation owns its data, in an ECS crate if it wants one; one example
in vapors; no glue crate. (b) C: `bevy_ecs` as the scene store. (c) A
plus D: as (a) and a `vapors-scene3d-bevy` glue crate now. (d) Nothing
beyond the parity plan's T1 and T2: leave the store's layout and the
per-render walk until a measurement shows them. Recommendation: (a).
(d) is defensible on "nothing is measured" but the walk is O(nodes)
by inspection and the perf workload in E1 is the measurement; (b) and
(c) are weighed in section 5.

**E2. Columns now or after a number.** (a) E1 and E2 first (the
algorithmic change, O(nodes) to O(changed)), then E3 (the layout) as
its own slice regardless, because instancing (parity I1) and culling
(C1) want a contiguous world-matrix column to copy from and test
against. (b) E3 only if E1's workload shows the layout matters at
10,000 nodes. Recommendation: (a): the slice is mechanical, API
preserving, and the later rows are simpler on columns.

**E3. The shape of task ordering.** (a) A closed enum of stages
(`Stage::{Simulate, Camera, Resources}`, extended by a PR when a use
appears), tasks ordered by stage then registration: no names, no
`before`/`after`, no cycle possible, the order visible in the type.
(b) Named stages with `before`/`after`, as design 09 ("before or after
another task by name", Threlte's `useTask`) and parity T1 word it,
with a cycle refused at registration. Recommendation: (a). It is the
analysable-sources rule applied to ordering: the set of stages is an
enum, as classes are. (b) is more flexible and is what the parity
plan's section 4 argues against for an ECS; the argument applies to it
too. This changes parity T1's wording.

**E4. Which ECS crate the example uses.** (a) `hecs`: storage and
queries only, no scheduler, no change detection; about the smallest
dependency that is an archetype store; the example then shows the
batch carrying every moved entity, which is the cost an app pays. (b)
`bevy_ecs`: change detection lets the example write only `Changed`
entities, and its schedule could run the simulation, but it brings the
scheduler vapors refuses and a larger wasm delta. Recommendation: (a)
for the example; the doc beside it says an app may pick either, and
what each costs by the example's method.

**E5. A glue crate for `bevy_ecs` (option D).** (a) Not now: the
example documents the glue (a `SceneNode(NodeId)`
component and one function writing the batch); a crate pins vapors to
`bevy_ecs`'s quarterly releases for one consumer. (b) Now, so a game
built on Bevy's ECS starts from a crate. Recommendation: (a) until a
second app, or an outside user, asks.

**E6. Open or closed node kinds.** (a) `NodeKind` stays a closed enum;
a game's per-entity data (health, AI state, inventory) never enters
the scene; the scene holds what is drawn and picked. (b) An
`Extra: Box<dyn Any>` or a component map per node so a game can hang
data on scene nodes. Recommendation: (a): (b) is C's open composition
by another route, and the analysis cannot see it.

**E7. Where a simulation runs first.** (a) In the tick phase on the UI
thread (E5, E6); off-thread as owned data (E7) measured second. (b)
Off-thread from the start. Recommendation: (a): the web has one thread
by default, the racer's simulation is a few microseconds per step, and
E7 gives the number before any app needs it.

## 9. Risks

- **The per-node signal path stays the only authoring form.** Until E4
  lands, `<Mesh transform={signal}>` is the only way to move a node
  from a template, and an author who animates 1,000 of them through
  signals pays section 6's per-node graph cost every frame. E4's
  `Option<Signal<Transform>>` and the task-owned rule are the fix; the
  docs beside `<Mesh>` must say which form is for what, as design 09
  does ("per-object signals are for state that is per object by
  nature").
- **A handle out of a component.** E4's `bind:node` is a compiler
  change (a new out-prop form on component tags) or a convention on
  props; the nested-component gap (#251, "Found") shows component-tag
  codegen has rough edges. If the out-prop is not available in time,
  the example adds its nodes under a `<Group bind:node>` with
  `Scene::add`, which is the M3 visibility question again.
- **The structural version and `set_appearance`.** E1 moves colour
  changes onto the dirty list so the renderer stops scanning for them;
  `#250` made `set_colour` (now `set_appearance`, C3c) move
  `world_version` precisely to avoid a scan.
  Both must agree, and the test that one colour change writes 80 bytes
  (#252) is the pin.
- **Columns and the sibling chain.** Child order matters
  (`3d-2.md`, PR 2; `world_reparent_tests.rs:72`: a moved node goes last). A
  sibling chain keeps it with O(siblings) removal; a scene with one
  group of 10,000 children removes a child in O(10,000). E3 pins the
  removal's cost in the workload and says so; a doubly linked chain
  (`prev_sibling`) makes it O(1) for 4 more bytes.
- **`hecs` or `bevy_ecs` licences and `unsafe`.** Both are MIT or
  Apache-2.0 and both have `unsafe` inside; `cargo xtask deny` decides.
  flecs is out for the web target (section 3).
- **"Suitable for games" is wider than this document.** Physics,
  audio, assets and lights are the parity plan's rows; this document
  answers only where a game's data lives and how it reaches the scene
  per frame. If a game needs the scene itself to carry per-entity
  state (E6 (b)), the closed kind is the thing to revisit, with a
  number from E6's demo.
- **The plans on PR #256 may change.** Parity T1 and T2 are cited by
  their wording on `docs/design-batch` at the time of writing; if
  #256 merges with different rows, E4 and E5 here are re-read against
  them before either starts.
