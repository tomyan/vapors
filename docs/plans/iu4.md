# IU4: GPU-side paint kept between frames

Property 2 of `docs/plans/perf-finish-line.md` (draw under 1 ms at any
row count; boxes outside the view cost nothing), order of work item 6.
It also takes the scroll remainder of property 4: after 3c-3e a scroll
still shifts every moved part and patches every moved step, O(rows in
the scroller). Read from the code at 7ab3984 (IU3 3b and 3c-3f merged).
Design only; every number marked "measured" is from
`docs/perf/baseline-2026-11-10.md`, every other number is an estimate
to be replaced by PR 0's measurement.

## 1. Where draw goes today

Draw is `ScenePainter::draw` (`crates/app/src/painter.rs:61-64`):
clear, then `paint_runs` (`painter.rs:100-112`) takes
`painted.batches(view)` and, per batch, cuts the target to the clip
(`painter_region.rs:50-57`) and calls `draw_run` (`painter.rs:117-130`),
which dispatches by kind (`painter.rs:134-151`).

Measured: at 3000 rows draw is 0.88 to 0.94 ms wall and 4.25 million
instructions per frame, the same at 1000 rows ("IU3 3b", "IU3 3c-3f").
Every rows workload draws 52 batches: 17 button quads and 35 text runs
for the 17 rows in the 640 x 400 view ("Culling (#184)"); 11,932 items
are culled at 3000 rows. The GPU's own time is 0.27 ms. So draw is CPU
recording, about 17 µs per batch, and already independent of the row
count on a frame whose view and order are unchanged. What each batch
costs, reading the code:

| Step | Where | Per batch |
|---|---|---|
| Kept batches | `painted_batches.rs:21-34`: the same view returns the kept `Batched` | O(1) on an unchanged view; `KeptBatches::of` (`kept_batches.rs:42-60`) is O(steps + items) after a view change or an order walk, with `hidden: vec![0; places]` (line 52) |
| Instance bytes, quads and shadows | `quads.rs:29-32`: `instance` (lines 41-51) allocates a `Vec<f32>` per quad, `float_bytes` a `Vec<u8>` per batch; `shadows.rs:28-32` the same | 2 allocations and 128 bytes per quad, every frame |
| Upload | `Instanced::draw` (`instanced.rs:64-83`): the instance bytes and a 16-byte viewport uniform each go through `FrameArena::push` (`frame_arena.rs:100-118`), one `Queue::write_buffer` each; the arena starts at offset 0 for every encoder (`frame_arena.rs:151-163`), so every frame rewrites everything it draws | 2 `write_buffer` calls |
| Bind group | `Instanced::record` (`instanced.rs:85-101`) creates a bind group per draw for the viewport uniform | 1 `create_bind_group` |
| Text glyphs | `painter_text.rs:14-23` collects the run's layouts; `quads` (`crates/text/src/quads.rs:24-39`) fetches every glyph from the atlas twice (`placed_all`, lines 78-86); each `GlyphAtlas::get` (`atlas.rs:106-113`) builds a `GlyphKey` whose `coords` is a `Vec<i16>` (`atlas_types.rs:146`, `153-164`) and hashes it (`atlas.rs:127-133`); then 48 floats per glyph (`quads.rs:108-131`), converted to clip space on the CPU (`to_clip`, lines 135-140) | about 2 allocations and 2 hash lookups per glyph; at 17 rows about 300 glyphs, so about 600 of each per frame |
| Text upload and bind group | `draw.rs:69-92`: a 16-byte params uniform pushed per draw (lines 85-90), the vertex bytes collected again from the floats (line 134) and pushed; `bind_group` (`pipeline.rs:87-108`) creates a texture view and a bind group per draw | 2 `write_buffer`, 1 `create_view`, 1 `create_bind_group` |
| Images and surfaces | `painter_images.rs:40-53` and `painter_surfaces.rs:127-140` call `draw_rect` per item; `ImagePipeline::quad` (`image.rs:128-150`) pushes 6 vertices and makes a view and a bind group per draw (lines 152-173) | per item: 1 `write_buffer`, 1 view, 1 bind group, 1 draw |
| Pass | `FrameEncoder::draw` (`frame_encoder.rs:106-117`): one pass per frame (measured), the scissor set per draw (`open_pass.rs:55-67`) | set_pipeline, set_bind_group, set_vertex_buffer, set_scissor, draw |

