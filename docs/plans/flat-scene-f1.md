# Flat scene F1: the transform class on frames

A design note for F1 (`flat-scene.md` section 8, the F1 row; the class
itself is section 4.3), written against main at ff808875. It says what
the code does today, what F1 adds, and the three questions the lead
answered before it was built.

## Today

- Paint snaps every edge with `paint::snapped` (`half_up` of the rect
  times the scale) at each paint site. The sites are `paint_own`,
  `paint_inline`, `paint_outline`, `text_paint` (decorations), `images`,
  `surfaces`, `svg_paint`, `svg_shapes_paint` (the origin),
  `checkbox_paint`, `range_paint` and `thumb`, plus the clips
  (`paint::clip_to`, and `device_clip` in `frames.rs` and
  `frames_lane_prime.rs`).
- A frame is a `FrameAt { shift, clip, device_clip }` (`frames.rs`).
  Shifts are made in four places:
  - `Frames::page` (zero);
  - `Frames::frame_at` (scroll frames, from `scroll::frame_shift`, which
    rounds);
  - `frames_relative.rs` `composed` (the outer shift plus the frame's
    own whole offset);
  - `frames_lane_prime.rs` `lane_child_at` (the template's shift plus
    the child's start in device pixels).

  A framed lane's starts are whole, because a lane with a fractional
  start is declined (`LaneDecline::LaneFramePixel`). Nothing checks that
  a shift is whole.
- A part's frame is `BoxFrom::frame` (from `KeptClips::frame`).
  `Repaint::painted` paints one slot at a time, so the frame of each
  part painted is known there.
- `kept_shift_items.rs` `whole_item` already tests an item's rects
  whole: quads, shadows, images, icons and surfaces. It answers false
  for text and shapes.

## What F1 adds

1. **`TransformClass` in `vapors-scene`**, with the three variants of
   section 4.3. It goes in a new `transform_class.rs`: `transform.rs` is
   the image viewport's `Transform`, which is unrelated. `snaps(self)`
   is true for `WholeTranslate` alone. A unit test covers each variant.
2. **`FrameAt::class`.**
   - Each frame takes the class of the frame it is made under (the
     outer frame, or the lane's template).
   - The page is `WholeTranslate`.
   - No class is worked out from floats in F1, as nothing can make an
     `Affine` frame before F4a.
3. **Paint reads the class.**
   - `Repaint::painted` and `Repaint::thumbs` set `PaintInput::class`
     from `painted.frames.at(frame)` before painting a slot.
   - The paint sites call `input.snapped(rect)` in place of
     `snapped(rect, input.scale)`.
   - Under `Affine` or `Projective`, `input.snapped` panics naming F4a.
     The unsnapped path is F4a's, built with its Chromium fixtures;
     written now, it would be a branch no test reaches.
   - Clip snapping is unchanged: section 4.3 keeps the scissor for a
     rect clip in a `WholeTranslate` frame, and F3 changes clips.
4. **Two debug assertions.** Together they say that every snapped item
   under a `WholeTranslate` frame is whole on the page:
   - (a) Where a shift is made (the four places above), a
     `WholeTranslate` frame's shift is whole on both axes. Otherwise it
     panics, naming the frame (the scroll container's slot, or the
     lane's container and the child's offset) and the shift. The proof
     is a break that adds 0.5 to `frame_shift`.
   - (b) In `Repaint::painted`, every item of the new part whose kind
     `whole_item` covers is whole. Otherwise it panics, naming the slot,
     the frame and the item. The proof is a break that drops `half_up`
     from `snapped`.
   - Text is left out. Its origin is the rect times the scale, and the
     painter quantises glyph positions (section 4.4, regime A).
   - Shapes are left out. Their geometry is in their transform,
     unsnapped, as Chromium paints SVG.
   - Cost: debug builds only, one check per frame shift made and one
     per item painted again.
5. **Tests.**
   - Frames carry `WholeTranslate`: the page, a scroll frame, a framed
     lane's child, and a scroll frame under a lane frame (in
     `frames_tests.rs`).
   - The class's `snaps`.
   - "Pixels unchanged" is every Chromium comparison and `gpu_*` test,
     unchanged.

Estimate: 250 to 300 lines with tests.

## Questions for the lead

Answered by the lead (2026-11-16): each recommendation below is taken.
`parts_reclassed` goes on `UpdateCounts` in F4b; W6 is its own pull
request after F1; paint under `Affine` or `Projective` panics naming
F4a.

1. **Where `parts_reclassed` goes, and when.** Section 8 puts it on
   `FrameCounts` (`vapors-app`, the painter's GPU work a frame). A
   reclass paints parts again, which is update work, counted where
   `boxes_drawn` and `scene_items_built` are: `UpdateCounts`. Options:
   - (a) Add `UpdateCounts::parts_reclassed` in F1, pinned at 0.
     Nothing increments it before F4b, so no break can prove the pin.
   - (b) Add it in F4b, with the code that increments it and the test
     that breaks it.

   Recommended: (b), on `UpdateCounts`.
2. **W6 (`Positioned`) in F1's pull request or its own.** Section 8's
   last row gives each workload its own pull request. W6 is a new
   off-path kind (`off_path_kinds.rs`), about 100 lines with tests and
   its first numbers in `docs/perf/`. It does not depend on F1.
   Recommended: its own pull request after F1, which keeps F1 small.
3. **Paint under a class that is not `WholeTranslate`, before F4a.**
   Recommended: the panic in 3. The alternative is the unsnapped path
   written now, which no test would reach.

## Risk

Assertion (b) may fire on today's fixtures, for example on an image
whose drawn rect is not whole. If it does, the case is reported with
its fixture before F1 goes further: it is either a pixel difference
from Chromium or a gap in the assertion's kinds, and the lead decides
which.
