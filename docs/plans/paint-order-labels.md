# Paint-order labels (layout-lane PR 4d-1)

Design note for review before code. Context: `docs/plans/splice-layout.md`,
section 11.2's "Splice patch" (4d). Lead approved option A (a note first),
2026-11-14.

## 1. The problem

A step's place is its index in `KeptOrder.steps`. Five structures hold
places:

| Structure | What it holds | How it uses a place |
|---|---|---|
| `KeptOrder.steps` | the steps, by place | the index itself |
| `DrawnSteps` | `places` by slot; `drawn`, `(place, step)` in blocks | order: binary search, sort |
| `Extents` | each frame's entries, each with its place; `at`, by place | order among equal tops; `at[place]` finds an entry |
| `KeptBatches` | the shown steps of a draw, with their places | order: sorted by place, then batched |
| `KeptHits` | `(place, hit)` ordered by place | order: binary search, reverse scan |

An insert of `k` steps at place `p` adds `k` to every place after `p`. So
today a splice makes `DrawnSteps`, `Extents` and the hits afresh
(`paint_partial.rs` `drawn_updated`), O(steps). Patching them by the
splice's edits would still add `k` to every later entry: an O(rows) loop.

The budget: FlexInsert's paint phase is about 3.5M instructions at 1000
rows. For section 11.3's "within 2% across sizes" (1000 to 10,000 rows),
that leaves about 8 instructions per extra row for all of paint. A loop
that touches every later entry, even in blocks, takes more.

## 2. What every reader needs

Apart from `steps` and `Extents.at`, every reader compares places: it
needs the order of two steps, never the distance between them or a count.
`steps` is read in order (walks, the splice). `Extents.at` uses the place
as the key of a dense vector.

## 3. Options

**A. Order labels (recommended).** Each step holds a `u64` label. Labels
increase along paint order and leave gaps. A step put in between two
others takes a label between theirs. Every comparison reader works on
labels unchanged, since a label orders exactly as a place does. Nothing
after the insert moves.
- When a gap runs out, a window of neighbouring labels is spread out
  again (list labelling, as in Dietz and Sleator's order maintenance):
  the smallest window around the insert whose labels have room is
  relabelled evenly. Those steps' entries are patched in each structure,
  as if painted again. Amortized O(log n) labels per insert. With `u64`
  labels and an initial spacing of 2^32, a window relabel needs more than
  32 inserts into one gap, which the rows pages do not reach. The debug
  build counts relabels (`UpdateCounts`).
- `Extents.at` becomes a map keyed by step (slot and kind), or a vector by
  slot with three kinds, as `DrawnSteps.places` is. It is no longer by
  place.
- `KeptOrder.steps` becomes blocks (`blocks.rs`) of `(label, step)`: a
  splice inserts and removes in blocks, O(CAP + log n), instead of
  copying the vector (`kept_order_splice.rs` `spliced_steps`). A walk
  makes it afresh with labels spaced evenly.

**B. Counted blocks (a rope).** Places stay positions, computed when read:
each block keeps its length and an index of prefix counts, and a place is
`prefix(block) + offset`, O(log n). Every reader that holds a place holds
a handle (block and offset) instead and asks for the place on each
comparison. That touches every comparison in five structures, adds
O(log n) to each, and handles go stale when a block splits.

**C. Leave the rebuild.** Paint stays O(steps) after a splice and the fold
does not meet 11.3. The lead ruled it out.

A keeps every comparison an integer compare and confines the change to
how a place is made, not how it is read. B changes every reader. So A.

## 4. A in each structure

- **Labels.** `PaintOrder` hands them out: `afresh(n)` spaces `n` labels
  evenly over `u64`; `between(a, b)` is the midpoint, or `None` when
  `b - a < 2`, which asks for a window relabel.
- **The splice** (`kept_order_splice.rs`). Each `Edit` (at, removed,
  inserted) becomes: take the removed steps' entries out of every
  structure (`DrawnSteps`, `Extents`, hits) by step; label the inserted
  steps between their neighbours'; put their entries in, as the patch for
  a slot painted again does today. O(steps in the edit × log n).
  `drawn_updated` stops making the three afresh after a splice. A walk
  still does.
- **`DrawnSteps`.** `places` by slot holds labels. `drawn` is ordered by
  label. `patch` is unchanged.
- **`Extents`.** An entry's `place` becomes its label (the tie-break among
  equal tops), and `at` is keyed by step. The splice's entries go in and
  out through `patch` (`frame_extents_patch.rs`). For a framed row, each
  row is its own frame index, so the new row's steps make one new
  `FrameIndex` and no other frame's entries move.
- **`KeptBatches`.** Sorts the shown steps by label instead of place.
  Batch matching across draws (`kept_batches_keys.rs`) matches by
  position in the shown list, not by place, so it is unchanged.
- **`KeptHits`.** `(label, hit)`, ordered by label. A splice takes the
  removed slots' hits out and puts the inserted slots' hits in, as
  `patch` does for slots painted again.

## 5. Checks and failure

- Debug: after every paint, labels strictly increase along
  `KeptOrder.steps`. Refuse otherwise, naming the first two steps out of
  order.
- The kept-paint check (`kept_paint_check.rs`) compares `DrawnSteps`,
  `Extents` and the hits with ones made afresh. Made afresh, labels
  differ, so those comparisons compare by order: each structure's steps,
  in label order, against the fresh one's in place order.
- A relabel that cannot find room (the whole list's labels are packed)
  relabels everything, counted as a walk: never a silent overlap.

## 6. Slices

1. `PaintOrder` labels and blocks for `KeptOrder.steps`. Every reader
   moves from place to label, and every splice still makes the three
   afresh. Behaviour unchanged; the kept-paint check compares by order.
2. The splice patches `DrawnSteps`, `Extents` and the hits by its edits.
   Accepted when FlexInsert's and Insert's paint phases are within 2%
   across sizes (with 4e for Insert), and the Chromium paint comparisons
   and the kept-paint check are unchanged.

## 7. Open questions for review

- `u64` labels with an initial spacing of 2^32: enough, or a relabel
  policy test at 64 inserts into one gap?
- `Extents.at` keyed by step: a map, or a vector by slot (dense, as
  `DrawnSteps.places`)? I lean to the vector, as slots are dense.
- 4d-2 (items 2 to 4) stays separate and lands first, being independent.