Measured, before culling, at 100 rows idle and 317 draws ("After one
submit and pass merging"): encode 2.33 ms, write 2.26 ms, the rest
1.68 ms, about 7.4, 7.1 and 5.3 µs per draw. Nothing since changed the
per-draw work, so at 52 draws today's 0.9 ms is roughly 0.4 ms of
encode (mostly the bind groups and views), 0.4 ms of writes (about 104
`write_buffer` calls) and 0.2 ms of building (the glyph lookups and
float vectors). That split is the estimate PR 0 replaces.

What still scales with the page: `KeptBatches::of` and `hidden` after a
view change (a resize) or an order walk (an insert until splice B);
the debug build's `assert_same` against `scene().batches(view)` on
every draw (`painted_batches.rs:31-32`, O(items), debug only); and a
scroll, which 3b places as every descendant of the scroller
(`placement_walk.rs:75-96`: the content origin moved, so every slot
placed in it is visited and reported moved, `put` lines 112-125), then
paint shifts every moved part (3c-3e) and patches its steps in the
drawn steps and the kept batches (`paint_partial.rs:71-86`,
`kept_batches.rs:79-90`), O(rows in the scroller) per scroll.

### What to measure, and how

`vapors-perf --instructions` (`crates/perf/src/instructions.rs`) counts
the thread's user-space instructions per `frame phase`, `update phase`
and `draw step` span (`spans.rs:1-8`). The `draw step` spans are
`encode`, `write` and `submit` (`work.rs:68`, `82`; `instanced.rs:87`;
`draw.rs:142`). PR 0 adds `batches` (around `painted.batches`), `build`
(instance and vertex building, the glyph lookups inside it) and `bind`
(views and bind groups, split out of `encode`), and the harness prints
each as it prints the phases. Instructions are the acceptance numbers:
they do not move with load. Wall clock and the real GPU time
(`gpu_reading.rs`) are recorded beside them, three runs, medians.

Workloads: the rows workloads at 1000 and 3000 rows exist
(`rows.rs:24-36`); 10,000 rows is added with the first PR that claims
property 1 (`perf-finish-line.md`), or by IU4 PR 0 if it comes first;
`Scroll` (the list scrolled one row per frame) comes with 3c-3e and
`Insert` with splice PR C. PR 0 adds `Resize` (the view's height
alternating by one row each frame): the view-change path of the kept
batches, which no workload exercises today.

## 2. What the GPU side keeps

Unit: the batch. `KeptBatches` (`kept_batches.rs:25-37`) keeps, for the
last view, the shown steps and `batched: Option<Batched>`, dropped when
a shown step is painted again (`kept_batches.rs:117`, `130`) and made
again from the shown steps' items (lines 99-109), O(shown items). IU4
keeps beside each batch its GPU data:

```
struct Kept {           // per batch, in KeptBatches
    run: Run,           // the items, as today (batch.rs:19-34)
    gpu: Option<Gpu>,   // None until the first draw of this batch
}
struct Gpu {
    range: Range,       // offset and size in the pipeline's retained buffer
    count: u32,         // instances or vertices
    atlas: u64,         // text and icons: the atlas generation the uv's are from
    bytes: Vec<u8>,     // debug builds only: what was uploaded (section 5)
}
```

- When the batches are made again, each new batch is matched with the
  old batch at the same position whose `run` is equal (`Run:
  PartialEq`, `batch.rs:20`); a match carries its `gpu` over, so the
  bytes of a batch whose items did not change are neither built nor
  written. The comparison is O(shown items) and allocates nothing.
- A batch without `gpu`, or whose `atlas` generation is stale, builds
  its bytes on its first draw as today and writes them into a
  `Retained` buffer (below) at a free range; the old range, if any, is
  freed. So per frame, uploads are proportional to the parts whose
  items changed, and the CPU work to the shown items.
- `Retained` (new, `crates/renderer`, beside `FrameArena`): one
  growing buffer per pipeline with a free list of ranges (first fit,
  ranges merged on free), `COPY_DST | VERTEX` (or `UNIFORM`). Writes
  go through `WorkCounter::write_buffer` as today, so `FrameCounts`
  still counts them. A released range is free for new bytes only once
  a newer encoder than the last one written or drawn for writes or
  draws, so a write never lands on a range a draw recorded into the
  open encoder reads, whatever the order of draws, releases and
  writes. When the free ranges fragment past half the buffer, the live
  ranges are compacted into a new buffer in one pass (O(shown
  batches)), counted. Growth doubles, up to `max_buffer_size`
  (section 5).
- Uniforms: one uniform buffer per pipeline per target holding the
  viewport and, after PR 4c, each scroll frame's translation, 256
  bytes apart (the WebGPU and wgpu default
  `min_uniform_buffer_offset_alignment`), written only when the
  viewport or a translation changes; one bind group per pipeline per
  target (and per texture, for images and surfaces), kept while the
  target's format and size and the texture are the same, bound with a
  dynamic offset per batch. The text pipeline's texture view is kept
  with the texture and made again when the page is resized.
- Text and icon bytes: their uv's name the atlas page's rects. A
  repack (`atlas_evict.rs:11-23`) moves glyphs, so the atlas gains a
  generation counter, bumped on repack and on a page clear; a batch
  whose `atlas` is behind it is rebuilt. Glyphs that kept bytes name
  are not fetched each frame, so the atlas's LRU (`cached`,
  `atlas.rs:127-133`; `evict_least_recent`, `atlas_evict.rs:42-60`)
  would not see them as used and could evict them. Each atlas entry
  gains a pin count: building a batch's bytes pins its glyphs,
  dropping the bytes unpins them; eviction skips pinned entries, and a
  page where nothing unpinned can make room repacks after clearing
  every pin and bumping the generation, so every text batch is rebuilt
  on its next draw (the `flush` at `draw.rs:106-108` already submits
  the draws recorded before a clear). Option 3 in section 8.
