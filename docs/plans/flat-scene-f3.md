# Flat scene F3: clips as data

A design note for F3 (`flat-scene.md` section 8, the F3 row; the clip
table is section 2.3), written against main and against F2.1 as built
in #459 (`flat-scene-f2.md`, "As built: F2.1"; the conventions lane3
listed on #459, issuecomment-6937542558). The clip table follows the
transform table: a storage buffer, entries written only when they
change, an entry changed twice for one encoder refused, growth by
doubling under the buffer limits and `ArenaFull` beyond them, writes
counted by the table's own `WorkCounter`, entries given back after a
frame that does not draw them, and a device that lacks what it needs
refused naming the limit. Revised after the review of #460
(issuecomment-6937889836).

## Decisions (the lead, 2026-11-17)

The six recommendations of the first draft are taken:
- **Clips inside lane rows:** an entry relative to the instance's own
  row (item 3 below says how many entries a lane makes).
- **Units:** device pixels, snapped as today's clips are.
- **Entries:** 48 bytes with circular radii. An elliptical or
  unequal-axis `border-radius` on a clipping element is refused by name
  in F3.2, not drawn as circular; what would retire that refusal is a
  slice that draws elliptical corners on boxes, which also draw
  circular corners only today.
- **`MAX_CLIP_CHAIN`:** set from a measurement, the measurement cited
  at the constant. A longer chain is refused naming the element.
- **The split:** F3.1 and F3.2, below.
- **Clips under a transform:** left to F4a.

## Today

- **Elements.** A box's clip is one rectangle in CSS px, the
  intersection of its clipping ancestors' content boxes (`clip.rs`,
  kept per slot by `kept_clips.rs`, parents from `clip_tree.rs`). Paint
  snaps it to device pixels and sets it on the scene before the box's
  items (`paint.rs` `clip_to`); hits are cut to the unsnapped
  rectangle.
- **Scene.** Every item carries an `Option<Rect>` clip in its frame's
  device pixels (`vapors-scene` `display.rs`); `batched` starts a new
  batch when it changes (`batch.rs` `push`).
- **Painter.** A batch's clip, moved by its frame's shift and cut to
  the frame's own clip (the scrollport), is its scissor
  (`painter_runs.rs`, `painter_region.rs` `clipped`); a batch whose
  scissor is empty is not drawn.
- **Rounded clips are refused:** `style/overflow.rs` (a clipping box
  with a radius), `img_checks.rs` and `svg_style_checks.rs` (a radius
  on an image or an icon), and a rounded clip on a `gpu-surface`
  ancestor (`docs/gpu-surface.md`, "Rounded clip on an ancestor").

## What F3 changes

1. **The table.** `ClipTable` in `vapors-renderer`, built as
   `TransformTable` is (`transform_table.rs`, `transform_entries.rs`):
   a storage buffer at binding 2, beside the view uniform (0) and the
   transform table (1), read in the fragment stage. An entry, 48
   bytes:

   | field | type | bytes |
   |---|---|---|
   | `rect` | `vec4<f32>`, device px in its space | 16 |
   | `radii` | `vec4<f32>`, circular, per corner | 16 |
   | `frame` | `u32`, the transform entry its space is in | 4 |
   | `parent` | `u32`, the next clip outward, 0 for none | 4 |
   | `flags` | `u32`, bit 0: relative to the instance's row | 4 |
   | padding | | 4 |

   Index 0 is no clip, held for good, as the transform table holds
   entry 0. The flag is per link, so a chain can mix a row-relative
   link with links in outer frames.
2. **Lifetime: drawn each frame, as frame entries are.** The painter
   keeps an entry per clip it draws under and gives it back after a
   frame that draws nothing under it (`painter_transforms.rs`
   `release_undrawn`'s rule). A box that leaves view and returns may
   get another index; `KeptGpu` builds again the bytes of every kept
   batch that names a changed index, before the frame's draws, as it
   does for a frame entry today, so a kept instance never names a freed
   or reused entry. Test:
   `a_clip_index_given_back_and_taken_again_rebuilds_the_bytes_naming_it`.
3. **Keys and counts.**
   - Outside lanes: one entry per clipping box drawn, keyed by its slot.
   - Inside a lane: rows that are instances of one template share an
     entry when their clipping box has the same rectangle and radii
     relative to the row, with the row-relative flag set; the shader
     adds the instance's row offset (the batch's row range at `row`, as
     F2 does for positions). A row whose clipping box differs (its own
     height, say) gets its own entry, keyed by the template's key and
     the values. So a 3,000-row lane of equal rows holds one entry per
     template clip: mount writes 1, an insert writes 0 (F2 rewrites the
     row ranges), a scroll writes 0. At worst every row differs: 3,000
     entries, 144 KB, mount writes 3,000.
