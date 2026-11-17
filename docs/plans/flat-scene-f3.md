# Flat scene F3: clips as data

A design note for F3 (`flat-scene.md` section 8, the F3 row; the clip
table is section 2.3), written against main and against F2.1 as built
in #459 (`flat-scene-f2.md`, "As built: F2.1"; the conventions lane3
listed on #459, issuecomment-6937542558), which the clip table follows:
a storage buffer beside the transform table, written only when an entry
changes and refusing an entry changed twice for one encoder, growing by
doubling under the buffer limits and refusing beyond them
(`ArenaFull`), counting its writes through its own `WorkCounter`, its
entries given back after a frame that does not draw them as kept bytes
are, and a device that lacks what it needs refused naming the limit. It says what
clips are today, what F3 changes, and the questions for the lead before
building.

## Today

- **Elements.** A box's clip is one rectangle in CSS px: the
  intersection of the content boxes of its clipping ancestors
  (`clip.rs`, kept per slot by `kept_clips.rs`, parents from
  `clip_tree.rs`). Paint snaps it to device pixels and sets it on the
  scene before the box's items (`paint.rs` `clip_to`); hits are cut to
  the unsnapped rectangle.
- **Scene.** Every item carries an `Option<Rect>` clip in its frame's
  device pixels (`vapors-scene` `display.rs`). `batched` starts a new
  batch whenever the clip changes (`batch.rs` `push`).
- **Painter.** A batch's clip, moved by its frame's shift and cut to
  the frame's own clip (the scrollport), is the batch's scissor
  (`app/src/painter_runs.rs`, `painter_region.rs` `clipped`). A batch
  whose scissor is empty is not drawn.
- **Rounded clips are refused.** `style/overflow.rs` (a clipping box
  with a `border-radius`), `img_checks.rs` and `svg_style_checks.rs` (a
  radius on an image or an icon, which would clip its own content),
  and a rounded clip on a `gpu-surface` ancestor (`docs/gpu-surface.md`,
  "Rounded clip on an ancestor").
- **What a clip costs.** Every clip change splits a batch, and so did
  every frame until F2: on a page of many small scrollers (section 6,
  W4), each scroller's rows are batched apart from the next one's
  (section 2.2's reason for clips as data).

## The constraints

- **Kept bytes outlive a frame.** As with transform entries, an index
  in a kept instance must mean the same clip in later frames, so a
  clip's entry keeps its index for as long as the box that clips
  exists.
- **A clip is in a frame's space, not always the instance's.** A scroll
  container's content is in the scroll frame; the container's own clip
  (its padding box) is in the frame outside it. So a clip entry names
  the transform entry its rectangle is in, rather than borrowing the
  instance's.
- **Lane rows.** A box that clips inside a lane row has its rectangle
  in the row's space, and a row's offset is a batch-local entry (the
  row range, `flat-scene-f2.md` "What F2 changes" item 1). Question 1.
- **The scissor's job stays.** Under a whole translation, rectangle
  clips sit on whole device pixels, and fragments outside every clip
  should still be skipped by the scissor, not shaded and discarded.

## What F3 changes

1. **The clip table.** `ClipTable` in `vapors-renderer`, built as
   `TransformTable` is (`transform_table.rs`, `transform_entries.rs`):
   one storage buffer of clip entries at binding 2, beside the view
   uniform (0) and the transform table (1), with the same growth,
   write and refusal rules. An entry is a rectangle (`vec4<f32>`, device
   pixels in its frame's space, snapped under a whole translation as
   today's clip is), four corner radii (`vec4<f32>`, circular, as
   `Quad::radii` are), its frame's transform entry (`u32`), and its
   parent clip (`u32`, the next clip outward that it does not fold
   into, 0 for none): 40 bytes, padded to 48 (question 3). Index 0 is
   no clip.
2. **Entries.** One per box that clips: `overflow` other than
   `visible`, and a replaced element with a radius clipping its own
   content (an image, an icon, a `gpu-surface`). The painter keeps an
   entry per clipping box's slot, as `painter_transforms.rs` keeps one
   per `BatchFrame`, and gives it back after a frame that draws nothing
   under it, the rule `KeptGpu` uses for kept bytes; `KeptGpu` builds a
   batch's bytes again when an index it names changes. An entry is
   written when the clip's rectangle, radii or frame change (a layout
   or style change). A
   scroll does not write it: the rectangle is in its frame's space,
   which the frame's transform entry moves. A rounded clip's radii are
   its padding box's: the border radius less the border width, as CSS
   Backgrounds and Borders defines the padding edge's curve.
3. **Rectangles fold.** A chain of rectangular clips in one frame
   folds into one rectangle on the CPU, as today's intersection does;
   an entry's parent names the next clip that cannot fold into it (a
   rounded one, or one in another frame). The chain a fragment walks
   therefore has one link per rounded clip or frame boundary, not per
   clipping ancestor. A chain longer than `MAX_CLIP_CHAIN` is refused,
   naming the element and the chain; the constant is declared with the
   longest chain measured on the corpus and the demos as its reason
   (question 4).