- Images: textures are already kept per image across frames
  (`painter_images.rs:13-19`); their composition quads become batches'
  retained vertex bytes like any other, and their bind groups are kept
  per texture. Surfaces: the renderer's texture is the surface's
  (`painter_surfaces.rs:136`); the composition quad and bind group are
  kept per surface and remade when `prepare` hands back another
  texture (a resize).

Cost model per frame, after the stack: `batches` O(1) on an unchanged
view; matching O(shown items) on a changed one; building and writing
O(items in batches whose items changed); recording O(shown batches),
each a dynamic-offset bind, a vertex-buffer slice, a scissor and a
draw; nothing per culled item. Memory: GPU bytes for shown batches only
(128 bytes per quad, 112 per shadow, 192 per glyph, 96 per image or
surface quad; about 0.5 MB for 100 visible rows), a `Kept` record per
shown batch, and the atlas pins.

## 3. Scroll-frame coordinates

Today a part's items are in absolute device pixels with the scroll
offsets applied: placement adds each ancestor's content origin, which
is its corner less its offset (`placement_walk.rs:143`,
`placement.rs:23-27`), and paint snaps the absolute rect
(`paint.rs:112-119`). A scroll therefore moves every descendant's rect.

IU4 paints each part in the coordinates of its scroll frame: the
nearest scroll container above it (box 0 for the rest of the page),
with that container's offset, and every outer one's, left out. A
part's items are then as they would be at offset zero, snapped as
today; the painter adds the frame's translation at draw. The result is
pixel-identical to today: every offset is applied rounded to whole
device pixels (`scroll.rs:60`, `78`), and `round(x) - n == round(x -
n)` for a whole `n`, so snapping at offset zero and translating by the
rounded offset gives the edges today's snap of the scrolled rect gives.
(Frames relative to the scroller's own box, which would also make a
scroller moving on the page O(1), snap relative to a possibly
fractional origin and can differ from Chromium by a pixel; option 2 in
section 8.)