4. **The frame index stays valid.** An entry stores its frame's
   transform index, which F2 can change when a frame entry is given
   back and taken again. The painter keeps, per frame entry, the clip
   entries that name it, and rewrites them when that frame entry's
   index changes, in the same pass that rebuilds the kept bytes naming
   the frame. The release order: after a frame, clip entries are given
   back first, and a frame entry is then held while any held clip
   entry names it, even when the frame draws nothing itself (an outer
   frame whose content is all in inner frames, say), so a held clip
   never names a released frame entry. Tests:
   `a_frame_drawing_nothing_under_a_held_clip_keeps_its_entry` and
   `clip_entry_follows_its_frame_entry_after_the_frame_leaves_and_returns`
   (released, taken again at another index, the clip entry rewritten).
5. **Rectangles fold, and what that costs.** A chain of rectangular
   clips in one frame folds into one rectangle on the CPU, as today's
   intersection does; an entry's parent names the next clip that cannot
   fold (a rounded one, or one in another frame). A box that does not
   clip names its nearest clipping ancestor's entry. A layout change to
   a clipping box rewrites the folded entries of the clipping boxes
   under it that are drawn: O(those boxes), found through
   `kept_clips.rs`'s dependents, as today's clips are made again. In W1
   (100 scrollers), resizing the outer scroller rewrites at most the
   100 inner scrollers' entries that are drawn. An ancestor gaining or
   losing `overflow` changes its descendants' indices, and their kept
   bytes are built again. Test:
   `an_ancestor_gaining_overflow_rebuilds_the_bytes_of_the_boxes_under_it`.
6. **`MAX_CLIP_CHAIN`.** The links a fragment walks: one per rounded
   clip or frame boundary. One constant in Rust, emitted into the WGSL
   prelude that is concatenated ahead of the shaders, so the shader's
   loop bound is the same number; a test asserts the WGSL text carries
   it. A longer chain is refused at paint, naming the element and the
   chain (`a_clip_chain_past_the_limit_is_refused_naming_the_element`).
   The value is the longest chain measured on the corpus and the demos,
   with headroom, cited at the constant.
7. **`clip.wgsl` and derivatives.** Concatenated ahead of each 2D
   pipeline's shader, as `transform_table.wgsl` is, and shared with
   `svg-support.md` V10. WGSL allows `fwidth` only in uniform control
   flow, and the chain walk is a loop on the instance's clip index.
   Checked on 2026-11-17 with three small shaders (in
   `flat-scene-f3/` beside this note):
   - naga 30.0.1, the repository's validator, with every validation
     flag, accepts `fwidth` inside the data-dependent loop
     (`loop_fwidth.wgsl`), and even under an `if` on the fragment
     position (`if_fwidth.wgsl`);
   - Chromium 155's compiler (headless, SwiftShader) rejects both:
     "'fwidth' must only be called from uniform control flow";
   - with the pixel footprint taken once before the loop
     (`masked_fwidth.wgsl`), both accept it.

   So `clip.wgsl` takes the footprint before the loop, and each link's
   coverage from its signed distance divided by that footprint. naga's
   acceptance is not the check; the page contract under Chromium is
   (item 12). Before F4a every frame is a whole translation, so a link
   moves the position by a whole-pixel subtraction and the footprint is
   the device pixel's. Index 0 ends the loop before its first read: an
   unclipped instance reads no entry.
8. **Coverage and pixels.** A rectangle on whole device pixels gives
   coverage 0 or 1 at pixel centres, the scissor's result, so quads,
   text and closed forms draw today's pixels. The cases that can
   differ, each a named fixture at a clip edge at 1x and 2x: a shadow
   whose blur reaches past the clip (the scissor cut it by whole
   pixels; coverage must cut it at the same pixels); an image (its
   clip moves from the scissor to the draw's uniform); text vertices;
   a closed-form shape. Clips on fractional device pixels keep
   `clip_to`'s snap. At a rounded corner where two links' edges meet,
   the product of their coverages is not the coverage of the
   intersection; the F3.2 fixture states its tolerance there.
9. **One more index per instance.** `clip: u32` beside `frame` and
   `row`: nothing for quad, shadow and closed-form instances (8 bytes
   of their padding are free, `instanced_bytes.rs` `instance_bytes`); 4
   bytes per text vertex after F2.2's index; 4 per mesh instance after
   F2.3's two; images take it in the draw's uniform.
10. **The scissor and joined batches.** F3.1 joins batches across clip
    changes within one frame only: the batch's scissor stays the
    frame's scrollport as today, and clips inside the frame become
    coverage. Pixels are unchanged by item 8, and a scroll costs what
    F2's does. Joining across frames is not in F3.1: a batch over
    several frames has no one scrollport, and a scissor over their
    union does no cutting while every fragment walks the chain, so it
    waits for W4's measurement of draws against fragment work (F3.3,
    the question below). Batches whose frame is scrolled out are still
    not drawn, as today.
