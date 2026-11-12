# GPU particles in the 3D scene

Scheduled after the performance finish line
(`docs/plans/perf-finish-line.md`): no row here starts until every
property there holds, measured, unless Tom moves it. Plan only; nothing
here is built. Read from the code at 9b405f9 and from open PRs #250
(1337128), #251 (3cd0696) and #252 (0712d8f). Line counts per PR are
estimates. Every number is cited or marked "to be measured"; no build
was run for this plan.

## Decided by Tom, 2026-11-12

- The simulation runs on the GPU, in compute shaders, from the start.
  There is no CPU simulation path in the product.
- A small CPU reference of the simulation is kept as a test oracle
  only: a pure function (emitter settings, seed, time) to each
  particle's position, age and colour, unit-tested without a GPU. A
  debug-build check reads back a sample of particles from the GPU and
  panics naming the particle and the field on a mismatch: the same
  pattern as the planned winners oracle (`docs/plans/build-time-styles.md`,
  "Loud checks") and #249's compare of kept bytes
  (`painter_kept_check.rs`, which panics naming the batch, instance,
  attribute and byte).
- Long term, Tom wants 3D in vapors that is eventually suitable for
  games. Nothing in this plan may rule that out.

Sections 1 to 6 follow the recommendation of each open decision in
section 8; where another option would change a row, the decision says
which.

## 0. Where things stand

- **No compute in vapors.** No crate creates a compute pipeline,
  dispatches workgroups or has a `@compute` entry point (searched at
  9b405f9). The only `STORAGE` buffer is in
  `crates/gpu-harness/examples/parallel_devices.rs:93`.
- **Devices.** wgpu 30 (`Cargo.toml:17`). Every device, window and
  test, asks for the default limits with `max_buffer_size`,
  `max_texture_dimension_2d` and `max_storage_buffer_binding_size`
  raised to the adapter's (`crates/gpu/src/limits.rs:40`), and for
  `TIMESTAMP_QUERY` when offered (`crates/gpu/src/features.rs`). Compute
  needs no optional feature. wgpu 30's defaults
  (`wgpu-types-30.0.1/src/limits.rs:421`) are the WebGPU defaults: 8
  storage buffers per stage, 128 MiB per storage binding, 256 MiB per
  buffer, 256 invocations per workgroup (256 in x), 65,535 workgroups per
  dimension, 16 KiB of workgroup storage.
- **The GPUs the checks run on.** The gate hides `/dev/dri`
  (`docs/decisions/2026-11-02-gate-hides-gpu-device.md`): the gpu,
  screenshot and oracle steps run on llvmpipe (Vulkan, lavapipe), and the
  web checks run headless Chromium 155 with its own SwiftShader Vulkan
  driver (`scripts/browser.sh`, `scripts/check-page-contract.sh`). An
  implementer's `cargo xtask ci` runs the gpu tier on the Intel UHD 630
  (ANV). WebGPU compute on SwiftShader and on the Pixel 9 Pro is
  unverified (`docs/briefs/NATIVE_SVG_BRIEF.md:65`, option B).
- **The 3D renderer.** `SceneRenderer` (`crates/scene3d-gpu`) is a
  `SurfaceRenderer` on a `gpu-surface`. Each render updates the scene,
  uploads missing geometry, writes changed mesh uniforms and records one
  render pass that clears colour to transparent and depth to 1.0, then
  draws each mesh node, one draw per node
  (`crates/scene3d-gpu/src/draw.rs:59`). The depth attachment's store is
  `Discard` (`draw.rs:92`), so anything depth-tested against the meshes
  must be drawn in that pass. Node kinds are `Group`, `Mesh` and
  `Camera` (`crates/scene3d/src/node.rs`).