4. **One more index per instance.** `clip: u32` beside `frame` and
   `row`. What it adds to each layout:
   - quad, shadow and closed-form instances: nothing, as `row` and
     `frame` use 8 of the 16 bytes of padding after their floats
     (`instanced_bytes.rs` `instance_bytes`);
   - text vertices: 4 bytes each, after F2.2's index;
   - mesh instances: 4 bytes each, after F2.3's two indices;
   - images: nothing, as the index goes in the draw's uniform.
5. **`clip.wgsl`.** Concatenated ahead of each 2D pipeline's shader, as
   `transform_table.wgsl` is, and shared with `svg-support.md` V10. One
   function: the coverage of a device-space position under
   clip `index`, the product of each link's coverage. Each link moves
   the position into its frame's space by that frame's transform entry
   (a translation by whole pixels for every frame before F4a; a debug
   assertion holds that, naming F4a), and takes the rounded
   rectangle's coverage from its signed distance with `fwidth`, the
   same function `quads.wgsl` uses for a rounded box's edge. A
   rectangle on whole pixels gives coverage 0 or 1 at pixel centres,
   the scissor's result, so rectangle clips draw the same pixels as
   today.
6. **The scissor.** A batch's scissor becomes the union of its
   instances' clip bounds on the target, worked out when the batch is
   made from the clip entries' device rectangles, and still cut to the
   frame's own clip (section 2.3).
7. **Batches join.** `batched` no longer starts a batch when the clip
   changes, and the batch key drops the clip. With F2's frame index,
   batches then join across frames and clips. `FrameCounts` gains
   `clip_writes` and `clip_bytes`, as F2 added `transform_writes` and
   `transform_bytes`.
8. **Refusals retired.** The four rounded-clip refusals above, each
   with its test converted into a test of the clip drawn.
9. **Hit testing.** Hits stay on the CPU, cut by the box's clip chain
   in CSS px; a rounded clip cuts them by its rounded shape, as
   Chromium does (to be confirmed by the events oracle, F3.2).
10. **The device.** The fragment stage now reads storage buffers (the
    clip table, and the transform table for each link's frame), so
    `vapors_gpu::raised_limits` adds that stage's limit to F2's vertex
    check, refusing it by name below what F3 reads, with the unit test
    on faked limits extended. There is no fallback, as for F2 (Tom's
    decision).

## Acceptance (the F3 row, with this design)

- Pixels unchanged on every Chromium comparison and `gpu_*` test while
  rectangle clips move from the scissor to the table (F3.1).
- A Chromium fixture of a rounded scroller holding text, an image and a
  shadow passes the paint rules at 1x and 2x (F3.2).
- W10 added; W1 and W4 with batch counts before and after F3.1.
- A scroll writes no clip entry; Idle writes none (`clip_writes`).
- The `kept_bytes` debug compare covers the clip index.
- The chain-length refusal and the storage-buffer refusal, each proven
  by a break.
- A click outside a rounded clip's corner but inside its rectangle
  misses, as in Chromium (events oracle, F3.2).

Each check is proven by a break.

## Questions for the lead

1. **Clips inside lane rows.** (a) An entry can be marked as in the
   instance's own row: its rectangle is relative to the row, and the
   shader adds the instance's row offset (the batch's row range at
   `row`). This works because a box that clips inside a row clips only
   instances in that row. (b) Clip entries in a batch-local range, as
   row offsets are. (a) keeps one table and stable indices; (b) gives a
   row's clip a different index in each batch, so kept bytes would be
   rewritten when batches change. Recommended: (a).
2. **Units.** (a) Device pixels in the frame's space, snapped as
   today's clips are, so rectangle clips keep today's pixels. (b) CSS
   px, scaled in the shader, which F4a's unsnapped boxes will want.
   Recommended: (a) for F3; F4a adds its class's handling where it adds
   the shader branch.
3. **Entry size.** 40 bytes padded to 48 (circular radii, as quads
   draw), or 64 with elliptical radii (`border-radius: 10px / 20px`).
   Elliptical box corners are not drawn today either. Recommended: 48
   bytes, with an elliptical radius on a clipping box refused naming
   the property, as on a box.
4. **`MAX_CLIP_CHAIN`.** Measured in F3.1 on the corpus and the demos;
   the constant is that number with headroom, declared with the
   measurement. Recommended: decide the number from the measurement,
   not before.
5. **The split.** Recommended, each keeping pixels unchanged:
   - **F3.1:** the table, `clip.wgsl`, rectangle clips moved from the
     scissor to the table, batches no longer split by clip, the limits,
     W10, and W1 and W4's counts.
   - **F3.2:** rounded clips and the four refusals retired, the
     Chromium fixture, and hit testing through rounded clips.
6. **Clips under a transform.** Before F4a every frame is a whole
   translation, so a link's move is a subtraction. F4a's affine frames
   need the inverse of the frame's matrix per fragment, or an inverse
   kept beside each transform entry. Recommended: left to F4a, with
   F3's debug assertion naming it.
