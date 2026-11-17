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
is F2a, F2b, F2c; Tom checks the Pixel 9 Pro at F2a's end, from a build
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
   - **F2a:** the table, the refusal, the page-contract case, and the
     quad and shadow pipelines on it.
   - **F2b:** the image and text pipelines (icons included, which the
     text pipeline draws).
   - **F2c:** the closed-form and mesh pipelines. This retires the shape
     batch cut, `Shift` and the uniform row table.
4. **The Pixel 9 Pro check.** It is by hand. I can serve the build on
   the tailnet (`cargo xtask serve`), and someone with the phone has to
   open it and report. Who does it, and at which slice: F2a, the first
   to read the table?