- **Instancing.** The 2D painter's `Instanced` pipeline draws one quad
  per instance, 6 vertices, from a per-frame `FrameArena`
  (`crates/renderer/src/instanced.rs:16`, `:69`). The arena grows up to
  `max_buffer_size` and refuses a draw past it as `ArenaFull`
  (`crates/renderer/src/frame_arena.rs`). #247 adds `Retained`, which
  keeps bytes between frames and refuses the same way. Nothing in the
  3D renderer is instanced yet (design 09, slice T5).
- **Components.** #250 to #252 add `vapors-scene3d-components`:
  `<Scene>`, `<Group transform>`, `<Mesh geometry colour transform>` and
  `<Camera>` as Rust components with typed props, each with an inert
  anchor `div`. A prop change writes one node and invalidates the
  surface. A generated `.wisp` component cannot yet be nested in another
  component's children (#251, "Found"); the scene components avoid it
  by taking `&impl DynDom`.
- **Idle.** After each frame `ElementsApp` asks for the next only when
  the document changed, a surface was invalidated, or the painter wants
  a frame (`crates/app/src/elements_app_decision.rs:34`); otherwise
  `Next::Wait` or `Next::WaitUntil` (`:41`). A surface renders every
  frame only while its `SurfaceHandle::request_animation()` holds, which
  is one flag per surface (`docs/gpu-surface.md`, "Animation"). The app,
  not the renderer, decides when a surface animates. The racer stops its
  animation from a frame-times hook
  (`crates/racer/src/app/surface.rs:58`).
- **Time.** `Frame::now` is read once per frame
  (`crates/platform/src/window.rs:50`); `vapors-perf` advances it 16 ms
  per frame (`crates/perf/src/sampler.rs:18`). `SurfaceFrame` carries
  no time (`crates/app/src/surface.rs:40`).
- **Counting.** A renderer's writes and draws issued through
  `SurfaceFrame::work` land in `FrameCounts::surfaces`
  (`crates/app/src/painter_counts.rs:42`). `GpuWork` counts buffer
  bytes, texture bytes, draw calls, uniform writes and bind groups made
  (`crates/renderer/src/work.rs:12`); it has no dispatch count. GPU
  time from `GpuTimer` covers render passes only
  (`crates/renderer/src/gpu_time.rs:32`).
- **Readback.** `read_rgba8` blocks on a map
  (`crates/renderer/src/readback.rs`), usable in native tests only.
  `GpuTimer` reads back without waiting through a ring of 4 slots
  (`crates/renderer/src/gpu_timer.rs:21`), which also works on the web.
- **Backing format.** A surface's backing is `Rgba8Unorm` with straight
  alpha, composited with ordinary alpha blending
  (`crates/app/src/surface.rs:10`).

## 1. Compute infrastructure

The first compute pass is proven on each GPU the checks use before any
particle code depends on it.

- `GpuWork` gains `dispatches` (calls to `dispatch_workgroups`) and
  `invocations` (workgroups times workgroup size, known on the CPU).
  `WorkCounter::dispatch(pass, workgroups, size)` issues and counts one,
  as `WorkCounter::draw` does for draws. Both reach
  `FrameCounts::surfaces` through `SurfaceFrame::work` with no change to
  the painter.
- A compute pass is recorded into `SurfaceFrame::encoder` before the
  surface's render pass. The painter already submits that encoder inside
  the surface's GPU error scope (`crates/app/src/surface.rs`, the
  `SurfaceRenderer` docs), so a validation error in the compute pass is
  refused as the renderer's failure, naming the surface, with no new
  code.
- Proof on native: a gpu-tier test runs a compute shader that writes
  `i * 3 + 1` into element `i` of a 4,096-element storage buffer and
  reads it back. It runs on llvmpipe in the gate and on ANV in an
  implementer's `ci`.
- Proof on the web: a `?demo=compute` page in `crates/web-demo` runs
  the same shader, maps the result without blocking, and writes
  `compute: ok 4096` or the first wrong index and value to `#status`. A
  `check-page-contract.sh` case requires the line. The Pixel 9 Pro is
  checked by hand on the tailnet page and the result recorded in
  `docs/web.md`.