Rounding (lead decision, 2026-11-12). The argument above holds only for
a rounding that commutes with a whole shift. `paint::snapped` rounded
half away from zero, which does not at a tie below zero: a row whose
top is at 37.5 device pixels in its frame, scrolled by 39, snaps to
38 - 39 = -1 through its frame but to round(-1.5) = -2 on the page (a
fractional scale; 4-ii's on-page placement check caught it). Edges now
snap half up, `half_up(v)` = the nearest whole number, a half up, as
Chromium's `LayoutUnit::Round` rounds; `half_up(v) + n == half_up(v +
n)` for every whole `n`. Pixels change only for an edge at exactly a
negative half device pixel, which needs a fractional scale; the moves
`kept_shift.rs` refused across zero at such a tie are now shifted.

What this changes:

- Placement (PR 4a, the layout lane). `Placement` keeps each slot's
  rect in its frame's coordinates and, per scroll container, the
  rounded offset it applies. A scroll marks the container dirty but
  moves no content origin: `visit_placed_in` is not entered for its
  descendants, `boxes_placed` is 1 and `moved` is empty, so no part is
  painted or shifted. `Painted::border_box` and `padding_origin`
  (`painted.rs:161-177`), the inline rects and the text origins add
  the frame chain's translation on read; the chain is O(nesting depth)
  and the per-frame translations are kept in `Painted`.
- Hits. `Painted::hit` (`painted.rs:192-200`) scans the gathered hits;
  each gathered hit carries its frame, and the query point is
  translated into that frame by a table lookup per hit: the same
  O(hits) scan, one subtraction more. 3c-3g's kept hit list keeps
  frames the same way.
- Clips. A part's clip (`BoxFrom::clip`, `kept_box.rs:25-33`;
  `kept_clips.rs`) is split: the clips of boxes inside the frame are
  in frame coordinates and move with the scroll; the frame's own
  padding box and its ancestors' clips do not. Each batch carries the
  inner clip (frame coordinates, `Option<Rect>`) and names its frame;
  the frame keeps its absolute clip (its padding box intersected with
  its parent frame's, kept per frame). The scissor at draw is the
  translated inner clip intersected with the frame's clip
  (`painter_region.rs:20-30` gains the translation).
- Batching. Consecutive items share a batch only within one frame: the
  frame joins kind and clip as a batching key (`batch.rs:91-102`). On
  the rows page every item is in the body's frame, so the batches are
  as today.
- Culling. An item is culled against the view translated into its
  frame, cut to the frame's clip; `culled` (`batch.rs:83-86`) takes
  the frame's view in place of the target's. Scrolling changes which
  steps show, so the kept batches of a scrolled frame are split again:
  section 4 makes that O(log n + shown).
- Nesting. An inner scroller's box is in the outer's frame; its
  translation is its parent frame's plus its own rounded offset's
  negative, computed per frame in one walk over the scroll containers
  (`KeptOrder::thumbs`, `kept_order.rs:25-27`, already lists them),
  O(scroll containers), never O(rows). `transform` is a known property
  (`crates/css/src/property/table_rest.rs:87`) that layout and paint
  do not read today; when they do, a transformed box is a frame whose
  uniform is an affine matrix in place of a translation, through the
  same bind and the same per-frame walk.
- The painter. The quad, shadow, text and image shaders take the
  frame's translation from the dynamic-offset uniform (section 2) and
  add it in device pixels before the clip-space conversion. The text
  and image vertices change from clip space (`quads.rs:135-140`,
  `quad.rs:51-58`) to device pixels, with the viewport in the uniform
  as the instanced shaders already have it (`quads.wgsl:33-41`).
  Immediates (push constants) are not in WebGPU; dynamic uniform
  offsets are, so the translation goes that way on both targets.
- Thumbs are painted every update that paints (`paint_partial.rs:47`)
  in the scroller's parent frame, as today.
- Debug checks: section 5.

An insert is not helped: the rows after it move within their frame,
and that is a true move. Its floor is layout: Taffy lays the list
container out again and writes every child's location
(`layout/written.rs:1-6`, `placement.rs:170-183`), O(rows after),
before placement reports them moved and 3c-3e shifts them. What IU4
does for an insert is bound the GPU side: only the shown batches among
the shifted rows are rebuilt and written (the others have no GPU
bytes), so uploads are O(visible), and section 4 keeps the kept
batches' patch O(log n) per shifted step. Splice PR C records the
insert's cost; if its layout floor is to move, that is a layout-lane
design (positions relative to the previous sibling), not this one.

## 4. Culling with kept GPU data

`KeptBatches::of` tests every item (`kept_batches.rs:56-58`, `113`)
and `hidden` is a `u64` per step (line 52); a scroll, a resize or an
order walk pays O(steps + items). IU4 keeps, per scroll frame, an
extents index over the drawing steps in that frame:

- `tops`: the steps sorted by the top of their items' bounds (the union
  of `Item::bounds`, `bounds.rs:29-37`, in frame coordinates), each
  with its place in paint order and its bottom; and `reach`: the
  running maximum of the bottoms along `tops`.
- The steps that can show in a view: those from the first index whose
  `reach` passes the view's top (binary search) to the last whose top
  is before the view's bottom (binary search). Every step entirely
  above the view is before the first; every step entirely below is
  after the last. The candidates in between are tested as today
  (`culled`) for their x extent and the few that reach past a tall
  earlier step, then sorted by place and batched. O(log n + candidates).
- Patching: a step painted again has its entry removed and inserted
  again (binary search and a `Vec` shift, O(n) memmove of 24-byte
  entries, measured against a B-tree in PR 5 if it shows); an order
  walk makes the index afresh, O(n log n). `hidden` and
  `hidden_culled` go: `Batched::culled` is the frame's item count less
  its shown items' count, with a per-frame item count kept and patched
  per step.
- The debug build compares the candidate set with a test of every item
  in the frame, naming the first step that one has and the other does
  not: a step with a wrong extent, or `reach` not maintained, fails on
  the draw that misses it.

Paint order inside a frame is not the same as top order (a positioned
box, a z-index), which is why the candidates are sorted by place, not
drawn in `tops` order. Horizontal scrolling uses the same index on the
other axis when a frame scrolls horizontally; the rows page scrolls
vertically.

## 5. Loud failure

- Kept bytes against a fresh build. Reading back the target is
  expensive and would compare pixels, not what was uploaded. A debug
  build keeps each batch's uploaded bytes (`Gpu::bytes`) and, on every
  draw, builds the bytes of every shown batch afresh as the
  non-retained path would, in absolute device pixels, and compares
  them with the kept bytes with the frame's translation applied,
  naming the first batch, instance and attribute that differ, the
  frame and the translation. One check pins the retention, the frame
  coordinates and the translations together. The fresh build is
  O(shown items), what the release path was before this plan.
- Counters, in `FrameCounts` (`painter_counts.rs:25-48`), pinned in
  the harness `Counts` line and in `gpu_*` tests: `batches_written`
  (bytes built and uploaded this frame), `batches_kept`,
  `retained_bytes` (live), `retained_grown`, `retained_compacted`,
  `bind_groups_made`, `uniform_writes`. Idle pins 0 writes, 0 bind
  groups; Text pins `batches_written` at the batches holding the
  edited value (1); Scroll pins 0 parts painted and writes only for
  the batches at the view's edges. `buffer_bytes` falls to those
  writes; `gpu_frame_counts_kinds.rs:58` and
  `gpu_frame_counts_surface_work.rs:67`, `86` pin today's per-frame
  bytes and are re-pinned in the PR that changes them, each proven by
  breaking.
- Refusals. A batch whose bytes cannot fit a `Retained` buffer under
  `max_buffer_size` is refused as `FrameArena` refuses one
  (`frame_arena.rs:168-181`, `ArenaFull`), naming the pipeline, the
  bytes and the limit; the painter ends the frame as it does for
  `take_refusal` (`painter.rs:126-129`). A compaction that still does
  not fit refuses the same way. The encoder budget
  (`encoder_budget.rs:17-33`) is unchanged: draws per frame are the
  shown batches, fewer than before.
- Atlas. An eviction that would take a pinned glyph panics naming the
  glyph and its pin count; a page clear with pins outstanding bumps
  the generation and records it in `FrameCounts::atlas_generation`,
  and the debug build asserts after the next draw that no shown text
  batch is behind it. The existing `AtlasError::Full`
  (`atlas_evict.rs:25-39`) stays for a frame whose own glyphs do not
  fit.
- Frames and clips. A step whose frame is not a scroll container, or
  a batch whose frame has no translation this frame, panics naming the
  step and slot, as `scene_view.rs:104-112` does for a vacant slot.
- Kept batches. `kept_batches_check::assert_same`
  (`kept_batches_check.rs:8-17`) stays and now compares frames and
  inner clips too; the extents check is in section 4.
- Every check is proven by breaking what it guards in its PR, and the
  commit says so.

## 6. Web and native; what IU4 must not break

- Dynamic uniform offsets: 256-byte alignment on WebGPU's defaults and
  on ANV and lavapipe (`device.limits()`, read at pipeline creation as
  `FrameArena::uniforms` reads it, `frame_arena.rs:84-87`); the
  uniform binding size per entry is far under the 64 KiB
  `max_uniform_buffer_binding_size`. `max_buffer_size` on WebGPU is
  256 MiB by default; the retained buffers are well under it, and the
  refusal names the limit when they are not.
- `Queue::write_buffer` on the web crosses into JS per call; fewer,
  larger writes help there more than natively. The retained path makes
  an idle frame zero writes on both.
- Timestamp queries: native only in the harness; the web is checked by
  the Chromium screenshot comparisons and `check_web`, unchanged.
- wasm is single-threaded: `Retained` and the pins are plain
  structures, no `Arc`.
- Pixels: every Chromium layout and paint comparison and every
  `gpu_*` test passes unchanged by each PR; text quality is the same
  shader with the same quads at the same device pixels (section 3's
  rounding argument; a Scroll screenshot comparison at a fractional
  scroller origin is added with PR 4b).
- Surfaces and 3D: a `gpu-surface` renders in its own encoder
  (`gpu-surface.md`, "Display list") and is composed as an image;
  only the composition quad and bind group are kept. The racer's
  workloads must be within noise.
- touche: it draws through `vapors_canvas` (`LabelDraw`,
  `crates/canvas/src/label.rs:52-64`, `96-110`: `SolidPipeline::fill`
  and `TextPipeline::draw` on their own `FrameEncoder`) and
  `ImagePipeline::draw` (`image_view_draw.rs:37`); it does not use
  `ScenePainter`. Those three entry points keep their signatures and
  the immediate path (`FrameArena`) behind them; the retained path is
  additional API on the pipelines, so a pinned touche builds unchanged.
- The tooltip (`painter_tooltip.rs:31-37`) draws through the same
  pipelines on the immediate path; unchanged.
- Culling counts (`FrameCounts::culled`, pinned in `gpu_cull.rs`) are
  equal before and after section 4's index, pinned.

## 7. PR stack

Each under about 400 production lines; acceptance in instruction
counts (`vapors-perf 60 10 --instructions`) with wall clock and GPU
time recorded in `docs/perf/`, three runs, medians, at 1000, 3000 and
10,000 rows. "Draw" is the `draw` frame phase.

| PR | Content | Acceptance | Parallel with |
|---|---|---|---|
| 0 | `draw step` spans `batches`, `build`, `bind`; the harness prints them; the `Resize` workload; 10,000 rows if not yet added; the table of section 1 measured | A section in `docs/perf/` with draw's split per workload | everything |
| 1 | Fixed per-draw costs: viewport uniform and bind group kept per pipeline and target, bound with a dynamic offset; text params written once per frame; the text texture view kept; image and surface bind groups kept per texture; `bind_groups_made`, `uniform_writes` counted | Idle: `bind_groups_made` 0, `uniform_writes` 0; draw instructions at 3000 rows recorded before and after (expected: most of `bind`'s share gone) | paint lane (3c-3e, 3c-3g), splice A and B, layout and style lanes: `crates/renderer`, `crates/text`, `painter*.rs` only |
| 2 | `Retained` buffers with free ranges and compaction; `Kept` per batch in `KeptBatches`; batches matched by equal `Run` when made again; quads, shadows, images and surfaces on the retained path; the debug compare of kept bytes; `batches_written`, `batches_kept`, `retained_*`; the refusal | Idle: `buffer_bytes` 0; Text: `batches_written` 1; Class: the row's batches; draw instructions at 3000 rows recorded (expected: `write` and `build` for those kinds gone) | as PR 1, plus `kept_batches.rs` (3c-3f merged; splice B does not touch it) |
| 3 | Text and icon bytes retained: device-pixel vertices with the viewport in the shader; atlas pins and generation; repack rebuilds; the pin panic | Idle: 0 atlas lookups per frame (counted); a test that evicts under pressure with pinned glyphs repacks and rebuilds every text batch, proven by breaking; Chromium text comparisons unchanged | as PR 1 |
| 4a | Placement in scroll-frame coordinates: `Placement::rects` per frame, per-container rounded offsets, `border_box` and the other readers translating on read; a scroll places one slot and moves none; hits carry their frame | Scroll at 3000 rows: `boxes_placed` 1, `boxes_moved` 0, `boxes_painted` 0, pinned; the placement debug check (`iu3.md` "3b") compares absolute rects | after 3c-3e; layout lane owns `placement*.rs` |
| 4b | Paint in scroll-frame coordinates: parts painted at offset zero; clips split into inner and frame; the frame as a batching key; culling in the frame's view; `Painted` keeps the frames' translations and clips, computed per update over the scroll containers | Scroll: `scene_items_built` 0, kept batches patched only at the view's edges; the kept-paint debug check compares with the translation applied; a fractional-origin scroller screenshot against Chromium | after 4a and 3c-3e; paint lane owns `paint*.rs`, `kept_box.rs`, `kept_clips.rs` |
| 4c | The painter: translations in the dynamic-offset uniform; scissors from inner clip and frame clip; the shaders add the translation | Scroll at 3000 and 10,000 rows: draw instructions equal to Idle's within noise; `batches_written` only at the view's edges | with 4b, same author |
| 5 | The per-frame extents index; `hidden` replaced by per-frame item counts; the candidate debug check | Resize and Scroll at 10,000 rows: `batches` instructions equal to 1000 rows' within noise; `culled` unchanged | after 4b; `kept_batches.rs`, `drawn_steps.rs` |
| 6 | Acceptance: every rows workload at 1000, 3000 and 10,000 rows, Scroll, Insert and Resize, instruction counts, wall clock and GPU time in `docs/perf/`; property 2 and the scroll remainder of property 4 marked held or not in `perf-finish-line.md` | Draw under 1 ms and the same within noise at every row count; Scroll's update within Idle's | last |

PR 2 as built (2026-11-12), split along the file seam into 2a
(`crates/renderer`: `Retained`, the pipelines' kept ranges) and 2b
(the painter and the keys):

- `Kept` is split across crates, because `vapors-elements` does not
  depend on the renderer. `KeptBatches` keeps a `BatchKey` per batch
  (`kept_batches_keys.rs`), and the painter keeps each key's range,
  viewport and debug bytes (`painter_kept_gpu.rs`).
- Matching by place alone gave every batch after an inserted one a new
  key, for example when a class toggle adds a background quad that
  splits a text batch. So batches are matched by place from the start
  up to the first difference, and by place from the end after it.
- A released range is reused only once a newer encoder writes, because
  a draw recorded earlier in the same frame may still read it.
- Text batches keep keys by `TextId` (risk 8). Their bytes are not
  kept until PR 3, which must compare the resolved layouts.

PR 3 as built (2026-11-13), split along the crate seam into 3a
(`crates/text`: device-pixel vertices, kept text bytes, atlas pins) and
3b (the painter):

- The viewport goes in the text params uniform beside gamma and
  contrast, so a resize writes the text uniform once, as it does the
  quad and shadow uniforms.
- Pins are a count per atlas entry. `AtlasPins` lists the entries one
  batch pinned, once per use. A pin clear voids every `AtlasPins` taken
  before it, by an epoch, so a batch released afterwards unpins
  nothing. Empty pins made before a clear take the new epoch, because
  a build's first pass can clear the pins before its second pass pins.
- The order when room is needed: evict unpinned entries the frame has
  not used; then grow the page; then clear every pin and evict again.
  A page whose shown text does not fit even after the clear still
  fails the frame with `AtlasError::Full`, as it did before.
- Risk 8: a text or icon batch is current only while each item
  resolves to the same `Rc` it was built from. `Painted::text` and
  `Painted::icon` hand back the `Rc`, and the kept batch holds clones,
  so no other layout can take the old one's address.
- The debug compare rebuilds a kept text batch's bytes from where the
  atlas holds its glyphs now (`placed_text_bytes`). It does no lookup,
  so the lookup count is the release path's.

### 4a to 4c land together

4a cannot land alone: once placement moves no slot on a scroll, paint
is told of no moved part, and until paint is in frame coordinates
nothing shifts the scrolled rows, so the page would not scroll on
screen (today a scroll keeps `boxes_painted` at 0 by shifting every
row, `tests/kept_shift.rs`). So 4a, 4b and 4c are one stack, each PR
reviewed on its own and kept green with pixels unchanged at its head,
merged together as a train (lead, 2026-11-12). The stack splits along
what each PR can hold alone:

- 4-i: parts, map entries and hits name their scroll frame
  (`frames.rs`; `BoxFrom::frame`, `Painted::node_frames`), and every
  reader puts them on the page as it reads (the scene's items and
  batches, hits, `border_box`, `padding_origin`, text origins, range
  tracks). Each frame's clip is its scroll container's content clip on
  the page. Every shift is zero at this PR's head, since placement is
  still on the page.
- 4-ii: placement and paint in frame coordinates: a scroll settles the
  container's offset and moves no content origin; each frame's shift
  is the container's rounded offset added to its outer frame's; clips
  split at a frame (a box inside a scroll container starts with no
  clip in its frame, the frame's clip is applied on read); the kept
  batches made again, and the hover tested again, when a frame's
  shift or clip changes. Acceptance is 4a's and 4b's.
- 4-iii: the translation moves from the CPU (where 4-i applies it as
  batches are made) into the dynamic-offset uniform, so kept GPU bytes
  survive a scroll. It pays only once PR 2 keeps GPU bytes, so it is
  its own PR after IU4 PR 2 merges (lead, 2026-11-12).

The a11y mirror reads border boxes from moved slots: with 4-ii a
scroll moves none, so 4-ii is built on R6 PR 5a (#243), whose mirror
bounds are relative to the scroll container, and lands after it; no
interim touch of the scrolled rows (lead, 2026-11-12). #243's
`scrolled` set (rows moved by a scroll alone) is always empty once a
scroll moves no row, so 4-ii removes it; placement reports the
containers whose offset moved instead (`ChangedSlots::offsets_moved`).

Recommended order: 0, 1, 2, 3, 4a, 4b, 4c, 5, 6. PRs 0 to 3 touch no
file the paint, splice, layout or style lanes change and can run as
one lane beside them. 4a waits for 3c-3e (which decides what a moved
part does) and belongs to whoever owns placement; 4b and 4c follow it.
5 is independent of 2 and 3 and can be pulled forward if PR 0's
`batches` share at 10,000 rows is large.

## 8. Open points and risks

1. Whether retention is worth its lines when PR 1 alone may bring
   draw well under 1 ms. Options: (a) stop after PR 1 if its draw is
   under 0.5 ms at 10,000 rows; (b) build 2 and 3 regardless. PR 1
   leaves about 104 `write_buffer` calls and 600 glyph lookups per
   frame, each a JS crossing or an allocation on the web, and the
   count grows with the visible rows (a 4K window shows 90 rows, not
   17), so (a) would hold property 2 on the harness and not in a
   window. Recommendation: (b), with PR 0's split deciding whether 2
   or 3 goes first.
2. Frame origin: the scroller's unscrolled content (this design,
   pixel-identical to today) or the scroller's own box (a scroller
   that moves on the page costs O(1), but boxes inside a scroller at a
   fractional device position snap relative to its edge and can
   differ from Chromium by a pixel). Recommendation: the unscrolled
   content; a moving scroller is a moved box like any other, shifted
   by 3c-3e, and no workload shows it.
3. Atlas eviction with kept text bytes: (a) touch every kept glyph per
   frame (O(glyphs), what retention removes); (b) pin counts, evicting
   around pins and repacking with a generation bump when pins block
   room (this design); (c) never evict, refuse a full page. (c) fails
   long sessions with many fonts and sizes; (a) keeps the cost.
   Recommendation: (b); the pressure test in PR 3 proves the repack
   path.
4. The translation's transport: dynamic uniform offsets (this design)
   or a per-instance frame index into a storage buffer of
   translations, which lets a batch span frames. Batches cannot span
   frames anyway, since the frame's clip differs, so the index buys
   nothing. Recommendation: dynamic offsets.
5. Inserts stay O(rows after) in layout, placement and the shift
   (section 3); IU4 bounds only the GPU side. If splice PR C's
   measurement puts an insert at 3000 rows over an edit's budget, the
   layout lane designs sibling-relative positions; not this plan.
6. The extents index assumes items' bounds in frame coordinates are
   fixed between patches. A text run's `ink` and a shadow's `drawn`
   are part of the item, so they are. A part painted again gets its
   entry patched; a part shifted by 3c-3e must patch its entry too, or
   the index points past the view. The debug check in section 4
   catches a missed patch on the draw that misses it; the 3c-3e shift
   in PR 4b calls the same patch as a repaint.
7. The debug build's fresh-build compare makes every debug draw
   O(shown items) plus the full cull in `assert_same` (O(items)), as
   today; the 1000-row oracle test's debug time is recorded before and
   after PR 2 and PR 5, and a rise past 10% is a finding for the lead.
8. `Run: PartialEq` compares `Item`s by value, including `TextId`s
   that the slabs (`painted.rs:22-24`) may reissue to a different
   layout after a repaint: a batch could match by id with other text.
   The match must compare the resolved layout (`Rc::ptr_eq`, as
   `kept_paint_check` does) for text, image and icon items, not the
   id; PR 2 adds a test where a repainted box's freed id is reissued
   to another text and the batch is rebuilt, proven by breaking.
9. Scroller-local coordinates change what `Painted::border_box`
   returns per call (a translation on read) and make `rects` in
   `PaintedFrom::placement` frame-relative; every reader of `rects`
   (`kept_maps.rs`, `paint_inline.rs`, `text_paint.rs:78`, hit
   gathering, the accessibility and tooltip code in `crates/app`) is
   listed in PR 4a's description with how it reads the translation.
   A reader missed is caught by the placement debug check, which
   compares absolute rects for every slot.
