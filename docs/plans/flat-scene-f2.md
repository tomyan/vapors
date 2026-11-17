# Flat scene F2: the transform table

A design note for F2 (`flat-scene.md` section 8, the F2 row; the table
is section 2.2), written against main at f224aa99. It says what the
painter does today, what F2 changes, where this note departs from
section 2.2 and why, and the questions for the lead before building.

## Today

- **Per batch, one uniform slot.** Each 2D pipeline binds one uniform
  slot per batch, at a dynamic offset (`renderer/src/uniform_slots.rs`).
  The slot holds the target's size, the batch's frame shift
  (`renderer/src/shift.rs`, whole pixels) and a table of up to 128 row
  offsets (`renderer/src/row_offsets.rs`, 1 KiB).
  - The quad, shadow, image and text pipelines read all three.
  - The closed-form and mesh pipelines (shapes) read the size and the
    shift only.
- **Per instance, a row index.** Each instance carries a `row: u32`
  into its batch's row table (`instanced.rs` `instance_bytes`, padded to
  16 bytes). A batch is in one frame, or in the rows of one framed lane
  (`BatchFrame`, `Framing::Lane`).
- **Shapes cut lane batches.** A row that paints a shape is batched in
  its row's own frame, because the shape pipelines take no row table
  (`framed_batches.rs` `resolved`; splice-layout.md section 12, "Lane
  batches cut at rows that paint shapes", which names F2 as the fix).
- **Per batch, the painter keeps the row table.** It is kept by batch
  key and made again when a row's offset changes
  (`app/src/painter_rows.rs` `RowTables`). A scroll writes one new
  uniform value for the scrolled frame (4c); Idle writes none.
- **The kept bytes outlive a frame.** A batch's instance bytes are
  kept in a `Retained` buffer and drawn again in later frames, so
  anything they name must still mean the same thing then
  (`painter_kept*.rs`; the debug compare in `painter_kept_check.rs`).

## The constraint that shapes the design

Section 2.2 gives every instance one index into one table that holds
an entry per node: the page, each scroll frame and each lane row in
view. Two facts make a lane row's entry unworkable as a single global
index:

- An entry per lane row that lives as long as the row would hold the
  row's offset from its lane. An insert moves every later row, so it
  would rewrite O(rows after) entries, which is what lane frames exist
  to avoid (property 3).
- An entry per lane row that lives only while the row is in view would
  give the row a different index when it scrolls back. Kept instance
  bytes would then name a stale entry, so they would have to be
  rewritten.

So this note keeps section 2.2's table but uses two indices per
instance, where section 2.2 has one.

## What F2 changes

1. **The table.** Each 2D pipeline group shares one storage buffer of
   `mat4x4<f32>` entries (64 bytes each). Its free ranges follow
   `Retained` (`retained.rs`). It has two kinds of entry:
   - **Frame entries:** one per frame node (the page, each scroll
     frame, each framed lane's template), keyed by the frame's slot. An
     entry keeps its index for as long as the frame exists, and is
     written when the frame's shift changes. Index 0 is the page's
     identity.
   - **Row ranges:** one per kept lane batch, holding each of its rows'
     offset from the lane's template. A range is allocated when the
     batch is first drawn, rewritten when a row's offset changes (as
     `RowTables` does today), and freed with the batch.
2. **Two indices per instance.** `frame: u32` (global) and `row: u32`
   (local to the batch's row range) replace `row: u32`. What that adds
   to each layout:
   - quad, shadow and closed-form instances: nothing, as they are
     padded to 16 bytes after their floats (`instance_bytes`);
   - text vertices: 4 bytes each (`VERTEX_SIZE`, 36 to 40);
   - mesh instances: 8 bytes each, as they carry no index today;
   - images: nothing, as they are drawn one item a draw and the indices
     go in the draw's uniform.
3. **The shaders read the table.** The vertex shader computes
   `transforms[frame] * transforms[rows_base + row]` applied to the
   local position, then the viewport. `rows_base` is the batch's range,
   bound as a dynamic offset into the same storage buffer (256-byte
   alignment, so a range starts on a multiple of four entries). A batch
   in one frame has the empty range at row 0.
4. **What retires:**
   - `Shift` and the row table in the uniform slot, for the painter;
   - the uniform slot then holds only the target's size, one value per
     target;
   - the shape batch cut: shapes read the same entries, so a row that
     paints a shape stays in its lane's batch.
5. **Touche's immediate path is untouched.** It draws at frame 0 with
   the empty range, so the pipelines' immediate entry points keep their
   signatures and bytes.
6. **Writes.**
   - A scroll writes one 64-byte frame entry, where 4c wrote one
     uniform value.
   - An insert rewrites the row ranges of the batches whose rows moved,
     as today.
   - Idle writes nothing.
   - The counts are `transform_writes` and `transform_bytes` in
     `FrameCounts`.
7. **Refusal.**
   - `vapors_gpu::raised_limits` adds
     `max_storage_buffers_per_shader_stage` (at least one, read in the
     vertex stage).
   - The device must have the `VERTEX_STORAGE` downlevel flag.
   - A device without either is refused naming the limit, as the other
     limits are, with a unit test on faked limits.
8. **The page-contract case.** A case in
   `scripts/check-page-contract.sh` renders a frame that reads the
   table under headless Chromium on SwiftShader.

The class word of section 2.2 is left out of the entry: in F2 nothing on
the GPU reads it, and the class stays on `FrameAt` (F1). F4a adds it
with the shader code that branches on it.

## Acceptance (the F2 row, with this design)

- Pixels unchanged: every Chromium comparison and `gpu_*` test.
- Scroll at 3,000 rows: transform writes equal to 4c's uniform writes,
  one per scrolled frame. Idle: 0.
- The `kept_bytes` debug compare covers both indices.
- A row that paints a shape no longer cuts its lane's batch: a new pin
  of the batch count on a rows page with a shape in every row.
- The device refusal test on faked limits.
- The page-contract case on SwiftShader.
- The Pixel 9 Pro checked by hand, and recorded in `docs/web.md`.

Each check is proven by a break.

## Questions for the lead

Answered by the lead (2026-11-17): each recommendation below is taken.
Two indices; the closed-form and mesh pipelines are included; the split
is F2.1, F2.2, F2.3 (renamed from F2a, F2b, F2c, which clashed with
`docs/F2c.md`, the CSS plan); Tom checks the Pixel 9 Pro at F2.1's end, from a build
served on the tailnet.

Decision (Tom, 2026-11-17, through the lead): a device without
vertex-stage storage buffers is refused, naming the limit. There is no
fallback path. What would retire this: a measurement of how many of
the devices vapors targets lack them. If it is enough to matter, a
fallback is designed then (for example, today's per-batch uniform
kept as a second path).

1. **Two indices, not one.** The instance carries `frame` (global,
   stable) and `row` (local to the batch's range, bound at a dynamic
   offset), for the reason in "The constraint". The cost is one more
   table read per vertex, plus the bytes in item 2 of "What F2 changes"
   (nothing for quads and shadows). The alternative, one global index
   with an entry per lane row, either rewrites O(rows after) entries on
   an insert or rewrites kept bytes when a row scrolls back. Recommended:
   two indices.
2. **Shapes in F2.** The F2 row names the quad, shadow, text and image
   layouts. Section 12 of splice-layout.md expects F2 to retire the
   shape batch cut, which needs the closed-form and mesh pipelines on
   the table too. Recommended: include them.
3. **The split.** Six pipelines, six shaders, the text crate and the
   app's painter come to more than one pull request of about 400 lines.
   Recommended, each keeping pixels unchanged:
   - **F2.1:** the table, the refusal, the page-contract case, and the
     quad and shadow pipelines on it.
   - **F2.2:** the image and text pipelines (icons included, which the
     text pipeline draws).
   - **F2.3:** the closed-form and mesh pipelines. This retires the shape
     batch cut, `Shift` and the uniform row table.
4. **The Pixel 9 Pro check.** It is by hand. I can serve the build on
   the tailnet (`cargo xtask serve`), and someone with the phone has to
   open it and report. Who does it, and at which slice: F2.1, the first
   to read the table?

## As built: F2.1

- **Scope.** F2.1 moved the closed-form pipeline onto the table along
  with quads and shadows, because all three share `Instanced`
  (`renderer/src/instanced.rs`), and one pipeline cannot take both the
  old and the new path. F2.3 keeps the mesh pipeline, the shape batch
  cut, and the retirement of `Shift` and `RowOffsets`, which the image
  and text pipelines read until F2.2.
- **Renderer.**
  - `TransformTable` (`transform_table.rs`) holds the entries on the
    CPU (`transform_entries.rs`): entry 0 is the identity, values are
    written only when they change, and an entry changed twice for one
    encoder is refused.
  - The table owns its work counter, so its writes are counted once,
    by `take_written`.
  - The instanced pipelines bind the view uniform (size and `rows_base`,
    16 bytes) and the table. Their immediate draws use an identity table
    of their own.
- **App.** `painter_transforms.rs` keeps a frame entry per
  `BatchFrame` and a row range per lane batch's key, each given back
  after a frame that does not draw it, as kept bytes are. `KeptGpu`
  builds a batch's bytes again when its frame's entry changes.
  `FrameCounts::transform_writes` and `transform_bytes` count the
  writes.
- **Refusal.** `vapors_gpu::raised_limits` takes the adapter's
  downlevel flags. It refuses `max_storage_buffers_per_shader_stage`
  below the default, and a device without `VERTEX_STORAGE`, naming
  each.
- **Page contract.** Unchanged. Every demo's screenshot pixel checks
  draw quads and shadows, which now read the table, under headless
  Chromium on SwiftShader. All thirteen cases hold.

### After the review of F2.1 (issuecomment-6937532551)

- **Growth is pinned.** A GPU test takes 70 entries in a second frame
  (past the first buffer's 64) and draws a quad kept in the last of
  them; it lands where the same quad drawn directly does. Removing the
  generation bump, or the rewrite of a fresh buffer, fails it.
- **A row must be in its own batch's range.** An item's row is
  checked against its batch's range when its bytes are built, and in
  debug builds on every kept draw (`painter_kept_bytes.rs`
  `rows_in_range`). A bad row is refused, naming the batch and the row.
  Before, a row past the range read the table's next entries: another
  batch's rows, or a frame's shift.
- **The growth refusal** names the limit that applied
  (`max_storage_buffer_binding_size` or `max_buffer_size`), with a fix
  for entries held while drawn (`Holding::TransformEntries`).
- **One table a pipeline.** Each table has its own id. A pipeline
  refuses a kept draw from a second table, because it keeps one bind
  group for its table, keyed by the generation alone.
- **The identity table's first write** (4 KiB) is counted in the
  pipeline's work.
- **Lane writes are pinned.** An unchanged frame of a framed lane writes
  nothing, and a row put in at the top writes the eight rows in view
  (512 bytes), not the rows below it.
- **Precision (known, not a defect).** The shader now adds a row's
  offset to its frame's shift before adding either to the position:
  `pixel + (offset + shift)`, where the old path did
  `(pixel + shift) + offset`. Both shift and offset are whole, so
  `offset + shift` is exact below 2^24, and the new order keeps a
  fractional position exact where the old order rounded it at the
  shift's magnitude. At a shift near 2 million device pixels (row
  100,000 of 20 px rows) an f32's step is 0.125 px. Pixels are the same
  in every tested case. Beyond the tested range, lane rows of shadows
  and shapes can differ from before, more precisely, and text and
  images keep the old order until F2.2, so a row's glyphs and its box
  can round differently there. No test pins a large offset.
- **Known term, retired in F2.2.** Row ranges were keyed by each
  batch's own key, where the old row tables were keyed by a part's
  first batch (`painter_places.rs`), so a part split into several
  batches held one range per batch and an insert wrote each. F2.2
  retired the old row tables, and the ranges are keyed by the part's
  first batch, one per part.
- **Scroll writes.** A scroll writes one frame entry whatever the
  list's length, pinned on a ten-row list (`gpu_kept_scroll.rs`). The
  plan's 3,000-row figure has no test of its own.

## As built: F2.2

- **Images and surfaces** are drawn one item at a time, so their frame's
  entry and their row's entry go in the draw's uniform: the size, the
  frame entry and the row entry, 16 bytes. Their vertex bytes are
  unchanged. The view group binds the table at binding 1, with the
  instanced pipelines' layout.
- **Text and icons.**
  - Each vertex carries its row's index and its frame's entry (36 to 40
    bytes). That is 24 bytes more per glyph (six vertices), about 11%
    more kept text memory and text upload on a rebuild; a 13-glyph run
    keeps 3120 bytes where it kept 2816 (`gpu_frame_counts_kinds.rs`). Every vertex of an F2
    batch has the same frame, so the frame could go in the params
    uniform, which has 12 spare bytes. It stays per vertex because F3
    joins batches across frames (`flat-scene.md`, the F3 row), and a
    joined batch's glyphs are then in different frames.
  - The params uniform is gamma, contrast, the viewport and the draw's
    first row entry (32 bytes, where it was 32 plus a 1 KiB row table).
  - The table is at binding 3.
  - Kept text is built again when its frame's entry changes, like the
    other kept bytes, and its rows are held to their range
    (`rows_in_range`).
- **One table a pipeline.** The image and text pipelines refuse a
  second table, as the instanced ones do. Each has its own identity
  table for immediate draws, and counts that table's first write.
- **Retired in the app.** Nothing reads the uniform row tables any
  more. `RowTables`, the places' table of `RowOffsets`, `Moved::table`
  and `Moved::item_shift` are gone. `Moved::item_row` gives an image or
  surface item's row, keeping the refusal for an item with no row.
- **Left for F2.3:** the mesh pipeline (still on `Shift`), the shape
  batch cut, and the renderer's `Shift` and `RowOffsets` types
  (`RowOffsets::CAPACITY` still caps a batch's range at 128 rows).
- **Kept bytes and a range that changes.** A batch keeps its key, and
  so its kept bytes, while its rows' offsets change: vapors-elements
  matches keys on the items' rows, not on the rows' offsets
  (`kept_batches_keys.rs` `same`). The range a batch's bytes are drawn
  with can therefore shrink after the bytes were built, which the
  build-time check (`rows_in_range`) does not see. Each kept batch
  records the rows its bytes name (the highest row and one), and every
  draw from kept bytes, in every build, refuses a range that holds
  fewer (`rows_fit`, O(1)).

### After the review of F2.2 (issuecomment-6939479919)

- **Kept text in rows is tested on the kept path.** The lane text test
  inserted a row, which gives every visible batch new item rows and so
  rebuilds them. It now grows the first row instead: the rows below
  move, their batches keep their bytes, and the test asserts the text
  was drawn with nothing written. A second test compares the first two
  rows with a page of those two alone (no lane), because a break in the
  shared draw also breaks the fresh painter the first test compares
  with.
- **Both `rows_fit` call sites have a test.** GPU tests call
  `KeptGpu::range` and `text_range` with a range that fits, then with
  one row fewer, and expect the refusal.
- **`TextGpu::new` takes `rows_needed`**, as `Gpu` is built with it.
- **Bind groups are keyed by both generations** (`Generations`, a
  pair), not two counters packed into a `u64`.
- **One range per lane part is tested:** rows with a shadow and a
  background (two batches a part) rewrite one range's rows after an
  insert.