- GPU time: the compute pass writes timestamps
  (`ComputePassTimestampWrites`) into the frame's `GpuTimer` slot as a
  render pass does, so `GpuReading::time` includes compute. Without it a
  frame's GPU time would leave the simulation out.

## 2. The simulation

### 2.1 Settings and liveness

An emitter's settings, validated on the CPU when given (as `Meshes`
validates geometry, `crates/scene3d/src/meshes.rs`), refusing
non-finite or out-of-range values by field name:

- `rate`: particles per second, an integer, 0 or more;
- `lifetime`: a `Duration`, the same for every particle of the emitter
  in the first version (decision P4);
- initial velocity: a direction, a cone half-angle and a speed range;
- `acceleration`: a constant vector (gravity, wind);
- start and end colour, start and end size, interpolated linearly over
  age;
- `seed`: a `u32`.

Which particles exist is decided on the CPU, in integers, once per
frame. With `t` the nanoseconds since the emitter started, particles
`k` with `floor((t - lifetime) * rate / 1e9) <= k < floor(t * rate /
1e9)` are alive, the lower bound 0 until one lifetime has passed and
the upper bound fixed at its value when emitting stops (u128
arithmetic, then checked to fit `u32`). The range goes to the GPU in
the emitter's uniform. The GPU never compares a float
age with the lifetime to decide life, so the CPU oracle and the GPU
agree exactly on which particles exist; only continuous fields carry
rounding.

### 2.2 Closed form, no integration state

A particle's fields are a closed-form function of its spawn record and
its age (decision P1): `position = p0 + v0 * age + acceleration * age^2
/ 2`, colour and size interpolated by `age / lifetime`. Random values
come from a 32-bit integer hash (PCG) of `(seed, k, field)`, converted
to `[0, 1)` by taking the top 24 bits times 2^-24. Both steps are exact
in `u32` and `f32` on the CPU and in WGSL, so the CPU and GPU draw the
same random numbers.

No atomics are used. Slot `k mod capacity` holds particle `k`, where
`capacity = ceil(rate * lifetime)`. Each frame one invocation per slot
works out which particle its slot holds now and writes that particle's
instance.

### 2.3 Space

The first slices use a static emitter, so emitter space and world space
are the same. A moving emitter (exhaust behind the car) needs particles
to stay where they were emitted: each slot keeps a spawn record (world
position and velocity at spawn), written by the frame in which `k`
first falls in the live range, from the emitter's world matrix
interpolated between the previous and current frame by the particle's
spawn time. Then the oracle's input is the list of frames, each a time
and the emitter's world matrix, not the time alone (decision P2).

### 2.4 Buffers

Per emitter, made at mount and kept:

- the particle buffer, `STORAGE | VERTEX`, `capacity` instances of 48
  bytes (position and age, colour, size and spawn index; the layout is
  fixed in PR S2);
- with world space, the spawn-record buffer, `STORAGE`, 32 bytes per
  slot;
- one uniform slot: time, live range, settings, the emitter's world
  matrices.

Per frame the CPU writes the uniform slot alone, so the bytes written
do not depend on the particle count (section 5). Capacity changes
(new settings) make the buffers again, counted.

### 2.5 Limits

Each limit is refused at mount, naming the emitter, the number, the
limit and the fix, never clamped:

- `capacity` over the emitter budget (decision P3);
- the sum of live emitters' capacities over the scene budget;
- a particle buffer over the device's `max_storage_buffer_binding_size`
  or `max_buffer_size`, read from `device.limits()` as `FrameArena`
  reads `max_buffer_size`;
- workgroups over `max_compute_workgroups_per_dimension` (65,535 by
  default; at 64 invocations per workgroup, 4,194,240 slots);
- a live range whose spawn index passes `u32::MAX`, refused at the
  frame it happens (at 10,000 per second, after about 5 days).

## 3. Drawing: instanced billboards

