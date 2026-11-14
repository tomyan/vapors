# Paint costs per scroll frame (2026-11-14)

Measured with `vapors-perf --off-path --instructions` on the nested
scrollers and code blocks pages (#275), where every unit is its own
scroll frame. Values are instructions per frame at 1,000 / 3,000 /
10,000 units.

## Fixed

| term | where it was | PR |
|---|---|---|
| Insert paint, quadratic in scroll frames (368 M / 3.18 G / 34.8 G) | `thumb_order.rs`: each thumb searched every paint step | #283 |
| every thumb repainted and patched on every update (~6,000 per frame) | `paint_partial.rs`, `repaint.rs` | #290 |
| every frame made afresh and compared on every update (~700 per frame) | `paint_partial.rs` `frames` | #292 |
| every frame's extents index searched on every sort (~280 per frame, IU4 M2) | `kept_batches.rs` | #298 |
| two sums over every frame's extents index on each patch (~12 each) | `frame_extents.rs` | #316 |

With all of them applied, Edit paint is flat: about 32 k on nested
scrollers and 21 k on code blocks. ScrollInner paint is about 15 k, and
batches on Scroll and ScrollInner are flat at about 132 k on nested
scrollers and 38 k on code blocks. Insert paint is linear: 24 M / 72 M /
242 M on nested scrollers. That is iu4.md's accepted insert floor until
the layout lane.

## Still O(units)

- **Body Scroll, paint: 0.67 M / 1.9 M / 6.4 M on both kinds.** Each
  frame keeps its shift on the page (`frames.rs` `FrameAt::shift`, the
  sum of the scroll offsets up its chain). Every unit's frame is nested
  in the body's, so a body scroll changes every frame's shift and clip,
  and `Frames::update` works them all out again. Flattening this needs
  frames kept relative to their outer frame and composed on read:
  O(depth) for each read of an item, a hit or a rect, plus the painter's
  per-frame translation (4c). That is a design change to section 3 of
  iu4.md, not a patch.
- **Nested scrollers Edit, layout: 1.36 M / 3.81 M / 12.4 M (about
  1,200 per unit).** The paint side of this is now flat. Code blocks'
  Edit layout is flat (85 k), so the term comes from the row shape: each
  row is a horizontal scroller of four cells. Not investigated further
  here; it belongs to the layout lane (`docs/plans/splice-layout.md`).