11. **Freed entries.** The checks, in order:
    - **The CPU invariant, in every build:** no kept batch names a
      freed clip entry. When the painter gives an entry back, every
      batch that names it is already marked to be built again (the
      painter keeps, per entry, the batches naming it), asserted at the
      free: O(batches naming it). When a batch's bytes are built, each
      instance's clip index is looked up live as its bytes are written,
      and a dead one panics naming the batch and the index: no work
      beyond the per-instance write the rebuild already does. Test:
      `a_batch_naming_a_freed_clip_entry_panics_at_its_rebuild`.
    - **The debug compare:** `kept_bytes`, extended to the clip index,
      fails on a kept instance naming a freed or reused entry. Test:
      `a_freed_clip_entry_named_by_kept_bytes_fails_the_compare`.
    - **The backstop, declared relaxed:** a freed entry is written with
      a NaN rectangle in every build, so a stale reference the two
      checks above missed covers nothing. In release that is missing
      content with no error, since the GPU cannot report it; it stays
      only as a backstop behind the CPU invariant, and is retired if
      the GPU gains a way to report it (a debug readback in a test is
      the nearest today).

    Removing one box's instances from a batch joined across its clips
    builds that batch's bytes again, as a changed item does today.
12. **The device.** The fragment stage reads two storage buffers (the
    clip table, and the transform table for each link's frame). F2.1
    already refuses `max_storage_buffers_per_shader_stage` below
    wgpu's default of 8 (`vapors_gpu::raised_limits`), so F3 adds no
    limit there. A table past `max_storage_buffer_binding_size` is
    refused by `ArenaFull`, as the transform table's is. The clip
    shader's compile is checked under Chromium by the page contract,
    and on the Pixel 9 Pro by hand at F3.1's end. The contract compiles
    every 2D pipeline, not only those its pages draw with: `Painter::new`
    builds each of its pipelines (quads, shadows, closed forms, meshes,
    text, images) when it is made (`app/src/painter.rs`), and one-scene
    F0's rule that every pipeline is built before the first frame,
    pinned by `gpu_app_start.rs`
    `gpu_a_started_app_builds_no_pipeline_in_its_first_frame`, keeps it
    so: a pipeline made lazily would be built in a frame and refused.
    Every elements demo the contract opens therefore has Chromium's
    compiler compile every 2D shader with `clip.wgsl` concatenated, and
    a compile error stops the page through the uncaptured-error watch
    (`platform/src/gpu_watch.rs`), which fails the case. The scene
    pipelines (`closed_forms_3d.rs`, `meshes_3d.rs`, `projective.rs`)
    are compiled only by the scene pages (one-scene, racer); F3 does not
    change them.
13. **Rounded radii (F3.2).** A clip's radii are its padding box's: the
    border radius less the border width, clamped at 0, after CSS's
    rule that scales all radii down when adjacent ones exceed a side.
    Tests: `a_border_wider_than_its_radius_gives_a_square_clip_corner`,
    `radii_past_a_side_are_scaled_down_as_css_says`. The elliptical
    refusal is in `style/overflow.rs`, naming `border-radius` and the
    element.
14. **Hit testing (F3.2).** Hits stay on the CPU, cut by the box's clip
    chain in CSS px; a rounded clip cuts them by its rounded shape.
15. **The other class.** F1 already panics on a frame class other than
    `WholeTranslate`, naming F4a, in every build; that is the guard for
    clips under a transform, and F3 adds none.

## The split

- **F3.1:** the table, `clip.wgsl`, `MAX_CLIP_CHAIN`, rectangle clips
  moved from the scissor to the table, batches joined across clips
  within a frame, W10, and W1 and W4's counts.
- **F3.2:** rounded clips with the radii rules, the four refusals
  retired (each test converted into a test of the clip drawn), the
  elliptical refusal, the Chromium fixture, hit testing.
- **F3.3 (on a measurement):** batches joined across frames, if W4
  shows draws costing more than the fragment work a union scissor
  would add.

## Acceptance

- F3.1 draws the same pixels as the scissor path: every Chromium
  comparison and `gpu_*` test unchanged, and item 8's fixtures (a
  shadow blurred past a clip edge, an image, text and a closed form at
  a clip edge, at 1x and 2x) compared byte for byte with images
  committed from the scissor path before F3.1 removes it.
- `clip_writes` 0 on a scroll and on Idle; 1 at mount for a lane of
  equal rows; the W1 resize bound of item 5.
- Batch counts on W1 and W4 before and after F3.1, measured in F3.1,
  with the prediction recorded first: within a frame, one batch per pipeline run where
  today clip changes split runs; across frames, unchanged until F3.3.
- The tests named in items 2, 4, 5, 6, 11 and 13; the kept-bytes
  compare covering the clip index.
- F3.2: a Chromium fixture of a rounded scroller holding text, an image
  and a shadow passes the paint rules at 1x and 2x, with the corner
  tolerance of item 8; a click outside a rounded corner but inside its
  rectangle misses, as in Chromium (events oracle).

Each check is proven by a break.

## Question for the lead

1. **Joining across frames (F3.3).** F3.1 joins within a frame and
   keeps today's scissor (item 10). Recommended: decide F3.3 on W4's
   numbers after F3.1, not now.