- In `SceneRenderer`'s pass, after the meshes, with depth test on and
  depth write off, one draw per emitter: 6 vertices per instance,
  `capacity` instances, the particle buffer bound as an instance vertex
  buffer (as `Instanced` binds its arena). Binding it as a vertex
  buffer, not as a storage buffer read in the vertex stage, keeps the
  vertex stage free of storage bindings.
- The vertex shader expands each instance to a quad facing the camera,
  from the view's right and up vectors, and moves a dead slot's
  vertices outside the clip volume.
- Size is in scene units, so it scales with an orthographic camera's
  `height` and with distance under perspective.
- The quad is a round soft sprite computed in the fragment shader; no
  texture in the first version.

### 3.1 Blending

- **Additive first** (decision P5): colour `ONE, ONE` times the
  particle's alpha. The sum does not depend on order, so nothing is
  sorted.
- The backing has straight alpha and is composited over the page
  (`crates/app/src/surface.rs:10`). Additive light is right only where
  the scene has drawn something opaque under it. Over a transparent part
  of the backing, the composite is not additive. The racer's ground
  covers the whole surface (`docs/plans/one-scene.md`, 2.2); a UI effect
  over HTML would not (decision P0).
- **Alpha with sorting** is a later row: a GPU sort by view depth
  (bitonic, in compute) each frame, then `SRC_ALPHA,
  ONE_MINUS_SRC_ALPHA`. Sorting on the CPU would read particles back and
  is not an option.

## 4. `<Particles>` and its properties

- `vapors-scene3d` gains `NodeKind::Particles { emitter }`, naming an
  `EmitterId` issued by an `Emitters` store that validates settings, in
  the shape of `Meshes` and `GeometryId`.
- `vapors-scene3d-components` gains `<Particles emitter={Some(id)}
  emitting={signal} transform={signal}>` as a Rust component like
  `<Mesh>` in #252, with the same refusals at mount (no emitter, no
  enclosing scene).
- **Props now.** Settings are a prop (`Signal<EmitterSettings>`); a
  change writes that emitter's uniform, or makes its buffers again when
  the capacity changes, counted.
- **CSS after one-scene PR 4b** (`docs/plans/3d-2.md`, row 4b): the
  anchor's computed `color` and `opacity` multiply every particle's
  colour, one uniform write per change; `visibility: hidden` stops the
  emitter's draw and dispatch. Rate, lifetime and forces stay props:
  they change the simulation's shape, and the analysis rule says a
  class is a bool or an enum, never a string
  (`docs/decisions/2026-11-11-analysable-sources.md`).
- A `.wisp` component holding `<Particles>` inside another component's
  children needs the compiler change #251 names, which one-scene R3a1
  needs too.

## 5. Idle and frame cost

### 5.1 When frames are asked for

- An emitter is live from the first render after `emitting` turns true
  until `lifetime` after the render in which `emitting` turned false.
  Both ends are known on the CPU from the settings and the frame times.
  Nothing is read back to decide it.
- `SurfaceFrame` gains `now`, the frame's `Frame::now`, so the
  simulation's time is the app's frame time (16 ms steps in
  `vapors-perf`, stepped in tests).
- While any emitter in a scene is live, its `<Particles>` holds an
  animation request on the scene's surface. Requests from several
  owners (several emitters, the racer's own) must not cancel each
  other; today one `stop_animation` ends every request on the surface
  (decision P6).
- The request ends in the first frame whose `now` is at or after the
  last live emitter's end. That frame draws the last particles dying,
  and the app then returns `Next::Wait`.
- No emitter live: no compute pass, no particle draw, no frames
  (finish-line property 6). Checked by `idle_fps`-style counting, as
  `surface_gradient --check` counts (`docs/gpu-surface.md`).

### 5.2 Frame-cost tests

- A frame with one live emitter writes the same number of buffer bytes
  at capacity 1,024 and 65,536: the camera and emitter uniforms. No
  per-particle byte is written from the CPU. Pinned in a gpu-tier test
  from `FrameCounts::surfaces`.
- `dispatches` equals the live emitters, `invocations` equals the sum
  of their capacities rounded up to workgroups, and draws equal the mesh
  nodes plus the live emitters.
- After the last emitter ends: one more frame, then `Next::Wait`, then
  zero frames over an idle second.

## 6. The CPU oracle

- A crate with no GPU dependency (`vapors-particles`, next to
  `vapors-scene3d`; decision P7) holds the settings' validation, the
  live-range arithmetic, the hash and `simulate(settings, seed, time)`
  returning each live particle's spawn index, position, age, colour and
  size. With world space (2.3), the input becomes the frame list.
- The WGSL simulation and `simulate` share the constants and the hash's
  test vectors, so a hash changed on one side fails a unit test that
  names the vector.
- Unit tests without a GPU: a particle under gravity alone lands at the
  closed-form point; the live range at the lifetime's boundary, by one
  nanosecond either side; the hash's vectors; each settings refusal.
- **GPU tests** read the whole particle buffer back (blocking, native)
  and compare every live slot with `simulate`: the live set exactly,
  continuous fields within a tolerance stated per field. The tolerance
  is to be measured on llvmpipe, ANV and SwiftShader in PR S2 and
  written into the test with where it came from.
- **Debug builds** (native and web): every Nth frame (N to be measured
  for cost) the renderer copies a fixed sample of 64 slots into a ring
  of readback buffers shaped like `GpuTimer`'s, and a later frame
  compares the mapped sample with `simulate` for that frame's time. A
  mismatch panics naming the emitter, the frame, the spawn index, the
  field, both values and the tolerance. A sample not mapped by the time
  its slot comes round again is counted as not compared
  (`oracle_samples_missed`), never as equal; GPU tests assert it is 0.
  A sample still pending when the scene goes idle is compared on the
  next frame drawn for any reason, or counted as missed at unmount.
- The check is proven by breaking: a sign flipped in the WGSL
  acceleration term must panic naming `position`.

## 7. PR stack

Each row is under about 400 lines (estimate), built by a fresh agent,
each check proven by breaking what it guards. After each row the next
is checked against what it found.

| PR | Content | Accepted when | Depends on | Parallel with |
|---|---|---|---|---|
| C1 | `GpuWork::dispatches` and `invocations`, `WorkCounter::dispatch`; the native compute test (1) | Passes on llvmpipe in the gate and on ANV in `ci`; a shader writing a wrong value fails naming the index | none | everything |
| C2 | `?demo=compute` and its page-contract case; the Pixel 9 Pro checked by hand | The case passes in the gate on SwiftShader; a wrong value in the shader fails the case naming the index; the Pixel result recorded in `docs/web.md` | C1 | O1 |
| C3 | Compute pass timestamps in `GpuTimer` (1) | A frame with one compute pass reads `pairs` one higher; its time is over 0 on ANV | C1 | O1, S1 |
| O1 | `vapors-particles`: settings and refusals, live range, hash, `simulate` for a static emitter (2.1, 2.2, 6) | The unit tests in 6, each proven by breaking (the range off by one at the boundary fails) | none | C rows |
| S1 | `NodeKind::Particles`, `Emitters` and `EmitterId`; `SurfaceFrame::now` | Scene unit tests: a particles node under a group has the group's world matrix; a foreign `EmitterId` refused; `now` equals the frame's `Frame::now` in an `ElementsApp` test | 3D PR 5 (shares `node.rs`) | C, O1 |
| S2 | The compute simulation in `SceneRenderer`: buffers, uniform, one dispatch per emitter; the limits (2.5) | The GPU test in 6 at three times (first frame, mid-life, after the first deaths) on llvmpipe and ANV, live set exact, fields within the measured tolerance; each limit refused naming it | C1, O1, S1 | C2, C3 |
| S3 | Billboards and additive blending in the scene pass (3) | Pixel test: one emitter of known particles over an opaque plane, sprite centres where `project` puts them, overlap brighter than either sprite; a mesh in front hides particles behind it | S2 | none |
| S4 | `<Particles>` with props; `emitting`; the animation request from 5.1 and the decision P6 change | `ElementsApp` test: emitting starts frames, stopping ends them `lifetime` later, then `Next::Wait` and 0 idle frames; a mesh's animation and an emitter's do not cancel each other | S3, 3D PR 4 (#250 to #252) | O2 |
| S5 | Frame-cost tests (5.2) | The four pins in 5.2, each proven by breaking (a CPU write per particle added fails the byte pin) | S4 | O2 |
| O2 | The debug-build sampled check (6) | A flipped acceleration sign panics naming `position` on llvmpipe; `oracle_samples_missed` 0 across the gpu tier; gpu tier time before and after in the PR | S2 | S4, S5 |
| W1 | Particles on the web: a `?demo=particles` page and a page-contract case | The case's screenshot held to a committed `.pixels` file on SwiftShader; 0 frames after the emitter ends (`web-idle-frames.sh`) | S4, C2 | O2, S5 |
| M1 | Moving emitters: spawn records in world space, the frame-list oracle (2.3) | An emitter moved along a line leaves particles where it was, compared with the oracle; a fast move leaves no gaps beyond one spawn interval | S5, O2 | W1 |
| R1 | First use (decision P0, recommendation (a)): racer exhaust from the car's tail while the throttle is held | Pixel test of the trail behind a moving car; the 60 s run of one-scene P0 with exhaust on, no frame over 16.7 ms, in `docs/perf/` | M1, one-scene R3a2 | B1 |
| B1 | Bursts: a count emitted at one time (sparks, confetti) | Unit and GPU tests as O1 and S2 for a burst; liveness ends `lifetime` after the burst | S5 | M1 |
| A1 | Alpha blending with a GPU sort by depth (3.1) | Pixel test: two overlapping translucent sprites in both orders give the back-to-front result; sort checked against a CPU sort of the read-back depths in the GPU test | S5, decision P5 | M1, B1 |
| K1 | CSS on particles after one-scene 4b (4) | A class toggle on the anchor recolours an emitter's particles with one uniform write and no dispatch change | S4, 3D PR 4b | M1, B1 |

Critical path to the first use: C1, O1, S1 (after 3D PR 5), S2, S3,
S4, S5, O2, M1, R1. C2 and W1 carry the web; C3 the timing.

## 8. Decisions for Tom

**P0. The first use.** (a) Racer effects: exhaust while the throttle is
held, later sparks on contact. The racer is the one 3D app in vapors,
its ground is opaque, so additive blending is right everywhere, and it
has the 60 s frame-time run (one-scene P0) to measure the cost. It
needs moving emitters (M1). (b) UI effects such as confetti over HTML:
a `gpu-surface` over the page, mostly transparent, so additive blending
is wrong there (3.1) and alpha blending with sorting (A1) comes first;
it also needs bursts (B1). (c) Game groundwork with no product use: a
demo scene of many emitters, chosen for budgets, not for a user.
Recommendation: (a). It uses what one-scene builds and has a frame-time
judge already; (b) is the larger job for the same machinery.

**P1. Simulation model.** (a) Closed form from a spawn record and age
(2.2): no state carried between frames apart from spawn records, so a
frame's result does not depend on the frames before it, and the oracle
is a direct function. Forces are limited to what has a closed form:
constant acceleration first, linear drag later. (b) Integrated each
frame with a fixed step: any force (turbulence, collisions), but the
oracle must replay every step, results depend on the step, and
GPU-to-CPU differences grow over a particle's life. Recommendation:
(a), with (b) kept open as a per-emitter mode if a game needs
collisions; nothing in (a)'s buffers prevents adding it.

**P2. The oracle's input for moving emitters.** Tom's decision names
(emitter settings, seed, time). A moving emitter's particles depend on
where it was at each spawn. (a) Extend the input to (settings, seed,
frames), each frame a time and the emitter's world matrix; still a pure
function. (b) Keep particles in the emitter's space, so the input stays
(settings, seed, time), and the exhaust moves rigidly with the car, a
visible error. Recommendation: (a), from M1 on; until M1 the
frame list has one matrix and the two are the same.

**P3. Particle budget.** (a) 16,384 per emitter and 65,536 per scene:
65,536 times 48 bytes is 3 MiB, under the 128 MiB WebGPU default per
storage binding, in 1,024 workgroups of 64. Raised later with a
measurement on the UHD 630 and the Pixel 9 Pro. (b) Read from the
device: whatever `max_storage_buffer_binding_size` allows, so the same
page could run on one machine and be refused on another. (c) 1,048,576
per scene now. Fill rate, not the compute pass, is the likely cost of
large additive sprites (not measured). Recommendation: (a): fixed
numbers refused loudly, as the finish line's rule of raising a budget
with a measurement asks.

**P4. Per-particle random lifetime.** (a) One lifetime per emitter in
the first version, so liveness is integer arithmetic on the CPU (2.1).
(b) A random lifetime per particle, which makes "alive" a float compare
on the GPU that can disagree with the oracle at the boundary unless
lifetimes are quantised to whole frames. Recommendation: (a), with (b)
as a later row using lifetimes in whole 1 ms steps decided on the CPU.

**P5. Alpha blending in the first version.** (a) Additive only; alpha
with a GPU sort is row A1, after the first use. (b) Alpha without
sorting: wrong where translucent particles overlap. (c) Alpha with the
sort in the first version, before any use needs it. Recommendation: (a)
under P0 (a); under P0 (b), (c).

**P6. Several animation requests on one surface.** Today
`request_animation` and `stop_animation` set one flag per surface
(`docs/gpu-surface.md`, "Animation"), so an emitter ending would stop
the racer's animation, and the racer stopping would freeze live
particles. (a) `request_animation` returns a guard; the surface
animates while any guard lives, and dropping a guard ends that request.
Every caller of `request_animation` moves to guards in the same PR. (b) Particles never
request; each app adds its emitters' liveness to its own decision.
Every app with particles has to do this right. (c) The painter asks the
renderer whether it wants frames, which departs from touche decision e3
(the app decides). Recommendation: (a). It keeps e3 and makes the
request's lifetime the owner's.

**P7. Where the oracle lives.** (a) A new crate, `vapors-particles`,
with no GPU dependency, used by `vapors-scene3d-gpu`'s tests and its
debug check. (b) In `vapors-scene3d`, beside `Meshes`. Recommendation:
(a): the oracle and the settings' validation change together, and
`vapors-scene3d` stays the scene's maths and nodes.

## 9. Risks

- WebGPU compute on SwiftShader may fail or differ (not verified, 0).
  C2 runs before any particle row depends on it; if it fails, the gate
  cannot check particles on the web, and the question goes to Tom
  before S2.
- WGSL leaves some float operations' precision to the implementation
  (fused multiply-add, division). The tolerances in 6 are measured per
  adapter; if SwiftShader's differ from llvmpipe's by much, the
  tolerance is per adapter, named in the test.
- Large additive sprites near the camera cost fill rate. The budget in
  P3 counts particles, not covered pixels; R1's 60 s run measures the
  real case.
- `GpuTimer`'s ring of 4 slots is reasoned for a native surface's frame
  latency (`gpu_timer.rs:21`); the debug sample ring reuses it, and on
  SwiftShader samples may arrive later than 4 frames. Missed samples
  are counted, never passed; if the count is not 0 in the web check, the
  ring is sized from a measurement there.
- The depth attachment is discarded at the end of the scene pass
  (`draw.rs:92`). Particles drawn in a later pass would need it stored,
  at a cost on tiled mobile GPUs; this plan draws them in the same pass.
- P6 (a) changes the `SurfaceHandle` API; if touche calls it, a touche
  pin bump follows the PR that makes the change.
