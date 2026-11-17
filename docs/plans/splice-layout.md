# Splice layout: the layout lane for inserting or removing a row

Property 3 of `docs/plans/perf-finish-line.md` (inserting or removing
a row costs about what an edit costs), the layout floor that
`docs/plans/splice.md` section (e) and `docs/plans/iu4.md` section 3
leave open: after splice A and B an insert builds one row's boxes and
splices its paint steps, but the list container is laid out again by
Taffy over every row, every row after the insert is placed again,
shifted in paint and read again by the a11y take. Design only; read
from the code at 9b405f9 (origin/main, 2026-11-12: splice A and B,
M10c, IU4 PR 1 merged; IU4 4-i (#248) and R6 5a (#243) open; line
numbers are from that revision). Numbers marked "measured" name their
run; every other number is an estimate for the measurement to replace.

The question the lead asked: can an insert or remove cost about the
same as an edit including layout, not just paint? Answer: yes, when the
list container's children are positioned by a sum over their sizes
that an insert updates in O(log n), rather than by Taffy writing each
child's location; and only if the four other O(rows) remainders listed
in section 1.3 go too, since each alone is already several edits'
worth at 1000 rows.

## 1. What an insert costs today, per phase

### 1.1 Measured

`vapors-perf 60 10 --scaling Insert` at 9b405f9 (release, Intel UHD
630, 640 x 400, scale 1; run 2026-11-12 at load average 47, which
instruction counts do not see: two runs agreed within 0.1% on every
phase), median instructions per frame. `accessibility` and `draw` are
frame phases beside `update`, not inside it.

| phase | 1000 rows | 3000 rows | 10,000 rows | per extra row, 3000 to 10,000 |
|---|---|---|---|---|
| update | 11,961,100 | 35,626,004 | 119,930,959 | 12,044 |
| of which style | 59,921 | 151,557 | 467,788 | 45 |
| of which layout | 3,534,637 | 10,243,074 | 34,056,089 | 3,402 |
| of which paint | 7,970,695 | 24,204,155 | 81,860,066 | 8,237 |
| accessibility | 8,787,219 | 25,678,959 | 86,456,938 | 8,683 |
| draw | 3,373,939 | 4,546,446 | 8,643,762 | 585 |
| frame (CPU) | 24,163,170 | 65,867,229 | 215,018,041 | 21,307 |

Wall clock at the same revision (the lead's timed run, median of 3,
µs): update 3,127 / 10,612 / 39,457 at 1000 / 3000 / 10,000 rows
(layout 1,134 / 3,774 / 14,262; paint 1,777 / 6,422 / 23,803), draw
848 / 1,617 / 3,970. Property 1's target is an update under 1 ms.

An edit (Text) is 123 k instructions for its whole update at every row
count (`docs/plans/iu1.md`, PR 2). An insert's update is 97 edits at
1000 rows and 975 at 10,000, and paint, not layout, is the larger
half of it. `--scaling Insert` alternates an insert and a remove per
frame, and the median over both kinds is the cheaper kind's slowest
frame (`docs/perf/baseline-2026-11-10.md`, "Insert"), so these are one
kind's numbers (the remove frames'); section 7's PR 0 reports the two
apart, and `docs/perf/splice-layout-2026-11-13.md` has the split: an insert
costs about 630,000 instructions more than a remove at every size, in
`style`, and the same per extra row. Section 1.4
splits the phases.

### 1.2 Read from the code: where every row is touched

A row inserted before the middle row of the rows page
(`crates/perf/src/rows.rs:124-137`: `Document::insert_before(body,
added, Some(middle))`), `n` rows, the insert at `k = n/2`.

| # | Phase | What touches every row, or every row after `k` | Where | Order |
|---|---|---|---|---|
| 1 | style | The restyle reaches the inserted row only (splice (b2)); the rows page has no sibling selector | `style/sibling_deps.rs` | O(row) |
| 2 | build | `build_shallow` walks every DOM child of the owner (the body) to make its `children` and the placeholder map, one hash lookup per kept child | `layout/boxes.rs:41-57`, `add_element` line 100 | O(n) |
| 3 | build | `Rebuilt::subtrees` clones the owner's old children and `reshaped` compares the two vectors | `layout/rebuilt.rs:49-64`, `72-` | O(n) |
| 4 | layout | The body's cache is not lent: its children differ (`before.children == children` false) | `layout/keep.rs:149-151` | — |
| 5 | layout | The body is a changed owner; `isolate` replays each way html asked it, and each replay is a whole `compute_block_layout` over every row: `generate_item_list` reads every child's style, `determine_content_based_container_size` asks every in-flow child for its size, `perform_final_layout_on_in_flow_children` asks every child for its layout and the deferred loop writes every child's `Layout` | `layout/isolated.rs:139-150`; taffy 0.15.1 `compute/block.rs:876-895`, `1040-1065`, `1085-1500`, `786-790` | O(n), about 2 cache lookups, a style read and a layout write per row per replay |
| 6 | layout | Each child's calls hit its cache; a hit still builds a `CacheKey` and compares it against up to 9 measure entries | `layout/tree_cache.rs:67-69`; taffy `tree/cache.rs:254-281` | per call |
| 7 | layout | The body's answer differs (its height grew a row): `Differs` clears html's cache and `compute_root_layout` lays html out; the body answers from the cache the replay filled | `layout/isolated.rs:117-119`, `layout/mod.rs:92-96` | O(1) |
| 8 | layout | `set_unrounded_layout` finds every row after `k` moved and lists it in `written.changed` | `layout/tree.rs:186-192` | O(n − k) |
| 9 | placement | `chain_children` chains every child of every slot laid out (the body, twice) | `placement.rs:177-192` | O(n) |
| 10 | placement | `mark` marks every slot in `written.changed` `Dirty` and notes it under the body | `placement_marks.rs:49-77` | O(n − k) |
| 11 | placement | The walk visits each marked row; its content origin moved, so it visits the row's six descendant boxes too, and `put` writes seven rects and reports seven moved | `placement_walk.rs:86-128`, `147-160` | 7 (n − k) |
| 12 | paint | The order splice: `plan` builds a hash map of the owner's old children and a set of the new ones; `spliced_steps` copies the whole step vector | `order_splice.rs:66-72`, `114-115`; `kept_order_splice.rs:59-77` | O(n), O(steps) |
| 13 | paint | `ClipTree::spliced` regroups the dependents of every slot | `clip_tree.rs:42-58` | O(slots) |
| 14 | paint | `to_paint` reads each moved slot's `BoxFrom` and decides a shift; `kept_shift::apply` rewrites each shifted part's items and hit; `kept_maps::fill` puts four elements' rect and padding entries per row | `paint_partial_read.rs`, `kept_shift.rs:62-119`, `kept_maps.rs:39-54` | 7 (n − k) parts |
| 15 | paint | After a splice the drawing steps are made afresh and the kept hits and batches dropped (the follow-up PR of splice B shifts them instead); the next hit test gathers every hit, the next draw tests every item against the view | `paint_partial.rs:85-93`, `drawn_steps.rs:22-35`, `kept_hits.rs:27`, `kept_batches.rs:42-60` | O(steps), O(hits), O(items) |
| 16 | accessibility | `touch_slots` touches every rebuilt and moved slot; the take reads each touched row's nodes and compares their records; with #243 a row's bounds are from the body's border box, which still change for every row after `k` | `a11y_mirror_state.rs:70`, `a11y_mirror_reach.rs:19-48`, `a11y_mirror_bounds.rs` (#243) | O(n − k) and an O(n) part (1.4), 8,683 per row measured |
| 17 | draw | `KeptBatches::of` after the remake of 15: every item tested against the view | `kept_batches.rs:42-60` | 585 per row measured; IU4 PR 5's extents index |

Also O(n) but not per insert: `Offsets::range` reads every child's
layout to clamp a scroll container's offset (`scroll.rs:160-175`),
paid when the body scrolls and is settled (every insert into a
scrolling list, since the body is `Dirty`).

### 1.3 The five remainders

Grouping the table: an insert pays O(rows) five times over, in five
owners' code.

1. **Layout** (rows 4 to 8): Taffy's block algorithm lays the
   container out over every child, and writes every later child's
   location. This is the floor `iu4.md` section 3 names, and what the
   lane in section 2 removes.
2. **Placement and the shift** (rows 9 to 11, 14): every moved box is
   placed again and its kept paint shifted. Correct and cheap per box,
   but 7 (n − k) boxes. Goes with the lane: a lane child's position is
   not stored, so nothing below it moves.
3. **The build and the splice's leftovers** (rows 2, 3, 12, 13, 15):
   listed in `docs/plans/splice.md` "As built" as the follow-up PR's.
   Not layout; needed for property 3 all the same, and on the splice
   lane's path (section 7, PR 1).
4. **The a11y take** (row 16): bounds relative to the parent node
   change for every row after the insert, so every row's record is read
   and written. Needs a mirror change (section 5.3) to be O(1) per
   insert; the largest term in the frame today, and with an O(n) part
   on an insert that moved rows do not explain (1.4).
5. **Draw** (row 17): IU4 PR 5.

Property 3 holds when all five are gone; none alone gets Insert under
the judge, since each is over 500 instructions per row (1.4) against an
edit's 123 k total (the judge fails a phase 5000 instructions and 2%
above its 1000-row count, `crates/perf/src/scaling.rs:20-29`).

### 1.4 Measured split

A prototype with an `update phase` span around each step of the
layout and paint phases (not pushed; the span names below are the
prototype's), `--scaling Insert` and `--scaling Resize` at 9b405f9,
median instructions per frame, and the growth per extra row from 3000
to 10,000 rows. Insert moves the n/2 rows after the middle; Resize (the
second row's height toggled) moves every row but two, so a step whose
cost is per moved row shows about twice Insert's growth under Resize,
and a step whose cost is per row of the page shows the same.

| step | what it is | Insert, 1000 rows | Insert, per extra row | Resize, per extra row | Reading |
|---|---|---|---|---|---|
| `l.rebuild` | `Document::rebuilt`: the shallow build of the body (`build_shallow`), `Rebuilt::subtrees` | 1,550,827 | 1,465 | 0 | per row of the page: the walk over every DOM child (1.2, row 2) |
| `l.taffy` | `lay_out`: `settle` (the body's replays) and the root | 1,190,006 | 1,160 | 1,130 | per row of the page: the container's block layout over every child (row 5) |
| `l.place` | `Placement::place` | 795,732 | 775 | 1,337 | about 1,100 per moved row (160 per box) and 225 per row of the page (`chain_children`, row 9) |
| `l.stackings` | `stackings`, `ChangedSlots::of` | 1,682 | 0 | 0 | flat |
| `p.order` | `KeptOrder::updated`: the splice, then `reclipped` | 2,263,172 | 2,328 | 3,786 | about 3,800 per moved row: the clips of every moved box redone (`kept_order_splice.rs:28`, `reclipped` over `rebuilt` and `moved`, 540 per box); the splice's own O(n) terms are the remainder, about 400 per row |
| `p.to_paint` | `to_paint`, `dropped`: each moved slot's `BoxFrom::of` and `Shift::of_box` | 4,214,792 | 4,458 | 9,071 | about 9,000 per moved row, 1,300 per box: reading each kept part against its new rect and deciding the shift |
| `p.shift` | `kept_shift::apply` | 492,913 | 491 | 983 | 983 per moved row, 140 per box |
| `p.maps` | `kept_maps::fill` | 395,731 | 393 | 787 | 787 per moved row: four elements' rect and padding entries |
| `p.repaint` | `Repaint::boxes`, `thumbs` | 6,009 | 0 | 0 | flat: the new row only |
| `p.drawn` | `drawn_updated` | 581,433 | 560 | 9,591 | Insert: the drawing steps made afresh after a splice, 80 per step, per row of the page; Resize: no splice, so each moved slot's steps patched in the drawn steps, the hits and the kept batches, about 9,600 per moved row |
| `u.touch` | `touch_slots` | 383,681 | 358 | 716 | 716 per moved row |
| `u.keep` | `KeptBoxes::keep` | 1,247 | 0 | 0 | flat |
| `p.cull` (in draw) | `Painted::batches`: `KeptBatches::of` after the remake | 626,448 | 586 | 0 | all of draw's growth; Resize patches instead |
| `style` | | 59,900 | 45 | 0 | small, per row of the page, not located: the restyle after an insert reads something per row |
| `accessibility` | the take | 8,807,692 | 8,699 | 11,672 | about 5,900 per moved row and, on an insert only, about 5,700 per row of the page that is not per moved row; not located here (R6's area; #243 changes the take) |

Sums: layout 3,400 per row is 1,465 build + 1,160 Taffy + 775
placement; paint 8,250 is 4,458 reading parts + 2,328 clips and splice
+ 560 remake + 491 shift + 393 maps. Per moved row, the kept-paint
side (clips 3,800, reading 9,000, shift 983, maps 787, touch 716:
about 15,300) costs more than layout and placement together (Taffy's
share of a row plus 1,100): the cost of a moved box is dominated by
deciding that it can be shifted, not by shifting it.

What this says for the design:

- Remainder 1 (Taffy) is 1,160 per row of the page: a third of the
  layout phase, a tenth of the update. The lane removes it, but the
  lane alone leaves an insert at over 10,000 per row.
- Remainder 2 (placement and the kept-paint reading of every moved
  box) is the largest term inside the update, about 15,000 per moved
  row across five steps. It goes only when the rows after the insert
  are not reported as moved at all, which is what derived positions
  (section 2 (c)) do; a faster `Shift::of_box` would still be
  O(n − k).
- Remainder 3 (the build walk, the splice's O(n), the remake, the
  cull) is about 2,600 per row of the page, 20 edits at 1000 rows on
  its own.
- Remainder 4 (the take) is the largest term in the frame and has an
  O(n) part on an insert that is not explained by moved rows.
- Resize, which has no splice, shows what Insert will pay once the
  splice follow-up stops the remake: `p.drawn` at 9,600 per moved
  row, patching each moved slot's steps, hits and batches. That is
  another reason for derived positions: with no moved slots there is
  nothing to patch.

## 2. What a layout lane is

A **lane** is a container box whose in-flow children are stacked along
one axis and laid out one at a time, and whose children's positions
along that axis are not stored. The lane keeps a **sum tree** over its
children's outer sizes along the axis, and answers from it in
O(log n):

- `start(child)`: where a child begins in the lane's content;
- `between(a, b)`: the children that intersect an interval of the
  lane's content (which rows are in the view);
- `total()`: the lane's content size along the axis, and the folds the
  container's own answer needs (its first baseline, its content-based
  cross size, its scrollable overflow).

Everything placed below a lane child is positioned relative to that
child: the child is a **frame** in IU4's sense (`docs/plans/iu4.md`
section 3, `frames.rs` in #248), and readers of a position below it
add the child's start, as they add a scroll container's shift. An
insert then changes one leaf of the tree and the sums on its path to
the root; no later child is laid out, placed, shifted, read for the
mirror or re-culled by its own position.

A lane has four parts.

**(a) The per-child layout.** The lane asks each child what Taffy's
block algorithm asks it, with the same inputs: its margins resolved
against the container's inner width, its width stretched or resolved,
its known height from a definite or percentage height, then
`perform_child_layout` with `SizingMode::InherentSize` and the
container's percentage-resolution height (taffy `compute/block.rs`,
the non-float, same-BFC branch of `perform_final_layout_on_in_flow_
children`, about 120 lines once floats and `align-content` are left
out, which vapors does not lay out). The child's output is its outer
size, margins, baseline and overflow contribution; its cross-axis
position (x in a column) is from the container's width and its own
width and margins, not from its siblings, so it is stored per child as
today. The child's Taffy cache works as today: a kept child whose
inputs are unchanged is not laid out, and the lane asks it only when
it is marked (rebuilt, or its container's inner width changed).

**(b) The sum tree.** An order-statistics tree over the children in
tree order, each leaf holding the child's summary (outer size; for a
block container also its top and bottom collapsible margin sets and
whether its margins collapse through it; its first baseline; its
overflow extent; its max-content width contribution), each inner node
the fold of its children's summaries. The fold is associative for all
of these, which is what makes it a tree (section 2.1). A child is
named by a handle, not an index: an insert shifts no other child's
handle, and `start(handle)` walks up from the leaf summing the
summaries to its left. Chosen shape: a B-tree of fanout 16 with parent
links, nodes in a slab, so a page of 10,000 rows is 4 levels deep and
an insert is at most 16 moves per level; `lane_tree.rs`, about 200
lines, with a property test that a random sequence of inserts, removes
and size changes folds to what a sequential pass over the children
gives (section 7, PR 2). A Fenwick tree does not insert in the middle;
a chunked vector (chunks of 64 with chunk sums) is simpler and O(√n),
and is the fallback if the B-tree does not fit a file.

**(c) Derived positions.** `Placement` (`placement.rs:27-40`) keeps a
rect per slot in its frame's coordinates (IU4 4-ii), and for a lane
child the axis coordinate is left at zero: `rect(slot)` on read is the
frame chain's shift plus, for each lane child on the chain,
`start(handle)`. The chain is O(nesting depth) with one O(log n) tree
walk per lane on it, and a read happens O(touched) times per update,
so the cost is per read, never per row. Placement's walk does not
enter a lane child whose own layout did not change, because its
content origin is not stored and so cannot have "moved": `moved` is
empty for the rows after an insert, `boxes_placed` is the new row's
boxes and the chain to the root.

**(d) The container's answer.** The lane is a layout algorithm in
`BoxTree::dispatch` (`layout/tree.rs:68-85`), chosen for a block
container that qualifies (section 4); its `LayoutOutput` is from the
tree's total and folds. The container is asked the same ways as today
(`asked.rs`), so `settle` compares its answer with the box it replaced
exactly as for any owner: an insert that changes the list's height is
`Differs` and lays html out; one inside a `height: 400px; overflow:
auto` list is `Same` and lays nothing above out. A lane child's own
height change (a row resized, or a nested lane's total) is a point
update to the tree, not a clear of the container's cache: the child is
a changed owner whose `Differs` today clears every ancestor
(`isolated.rs:117-119`); with a lane above it, the lane container
re-folds and answers its own parent, and only when that answer differs
is the next ancestor cleared. This is the part of the design that
changes `settle`: an ancestor that is a lane container is updated,
not cleared.

### 2.1 The fold

For a flex column with no grow, shrink or wrap: a child's summary is
its margin-box height; the fold of a segment is the sum plus one `gap`
per boundary; `start(child)` is the sum to its left plus one gap per
child to its left. Plainly associative.

For a block container: Taffy's `perform_final_layout_on_in_flow_
children` keeps a running `committed_y_offset` and an
`active_collapsible_margin_set` (positive max, negative min), and for
each child adds the collapsed margin between the previous child's
bottom set and this child's top set, then the child's height, and
takes its bottom set as the active set; a child whose margins collapse
through it adds nothing and joins its sets into the active one. A
segment's summary is then (top set, inner height, bottom set,
collapses through), and the fold of two segments L and R is: height
L.inner + R.inner + resolve(collapse(L.bottom, R.top)) when neither
collapses through, with the obvious joins when one or both do. This is
a monoid over the same values Taffy carries sequentially, so the fold
of the whole sequence is what Taffy computes; the debug check against
a fresh Taffy layout (`layout/layout_check.rs:34-81`) is what proves
it on every test update, and PR 2's property test proves the monoid
on random sequences. The first slice declines a child that collapses
through (`margins_can_collapse_through` on its output, which
`SizeAnswer` already records, `layout/asked.rs:50-55`), so the first
fold is the plain one; the collapse-through join comes with its own
test.

Sums are kept in f64 and the fold is applied in tree order, so a lane
child's start can differ from Taffy's f32 running sum in the last
bits. The debug check compares a lane child's location within 1/1024
CSS px (declared in `layout_check.rs` where it is relaxed, with the
reason: f32 is not associative; what would retire it: a Taffy that
sums in f64); the Chromium comparisons, which compare device pixels,
are unchanged unless a start sits within that of a half-pixel
boundary, which a comparison would show.

## 3. Designs compared

| Design | Insert, layout | Insert, positions below | Read a row's position | Verdict |
|---|---|---|---|---|
| L0, today: Taffy lays the container out | O(n) | O(n − k) rects written, parts shifted, records read | O(1) | the measured floor |
| L1, positions kept absolute and written lazily for the rows in view | O(log n) | O(visible) written, the rest stale | O(1) if fresh; a stale rect read is a silent wrong answer unless every read goes through an accessor that materialises, which is L2 with a cache | rejected: a stale state readers can reach |
| L2, derived positions: the lane as a frame (section 2) | O(log n) | none stored, nothing moves | O(depth log n) | recommended |
| L3, each row positioned relative to the previous (a linked offset) | O(1) | none | O(k) for row k: a hit test or cull at row 5000 walks 5000 links | rejected |
| L4, the same incremental block container inside Taffy | as L2 | as L2 | as L2 | not now: a feature PR upstream against the rule that external changes are issues (Tom, 2026-11-01); an issue describing it is worth filing after L2 works |

Within L2, the per-child layout can come from (i) vendored Taffy
per-child code (section 2 (a)), or (ii) laying the container out
through Taffy as if it had one child, by filtering `child_ids`. (ii)
is less code but collapses the child's margins with the container's
edges as a first and last child would, and sizes the container from
that one child, both of which the lane would then undo; and it is
behaviour Taffy does not promise. (i) is chosen; a comment names the
Taffy lines it mirrors and the `=0.15.1` pin in `Cargo.toml` means a
bump is a deliberate re-read.

## 4. CSS cases

A container is a lane when the build reads these from its used style
and its children's (`style/used.rs`), once per rebuild of the
container's own box, and records the reason it is not (`LaneDecline`,
counted in `UpdateCounts::lanes_declined`, read by tests as
`splice_decline` is). A declined container is laid out by Taffy as
today: the exact answer, slower; never a wrong one. Every case below
has a test in the slice that handles or declines it, each proven by
breaking.

| Case | First slices | Why |
|---|---|---|
| Block container, every child block-level and in flow, no child collapsing its margins through | lane (PR 3) | the rows page; the fold of section 2.1 |
| Child vertical margins collapsing between siblings | lane (PR 3) | in the fold |
| A child whose margins collapse through it (empty, no padding or border) | declined, then lane (PR 3b) | the join needs its own test |
| Container `padding`, `border`, `min-height`, `max-height`, definite `height` | lane | the container's own answer, folded once |
| Children with `%` width, margin or padding | lane | resolve against the container's inner width, which an insert does not change; per child |
| Children with `%` height | lane | resolve against the container's definite height, or auto; per child |
| `position: relative` on a child | lane | the inset is added to the child's own rect, siblings unmoved (CSS 2.2 9.4.3) |
| `position: absolute` or `fixed` child of the container | declined (PR 3); lane later | its static position is the in-flow position of its place among the siblings, derivable from the tree, not built first |
| An absolute box inside a row whose containing block is the row | lane | hoisted onto the row, in the row's frame |
| An absolute box inside a row whose containing block is above the lane | declined | hoisted past the lane: its position is from the row's start, which the containing block's `compute_oof_layout` would have to read from the tree |
| Inline or text children, `display: contents` | declined | siblings share an anonymous box, as splice (a)2 |
| `overflow: scroll` or `auto` on the container | lane | the container is a scroll frame and a lane; `Offsets::range` reads `total()` instead of every child |
| `align-content` on the container other than `normal` or `start` | declined | shifts every child by the free space |
| `direction: rtl`, `text-align`, `justify-items` | lane | cross-axis per child |
| `writing-mode` vertical | declined | vapors does not lay it out |
| Flex column, `flex-wrap: nowrap`, every item `flex-grow: 0`, `flex-shrink: 0` or the container's main size auto, `justify-content` `flex-start` or `normal` | lane (PR 3f; every flex case in section 10.3) | the sum-plus-gaps fold; `align-items` is cross-axis per item |
| Flex row (a horizontal list) | lane (PR 6, same code, axis swapped; section 10.6) | |
| Flex with grow, shrink against a definite main size, `justify-content` centring or spacing, `order`, `flex-wrap: wrap` | declined | every item's size or position depends on the total |
| Grid | declined, not planned | tracks are shared by several items; an insert moves items by cell |
| `position: sticky` | n/a | refused by `style/position.rs:31` |
| Floats, `clear` | n/a | Taffy's `float_layout` feature is off |
| Keyed `{for}` reorder (swap rows) | the lane is rebuilt from the container's children, O(n log n) | counted; after the finish line, the js-framework-benchmark swap case may want a move operation on the tree |
| A `{for}` body whose row makes several top-level boxes | lane | each box is a child; the sum is over boxes, not rows |

## 5. Interactions

### 5.1 Paint's kept order (splice B)

Unchanged: the order splice (`order_splice.rs`) decides from
`ChangedSlots` and `Stacking`, which a lane does not touch. A lane
child in flow with `Stacking::default()` is exactly a splice-able row.
What changes is downstream of the order: `moved` no longer lists the
rows after an insert, so `to_paint` reads nothing for them and
`boxes_shifted` is 0 (acceptance of PR 4). The parts' items are in
their frame's coordinates (IU4 4-ii); with a row as a frame, a row's
items are relative to the row's border box, so a row's part is byte
for byte what it was before the insert.

### 5.2 Hits

`KeptHits` keeps each hit's rect and, with #248, its frame;
`Frames::hit` puts it on the page on read. With rows as frames the
translation of a hit is its row's start, O(log n) per hit through the
lane; `Painted::hit` scans every kept hit (`kept_hits.rs:67-72`, about
13 µs at 3000 rows, property 5's note), so a scan would be O(hits log
n). Two fixes, one chosen: (i) a frame's shift is computed once per
hit test and cached by frame in `Frames` (a hash map, O(visible
frames) entries since only hits in the view can hold the point: cheap
but still O(hits)); (ii) the lane answers `between(y, y)` first, so a
hit test inside a lane tests the hits of one row: O(log n + the row's
hits), and the scan over every kept hit goes for lane children. (ii)
is the design; it also retires property 5's residual scan for lists.
PR 5.

### 5.3 a11y bounds (#243)

#243 makes a record's bounds relative to its parent node's border box,
scrolled by the parent's applied offset, so a scroll writes one
offset. An insert still changes every later sibling's `y` within the
parent, so every later row's record differs and is written: 8,683
instructions per row measured at 9b405f9 (1.1), the largest remainder.

With a lane: the mirror's elements for a lane's children are placed in
flow (`position: static`, `display: block`, the record's height and
width as the element's size, its cross-axis offset as a margin) inside
the container's element, in the lane's order, so the browser stacks
them and an insert is one DOM insert, as it is for any page. The record
gains a `bounds` variant: `AxBounds::Stacked { size, cross }` beside
the relative rect; the mirror writes `position: static` for it and the
sizes. The order of the elements is the lane's order, which the
mirror's placements already keep (`a11y_mirror_place.rs`). What the
browser then reports as the element's bounds is what Chromium's own
block layout gives for a stack of those sizes, which is the lane's
answer by section 2.1; the 5b oracle compares `DOM.getBoxModel` per
node, so a drift shows there. The touched set after an insert is the
new row and its ancestors; `--scaling Insert` judges the
`accessibility` phase as every other (`Judged::EveryPhase`) once this
lands, in place of the per-row ceiling (`INSERT_A11Y_PER_ROW`), and
`RESIZE_A11Y_PER_ROW` goes back to 13,000 from the 13,500 #289 relaxed
it to (`crates/perf/src/scaling_judged.rs`).

This is a change to `vapors-a11y`'s types and the mirror in
`vapors-platform`, the R6 owner's area; PR 5 in section 7 is theirs
to take or to hand over. Until it lands, PR 4 keeps reporting the rows
after an insert to the take as a `restacked` set (as #243's `scrolled`
set is reported but ignored by paint), so the mirror stays right and
property 3's accessibility term stays at today's 8,683 per row.

### 5.4 IU4 scroll frames (#248, 4-ii)

#248 gives each part, entry and hit a frame named by its scroll
container's slot, and `Frames` (`frames.rs`) holds every frame's shift
and clip on the page, made once per paint over the scroll containers
(`Frames::of`, O(scroll containers)). The lane uses the same naming: a
lane child's frame is its own slot, and its parent frame is the
container's. Two things in 4-ii's design change:

- `Frames::of` cannot make every frame per paint when rows are frames
  (O(rows)). `Frames::at(frame)` resolves a frame on first read and
  memoises it for the paint: the parent frame's shift plus the lane's
  `start(handle)` for a lane child, plus the rounded scroll offset for
  a scroll container. The map holds the frames read this update,
  O(visible), and is cleared per update.
- The uniform table (4-iii, a 256-byte slot per frame) holds the frames
  drawn this frame: O(visible rows), written when a drawn frame's
  shift changed. Batches already break per row on the rows page (52
  batches for 17 rows), so the bind count is unchanged.

And one thing the lane needs from 4-ii: parts in frame coordinates
relative to the frame's border box, not its unscrolled content, for a
lane child (a row does not scroll, so the two are the same for it; the
scroll container keeps 4-ii's choice, option 2 of `iu4.md` section 8).
The culling index of IU4 PR 5 (`tops` and `reach` per scroll frame)
indexes the steps directly in the frame; a lane in the frame is one
entry with its total extent, and its visible children come from
`between(view)`, O(log n + visible). PR 5 should be written with that
in mind or after PR 4 here; the lead orders them.

### 5.5 Placement's debug check and the layout check

`placement_check.rs` places every box afresh with `placed` and
compares rects; `layout_check.rs` lays the tree out afresh with Taffy
and compares each box's size and location. Both stay, both compare
materialised positions (the check asks `rect(slot)` for a lane child),
so a lane that answers wrongly fails on the update that asked, naming
the slot. The layout check's tolerance for lane children is section
2.1's.

## 6. Expected cost of an insert per design

Instructions per frame, `update` phase (the mirror's take is the
`accessibility` frame phase beside it), the rows page. "Edit" is Text's measured 123 k. The L0 row is measured (section
1.1); the L2 rows are estimates to be replaced by each PR's run.

| Design | 1000 rows | 3000 rows | 10,000 rows | Judged by `--scaling Insert` |
|---|---|---|---|---|
| L0, today (9b405f9) | 12.0 M | 35.6 M | 120 M | reported, not judged |
| L0 + the splice follow-up (remainder 3 gone) | about 2,600 per row less (1.4): 9.4 M | 28 M | 94 M | fails: layout, placement, paint still O(n − k) |
| L2 after PR 3 (layout lane, positions still written) | layout ≈ edit + log n; placement and paint still 7 (n − k) | | | passes `layout`, fails `paint` |
| L2 after PR 4 (derived positions, parts in row frames) | ≈ edit + 2 to 5 k (tree, frame reads) + O(visible) uniform writes, ≈ 130 to 140 k | the same within noise | the same within noise | passes every phase but `accessibility` |
| L2 after PR 5 (mirror in flow) | ≈ 140 k + the take of one row | the same | the same | passes |

Remove is the same path with one leaf taken out; Resize (a row's
height toggled) is a point update and the same cost; Scroll is IU4's.

## 7. Slices

Each under about 400 production lines; each check proven by breaking
what it guards in its PR; counts pinned in `sparse_updates.rs` and a
new `tests/lane_*.rs`; instruction counts at 1000, 3000 and 10,000
rows in `docs/perf/`, three runs, medians.

| PR | Content | Accepted when | Owner, order |
|---|---|---|---|
| 0 | Measurement: `--scaling Insert` reports insert and remove frames apart (two medians per phase; the judge reads the worse); a `Remove` workload is not needed. Section 1.4 filled with the per-phase split and the counts | `docs/perf/splice-layout-<date>.md` with the split; no code change beyond the harness | this plan's author; first; #270 |
| 1 | Built as 1a (layout: an insert or remove edits the owner's box children in place, `layout/child_edits.rs`, `slots_edit.rs`) and 1b (paint: the rest of this row). The splice follow-up (`docs/plans/splice.md` "As built"): `build_shallow` builds the owner's children from the kept box's children with the inserted slots put in after their previous element sibling's slot and the removed ones dropped (the children mark carries the nodes, `Marks::mark_child_inserted`), no walk over the DOM children; `Rebuilt::subtrees` keeps a count, not a clone; `plan` reads the old children by rank from the kept order; `spliced_steps` edits in place; drawn steps, hits and batches patched by the splice's edits instead of made afresh; `ClipTree::spliced` regroups the touched parents only | Insert at 3000 rows: `boxes_built` unchanged, build-phase and paint-phase instructions within an edit's except the shift (counted); `rebuild_check` and `kept_order_check` green | splice lane; before PR 3's acceptance number, not before its code |
| 2 | `layout/lane_tree.rs`: the sum tree (B-tree, fanout 16, handles, parent links): insert after, insert first, remove, set, `start`, `between`, `total`, in-order iteration; generic over a `Summary` with `combine(left, right, gap)` and `start_of` (section 10.1); `BlockSummary` (the fold of section 2.1) and `FlexSummary` (10.2), each with a property test over random edits against a sequential fold; the collapse-through join declined and tested | Unit tests; `cargo xtask lint`'s file sizes; no behaviour change | layout lane; with PR 0 |
| 3 | The block lane in layout: `LaneDecline` from the used styles at build; `LayoutBox::lane: Option<Lane>` on the container, lent with its cache; `lane_block.rs`, the per-child layout mirroring taffy `block.rs`; `dispatch` routes a lane container to it; a lane child's `Layout.location` on the axis is zero and `BoxTree::location(slot)` derives it; `written.changed` not pushed for lane children whose summary is unchanged; `settle` updates a lane ancestor in place of clearing it; the layout check's tolerance; counts `lanes`, `lanes_declined`; a decline test per row of section 4; flex containers declined as `FlexLaneNotBuilt` (10.6) | Insert at 3000 rows: `layout_calls` the new row's plus a constant, `boxes_laid_out` the same; `--scaling Insert` `layout` phase within 2% across sizes; the Chromium layout fixtures unchanged; every decline reason tested | layout lane; after 2 |
| 3f | The flex column lane (section 10): `lane_flex.rs`, the per-item layout mirroring taffy `flexbox.rs` (10.5); a decline test per row of 10.3; `FlexInsert` on `rows_list.rs` | FlexInsert at 3000 rows: `layout_calls` the new row's plus a constant; `--scaling FlexInsert` `layout` within 2% across sizes; the Chromium layout fixtures unchanged | layout lane; after 3, before 4 |
| 3b | The collapse-through join in the fold, with its test; absolute children of a lane container from the tree | the two declines of section 4 become lanes, each with a Chromium fixture; `--scaling PositionedInsert` (W6, a fixed footer under the body) judged as `Insert` in place of reported (`docs/perf/w6-positioned-2026-11-17.md`) | layout lane; any time after 3 |
| 4 | Derived positions: `Placement` leaves a lane child's axis coordinate unstored and `rect(slot)` adds the chain's starts; the walk does not enter a lane child whose summary is unchanged; `moved` empty for them; a `restacked` set reported to the a11y take until PR 5 (5.3); parts, entries and hits of a lane child in its frame (its slot), on #248's naming; `Frames::at` lazy and memoised (5.4); `between(view)` for culling and for hit tests inside a lane (5.2) | Insert at 3000 rows: `boxes_placed` the new row's boxes plus the chain to the root, `boxes_moved` 0, `boxes_shifted` 0; `--scaling Insert` every phase but `accessibility` within 2% across sizes; pixels unchanged (the Chromium paint comparisons, the kept-paint debug check with translations); hit tests on the rows page unchanged (`sparse_updates` hover) | layout lane with the paint lane for `paint*.rs`, `kept_*.rs`; after 3 and IU4 4-ii |
| 5 | The mirror in flow for lane children (5.3): `AxBounds::Stacked`, the mirror's `position: static` elements, the touched set after an insert; `--scaling Insert` judged `EveryPhase` | `accessibility` within 2% across sizes for Insert; `RESIZE_A11Y_PER_ROW` back to 13,000 (#289); the 5b oracle's bounds unchanged; `a11y_mirror_scroll.rs`-style tests for an insert | R6 owner; after 4, or in parallel on `vapors-a11y` and `vapors-platform` with the elements side stubbed |
| 6 | Flex rows (a horizontal list): 3f's summary and per-item code on the other axis; the stretched-height refusal or `lane_relaid` count of 10.6 | a horizontal-list insert's `layout` within 2% across sizes; `sparse_updates_flex` counts | layout lane; after 4 |
| 7 | Acceptance: Insert, Remove and Resize at 1000, 3000 and 10,000 rows, instruction counts, wall clock, in `docs/perf/`; property 3 marked held or not in `perf-finish-line.md`; `splice.md` (e) and `iu4.md` 8.5 closed | `--scaling Insert` and `Resize` pass `EveryPhase`; update under 1 ms at every size | last |

Order: 0, 2, 3 can start now (2 and 3 touch `crates/elements/src/
layout/` only, no file #248, #243 or the splice follow-up change); 1 is
the splice lane's; 4 waits for IU4 4-ii's frame coordinates; 5 waits
for 4 on the elements side; 3f after 3 and before 4 (section 10.6); 6 after 4; 7 last. The estimates in
section 6 assume PR 1 lands before PR 4's acceptance run.

As built, PR 3 (3a-1 #315, 3a-2 #323): the lane is not lent with its
container's cache. `put_box` drops it with the container's box, so any
rebuild of the container's own box (a class toggle or hover style on the
list itself, or a declined child edit) makes the next layout lay every
child out again: O(rows) `lane_child` calls, each a cache hit, no worse
than Taffy. The leaves' inputs are split in two (`lane_share.rs`): the
height bases change between a flex parent's size query and its layout,
and only the children whose styles read them (a percentage height,
min-height or max-height, or a relative `top` or `bottom` percentage)
are laid out again when they do.

## 8. Decisions for Tom

Decided by Tom, 2026-11-13: 1 build the lane (Taffy stays for every other
layout; the per-child lines mirrored from Taffy carry its MIT copyright
and licence notice); 2 start now in plan order, PR 0 first, with
build-time styles (U2 onward) alongside (4-ii has merged, so PR 4 waits
on nothing); 3 the a11y mirror in flow as its own PR; 4 block and flex
lanes designed together, not block first (a design note for flex columns
precedes PRs 1 to 3, so the sum tree serves both; flex cases that grow or
shrink are refused loudly and laid out by Taffy); 5 one Taffy issue after
PR 4 is measured, no PR. Also (lead, 2026-11-13): the extents index
(IU4 PR 5) must update incrementally in the lane, not be rebuilt
(docs/plans/iu4.md). The options below are kept as the record.

1. **Build the layout lane, or hold property 3 at "O(rows after) in
   layout, bounded on the GPU side".** The measured floor is 12,044
   instructions per row in the update and 21,307 per row in the frame:
   at 3000 rows an insert's update is 290 edits, 975 at 10,000. Recommendation: build it; property 3 is stated as "about
   the cost of an edit" and nothing short of the lane reaches it.
2. **Order against IU4 and property 7.** PRs 0, 2 and 3 are
   independent of IU4 4-ii and the build-time analysis; PR 4 needs
   4-ii. Recommendation: 0, 2, 3 now in the layout lane; 4 as soon as
   4-ii merges; the build-time analysis unaffected.
3. **The mirror in flow (PR 5).** It is the largest term and in the R6
   owner's crates. Recommendation: yes, as its own PR on
   `vapors-a11y` and `vapors-platform`, specified by section 5.3; the
   elements side of it is small.
4. **Block lanes first, flex lanes second (PR 6).** The rows page is a
   block container; real pages list in flex columns too (`rows_list.
   rs`). Recommendation: block first, since the rows page is the
   finish line's workload, flex within two PRs after.
5. **Upstream.** An incremental block container is a Taffy feature.
   Recommendation: an issue on Taffy describing the lane once PR 4
   measures, no PR (Tom, 2026-11-01); vapors keeps its own.

Implementation choices recorded here, not for Tom: the sum tree's
shape (B-tree, fanout 16; a chunked vector if it does not fit a file),
vendored per-child code over a one-child Taffy call (section 3), f64
sums with a declared tolerance in the debug check (section 2.1),
handles not indices (section 2 (b)).

## 9. Open points and risks

1. The per-child code mirrors about 120 lines of taffy `block.rs`.
   Drift is caught by the debug layout check on every test update and
   by the Chromium fixtures, not by the compiler; a Taffy bump is a
   re-read, noted at the pin.
2. `Layout.location` for a lane child is zero on the axis, and any
   reader of `layout.location` that is not placement reads a wrong
   value silently. The readers today: `placement_walk.rs:150-151`,
   `layout/mod.rs:148` (`placed`, the debug check) and
   `scroll.rs:170-171` (`Offsets::range`); PR 3 changes all three and
   a debug assertion in `set_unrounded_layout` refuses a non-zero axis
   location written to a lane child by anything but the lane.
3. `between(view)` replaces the test of every item for lane children;
   an item that extends outside its row's border box (a shadow, an
   outline, overflowing text) is in the row's frame and would be culled
   with the row. The row's summary carries its overflow extent
   (section 2 (b)), and `between` widens the interval by the largest
   overflow in the lane (a fold: max), so nothing visible is skipped.
   The debug build compares the candidates with a test of every item,
   as IU4 PR 5 does.
4. A nested lane (a tree view) is O(depth · log n) per read; depth is
   the nesting of lists, not rows. Not measured; a `NestedInsert`
   workload is worth adding if touche's layer list nests.
5. The a11y oracle (5b) is not built; PR 5's bounds are checked by
   `DOM.getBoxModel` once it is. Until then PR 5 is checked by the
   fake DOM's counts and by vapors' own bounds.
6. `--scaling Insert`'s median over alternating kinds hid the
   costlier kind (section 1.1); PR 0 (#270) reports each kind apart,
   with the split in `docs/perf/splice-layout-2026-11-13.md`.
## 10. Block and flex in one lane

Tom's decision 4 (2026-11-13): block and flex lanes designed together,
so that the sum tree and derived positions serve both, with the flex
cases that grow or shrink refused loudly and laid out by Taffy. This
section is that design; it changes PRs 2, 3 and 6 of section 7 (10.6).
Taffy line numbers are from the 0.15.1 pinned in `Cargo.toml`
(`compute/flexbox.rs`, `compute/block.rs`).

### 10.1 What the two share

Everything outside the per-child layout and the fold:

- **The tree.** One `LaneTree<S: Summary>` (`lane_tree.rs`, section 2
  (b)): handles, parent links, fanout 16, `start`, `between`, `total`,
  in-order iteration. `Summary` has `combine(left, right, gap) -> Self`
  with `Default` as its identity; the tree holds the container's main-axis gap as one
  `f64` and passes it to every `combine`, 0 for a block container
  (`row-gap` does not apply to block layout). A change of gap is a
  change to the container's style, which rebuilds the container's box
  and with it the tree.
- **Derived positions.** `start(handle)` is the child's border-box
  offset along the axis in the container's content box. The tree folds
  the leaves to the left of the handle (O(log n)) and calls
  `S::start_of(left, leaf, gap)`, which each summary defines (10.2).
  Placement, `Frames::at`, hits, culling and the extents index (5.2 to
  5.4) read `start` and `between` and never ask which algorithm made
  them.
- **The cross-axis position**, stored per child as today: from the
  container's inner cross size and the child's own size, margins and
  alignment, never from its siblings.
- **The container's answer.** `LaneAnswer` from the root's summary: the
  content size along the axis, the first baseline, the min- and
  max-content cross contributions (for the container's intrinsic
  width), and the scrollable reach along the axis. Block adds the
  collapsible margin sets the container passes to its parent (10.4).
  Each algorithm turns its root summary into one `LaneAnswer`, and the
  container's `LayoutOutput` is built from that in one place.
- **`settle`** updates a lane ancestor in place of clearing it (section
  2 (d)), for either kind of lane.
- **`LaneDecline`**: one enum, one variant per refused case of either
  algorithm (10.3 and section 4), counted in
  `UpdateCounts::lanes_declined` by reason. A declined container is
  laid out by Taffy, as today.
- **The extents index.** A lane in a scroll frame is one entry in its
  frame's index (IU4 PR 5, `docs/plans/iu4.md` section 4) holding the
  lane's total extent and the largest overflow reach of its children;
  the lane's visible children come from `between(view)`. An insert or
  remove in the lane updates that one entry (its extent) and the tree;
  the index is never rebuilt for a lane edit. Lead's requirement,
  2026-11-13; built in PR 4.

### 10.2 What differs: the summary and the per-child layout

| | Block container | Flex column |
|---|---|---|
| Leaf summary | top margin set, border-box height, bottom margin set, collapses through, first baseline, overflow reach, min/max-content width | margin-box height, item count (1), base-size contribution, content contribution, first baseline, overflow reach, min/max-content width |
| `combine` | section 2.1: heights added with the collapsed margin between L's bottom set and R's top set | heights added, plus one gap when both sides are non-empty; contributions added; counts added |
| `start_of(left, leaf)` | `left`'s height, plus the margin collapsed from `left`'s bottom set and the leaf's top set; for the first child, its top margin unless it collapses with the container's (10.4) | `left`'s margin-box height, plus one gap per child in `left`, plus the leaf's top margin |
| Content size along the axis | the root's height and the outer margins kept inside the container (10.4) | the definite-space branch: the root's base-size contribution plus gaps; the min- or max-content branch: the root's content contribution plus gaps (taffy `determine_container_main_size`, 1420-1560); either clamped by the container's min and max height |
| Per-child layout | the non-float, same-BFC branch of `perform_final_layout_on_in_flow_children` (section 2 (a)) | the per-item steps of `compute_preliminary` for one item (10.5) |

The reach and baseline folds are the same shape for both: the first
baseline is the left side's if it has one, else the right side's moved
by the left side's extent; the reach is the larger of the left side's
reach and the right side's moved by the left side's extent (a max-plus
fold, associative).

The flex leaf carries two contributions because Taffy sizes a column
container whose height is auto in two ways: with definite available
height, the sum of each item's `max(flex_basis, min_height)` plus its
margins, floored at its padding and border; with min- or max-content
available height, the sum of each item's content contribution. Both
are per item, so both are sums; the container picks the branch from
its inputs as Taffy does.

### 10.3 Flex cases

The flex lane is a flex column whose items' sizes and positions do not
depend on the total. Taffy lays out every case the lane refuses.

| Case | Lane | Why |
|---|---|---|
| `flex-direction: column`, `flex-wrap: nowrap` | lane | one line; positions are a sum |
| Every item `flex-grow: 0` | lane | the default; with grow above 0, positive free space (a definite height, a `min-height`, an item's max below its basis) is shared by every item |
| Any item `flex-grow` above 0 | declined | as above |
| Any height, items no taller in total than the column's inner height | lane | no item shrinks, and with grow 0 nothing grows, so an item keeps its hypothetical height, except as the next row says |
| An item that can shrink whose hypothetical height lies outside its own minimum and maximum (a floor from its aspect ratio or its padding and border above its `max-height`) | declined (`ClampedHeight`) | unless the column is exactly sized, Taffy leaves such an item unfrozen and clamps it to its resolved minimum and maximum (`resolve_flexible_lengths`, step d), so its height depends on the total (review of #342, H1) |
| Items taller in total than the column's inner height, none that can shrink | lane | an item shrinks only when its `flex-shrink` is above 0, its basis is not below its hypothetical height and its minimum height is below it (`resolve_flexible_lengths`); auto-height rows of content have a minimum equal to their content. As built (3f-1): a leaf carries `shrinkable` (an OR fold), and the column's answer compares the sum with the inner height |
| Items taller in total than the column's inner height, one that can shrink | declined at layout (`Shrinks`) | its height depends on the total. As first planned, an auto-height column could not reach this: it can, when an item's automatic minimum exceeds its basis share (`flex-basis` below its content), so the same check covers every height. This takes in PR 3s (10.6) |
| `justify-content` `normal`, `flex-start`, `start`, `stretch` | lane | items packed from the start |
| `justify-content` `flex-end`, `end`, `center`, `space-*` | declined | every item is moved by the free space |
| `row-gap`, a length, or a percentage with the column's height definite | lane | the gap is the container's |
| `row-gap`, a percentage or a `calc()` with the column's height auto | declined (`RelativeGap`) | Taffy resolves it to 0 to size the column, then again against the height the items sum to (`compute_preliminary` 428-438), which depends on the total |
| Main-axis `auto` margins on an item (`margin-top` or `margin-bottom: auto`) | declined | they take a share of the free space |
| `align-items` or `align-self` `start`, `end`, `center`, `flex-start`, `flex-end`, `stretch`; cross-axis `auto` margins | lane | the cross position is per item, against the container's inner width |
| `align-self: baseline` | lane | in a column an item aligned by baseline is placed by itself (Taffy reads line baselines only in rows, `calculate_children_base_lines` 2054-2056); in a row (PR 6) it depends on every item |
| `flex-basis` (any), item `min-height`, `max-height`, `aspect-ratio`, `%` sizes | lane | per item; a `%` resolves against the container's inner size only when that size is definite and not derived from the container's content (taffy `generate_anonymous_flex_items`, 783-789), a property of the container |
| `position: relative` on an item | lane | the inset moves that item only |
| Container `padding`, `border`, `min-height`, `overflow: scroll` or `auto` (with height auto) | lane | the container's own answer |
| Container width not definite (the column is itself a row item or shrink-to-fit) with stretched items | lane, with a full relayout when the widest item changes | items stretch to the container's width, which is the widest item's contribution (a max fold). An insert that changes the max lays every item out again: correct and O(n), counted as `lane_relaid`; one that does not is O(log n). Block containers in shrink-to-fit width behave the same |
| `flex-direction: column-reverse` | declined | positions run from the container's end, so every item moves when the total changes |
| `flex-direction: row`, `row-reverse` | declined until PR 6 | 10.6 |
| `flex-wrap: wrap`, `wrap-reverse` | declined | lines are formed from the running total |
| Text or inline children | lane | each is an item of its own in vapors' boxes, laid out as Taffy lays it out (a page with a loose text node and a `display: inline` div matched Taffy, review of #342) |
| `position: absolute` children | declined, as block (section 4) | their static position is from the container's alignment |
| `order` | n/a | not a property vapors reads (`crates/css/src/property/longhands.rs`); Taffy keeps tree order |

As built, PR 3f is two PRs, as PR 3 was: 3f-1, the column mirror
(`lane_flex*.rs`) compared with Taffy on pages of each case and its
declines, with no change to layout; 3f-2, the flex lane live
(`FlexLaneNotBuilt` retired) with `FlexInsert`. One input differs from
Taffy's, declared where it is made (`lane_flex_child.rs`,
`available_along`): for a column whose height is the items' total, an
item is asked with its own hypothetical height as the space along the
column, since one item does not read the total; its height is known in
every ask that passes it, so neither Taffy's cache nor the item reads
that space.

As built, 3f-2: one `Lane<K>` serves both kinds (`lane_kind.rs`: the
summary, the settled container, laying one child out, where starts are
measured from, the answer), `Lanes` holding a map per kind. A column's
leaves must serve every ask of it, and a parent's size queries ask it
under max-content, min-content and definite space along in turn, so
the kind of space is not part of a column's share: a leaf holds the
item's share under definite space and under min- or max-content
space, and an item's height is measured under both min- and
max-content space, the column declined at layout
(`SpaceDependentHeight`) where the two differ. Block and text items
measure the same under both. As for blocks, the column's height inputs
(its height, inner height and whether it is definite) are apart from
its share, and only items whose styles read them are laid out again
when they change. The in-crate comparisons (`lane_compare.rs`) lay
their pages out with lanes off (`KeptBoxes::lanes_off`, test builds
only), so a mirror is compared with Taffy's answers and not a lane's.

Known O(rows) case (review of #343, M1; `tests/lane_flex_heights.rs`
pins it at 366 calls for an insert at 50 rows and 3516 at 500):
touche's shell one level deeper, a definite-height column that is an
item of a flex column, its rows reading its height (`min-height: 10%`).
The parent asks the column for its content size, an ask that drops its
styled height, so its height inputs flip between that ask and the
layout, and every reading row is laid out again on each. The fix is a
leaf set per height inputs (two trees, edits applied to both, each
filled from the stale slots it has not yet seen); after PR 4, as rows
that read their column's height are the less common list.

### 10.4 Margins

Block: margins collapse between siblings by the fold of section 2.1,
and a child that collapses through is declined until PR 3b. The
container's own top margin collapses with its first child's when the
container has no top padding or border and does not establish a block
formatting context (taffy `block.rs`, `own_margins_collapse_with_
children`), and the same at the bottom. The root summary's top and
bottom margin sets are then the container's collapsible margin sets in
its `LayoutOutput`, and the first child's `start_of` leaves its top
margin out; otherwise the first child's top margin is inside the
container. Both cases are in PR 3's tests.

Flex: margins never collapse. A flex container establishes a flex
formatting context, so its margins do not collapse with its items',
and items' margins do not collapse with each other's. An item's leaf
height is its margin-top, border-box height and margin-bottom.

In both, percentage margins and paddings resolve against the
container's inner width (taffy `generate_anonymous_flex_items`,
800-806, and `block.rs`'s `generate_item_list`), which an insert does
not change; a width change marks every child, as in section 2 (a).

### 10.5 The flex per-item layout

Mirrored from taffy `compute/flexbox.rs` for one item, in the order
`compute_preliminary` runs them, with the multi-item parts left out:

1. The item's fields as `generate_anonymous_flex_items` (777-895) makes
   them: sizes, min and max resolved against the percentage basis
   above, margins, padding, border, `align-self`, `flex-basis`.
2. Its flex base size and hypothetical main size
   (`determine_flex_base_size`, 947-1231), with the container's
   available space from `determine_available_space` (896-946),
   computed once per container layout.
3. Its target main size: the hypothetical main size. Grow is 0, a
   column whose items would shrink is declined (`Shrinks`), and an item
   that can shrink and whose hypothetical size lies outside its own
   minimum and maximum is declined (`ClampedHeight`): otherwise Taffy's
   `resolve_flexible_lengths` (step d, 1898-1915) clamps an unfrozen
   item whenever the column is not exactly sized.
4. Its hypothetical cross size (`determine_hypothetical_cross_size`,
   1945) and its used cross size, stretched to the container's inner
   width when `is_stretched` (`determine_used_cross_size`, 2228-2281).
   With one line and a definite container width the line's cross size
   is that width (`calculate_cross_size`, 2128).
5. Its cross-axis offset (`align_flex_items_along_cross_axis`, 2410,
   and `resolve_cross_axis_auto_margins`, 2355, for one item).
6. Its final layout (`calculate_flex_item`, 2557): `perform_child_
   layout` with the target sizes, and its baseline and overflow for
   the leaf.

The mirrored lines carry Taffy's MIT copyright and licence notice and
name the functions and lines they mirror, as `lane_block.rs` does for
`block.rs` (decision 1).

### 10.6 Effect on section 7

- **PR 2** builds `LaneTree<S>` with both summaries, `BlockSummary` and
  `FlexSummary`, each with a property test: random inserts, removes and
  size changes fold to what a sequential pass over the children gives
  (Taffy's running sum for block margins; the sum plus gaps for flex).
- **PR 3** is the block lane and `LaneDecline` with every reason of
  section 4 and 10.3; flex containers are declined in PR 3 with the
  reason `FlexLaneNotBuilt`, so the count says what is not yet a lane.
- **PR 3f** (new, after 3, before 4): the flex column lane, `lane_flex.
  rs` (10.5), its declines each tested, and `FlexInsert` on the
  flex-column list (`rows_list.rs`). Accepted as PR 3: `layout_calls`
  the new row's plus a constant, `--scaling FlexInsert` `layout` within
  2% across sizes. PR 4's derived positions then land on both kinds of
  lane at once.
- **PR 6** becomes flex rows (a horizontal list): the same summary and
  per-item code on the other axis. Its extra refusal: a container whose
  height is auto with stretched items, since every item's height is
  the tallest item's (the max fold), so an insert of a taller item lays
  out every item; accepted with `lane_relaid` counted, or declined if
  the measurement shows the relayout common.
- **PR 3s** (next after 3f, before 4; lead, 2026-11-13: scrolling
  lists are a real-app case): the scrolling flex list with
  shrinking items (10.3). An item shrinks only when the total exceeds
  the container's height and the item's minimum main size is below
  its hypothetical size, which is per item. The leaf would carry
  "can shrink" (an OR fold), and a lane whose total exceeds its
  definite height and whose fold says some item can shrink hands that
  layout to Taffy, counted as a runtime decline. Rows with `overflow:
  visible` and an auto height have a minimum equal to their content,
  so in the usual list nothing can shrink and the lane holds.

## 11. Lane frames: PR 4's design

PR 4 (section 7) makes a lane child's position derived: an insert
moves no later child's stored position, and nothing below a later
child is placed, painted, shifted or read for it again. This section is
how, after reading the paint side as it is (`frames.rs`,
`kept_clips.rs`, `frames_update.rs`, `kept_batches.rs`,
`frame_extents.rs`, `placement_walk.rs`, `a11y_mirror_state.rs`).

### 11.1 What a lane frame is

Paint already draws every part in its frame's coordinates and adds the
frame's shift on read: scene items (`Frames::item`), hits
(`Frames::hit`), border boxes and origins (`Frames::rect`, `point`),
batches (`frames_draw.rs`, the shift as a uniform). Today a frame is a
scroll container's content (IU4). A **lane frame** is a lane child: its
own box and everything placed below it are in its coordinates, and its
shift is its scroll frame's shift plus its start along the axis from
the lane's tree. An insert changes the later children's starts, so
their frames' shifts, and nothing in them.

Lane frames are lighter than scroll frames:
- A lane frame clips nothing: its clip is its scroll frame's.
- It is not in `Frames::of`'s list or the nested-scroller index
  (`frames_nested.rs`): those are per scroll container and made per
  paint, O(rows) if rows were in them. A lane frame is resolved on
  first read and memoised for the paint (section 5.4's `Frames::at`):
  O(frames read), the rows drawn and hit.
- Its shift is whole device pixels, as a scroll frame's is (snapped
  items keep their bits only under a whole shift). The shift past its
  scroll frame's is the container's content origin in that frame, the
  lane's top (the inset, less an escaping margin) and the child's
  start, so a lane's children are frames only when the origin and every
  start are whole at the scale. That is decided per lane, by the
  placement walk when it places the container: the lane keeps the
  children whose margin box or margins are fractional and those with a
  scroll container below (sets, updated as leaves are put), and the
  origin is the container's placed rect. A lane that is not framed
  keeps today's placement for its children (correct, O(rows after)),
  counted as a decline, `LaneFramePixel`; a lane whose framing flips
  has its children placed and their clips redone, O(children), rare.
- A lane with a scroll container inside one of its children is framed
  too (4e; until 4e it was a counted decline, `LaneFrameScroller`).
  Real pages put scrollers in rows: the touche shell does, and so does
  the rows page `--scaling Insert` measures (each row's `.value` hides
  its overflow). A scroll frame's shift was worked out from the frame
  it is in when frames were made, and a lane frame is resolved later,
  so a scroll frame inside a row would hold a shift that an insert
  above it makes stale. From 4e a scroll frame whose outer frame is a
  lane frame, or a scroll frame under one, keeps only what is its own
  (section 11.2, "Scroll frames under a lane frame").
- A lane whose child is itself a scroll container is not framed: a
  counted decline (`LaneFrameScrollingChild`) of that lane alone, its
  children keeping today's placement (correct, O(rows after)). Frames
  are named by slot, so that child's lane frame (its own box) and its
  scroll frame (its content) would share a name, and composing the
  scroll frame would read itself (the gpu nested-scrollers page, whose
  rows are scrollers, overflowed the stack). A scroller inside a child
  is framed. Follow-up, which retires the decline: lane frames named
  in a key space of their own (a lane frame's name the child's slot
  with a tag that no scroll container's slot carries), everywhere a
  frame is named: a part's frame (`BoxFrom::frame`), the hits and the
  extents index by frame, `Frames`' maps and the placement's framed
  slots.

### 11.2 What each side does

- **Layout** (4c): a lane child's `Layout.location` along the axis
  holds its relative offset only; the lane writes a child's layout when
  the child was laid out, not when its start moved
  (`lane_layout.rs` `write_starts` goes). The debug layout check adds
  the start to a lane child's location before comparing with Taffy.
- **Clips** (4b): `Clips::frame_inside` names a lane child's slot as
  the frame of its own box and of everything below it; a lane child's
  own frame is itself (a scroll container's own box is in its parent's
  frame; a row's moves with it).
- **Placement** (4b): a lane child's rect is in its own frame, its
  start left out, so after an insert the later children's rects are
  unchanged and the walk does not enter them: `boxes_moved` and
  `boxes_shifted` 0. Until 4c removes the writes, the walk still visits
  them (their layouts were written) and finds nothing changed.
- **Frames** (4b): a lane frame is resolved from the lane's
  `start(handle)`, the container's placed origin and its scroll frame's
  `FrameAt`, when it is read. Frames are read at draw too, after the
  update (scene items, batches), so resolving lazily needs the lanes
  then: each read of a frame takes a resolver holding the frames, the
  lanes (`Placed::lanes`) and the placement, memoising per draw. Making
  every lane frame when frames are made instead would be O(rows after)
  again, since an insert changes every later row's shift.
- **Draw** (4b): each lane frame drawn is a batch frame with its shift
  in the uniform table (4-iii): O(rows drawn); batches already break per
  row on the rows page.
- **Culling** (4b, 4d): a framed lane's children are frames of their
  own, so its scroll frame's extents index (`frame_extents.rs`) holds
  the container's steps alone, not its rows'; the children that can
  show come from `between(view)`, widened by the most ink seen past
  them (`paint_lane_ink.rs`), and each child's steps are indexed in its
  own frame. What 4d adds is the splice patch below: the per-row frame
  indexes are not made afresh on an insert.
- **Splice patch** (4d): an insert or remove of a lane child reshapes
  paint order, and the drawing steps, the extents index and the hits
  were made afresh after a splice (`paint_partial.rs` `drawn_updated`),
  O(steps) whatever the lane. Each framed child is a frame of its own,
  so its steps and entries move no other child's: the splice patches
  the drawing steps, the extents index and the hits by its edits, the
  new child's steps added and the removed child's taken out, O(that
  child's steps).
- **Scroll frames under a lane frame** (4e): a scroll frame whose
  outer frame is a lane frame, or such a scroll frame, keeps its own
  part only: its scroll shift and the clip it gives its content, in its
  outer frame's coordinates (`Clips::inner`, which already is). Its
  shift and clip on the page are composed when it is read: the outer
  frame's, resolved (a lane frame's from its lane, section 11.1), plus
  its own, memoised with the lane frames until the next update, as
  `Frames::at` does for them. Made and updated as today
  (`frames_update.rs`), only the stored part is relative, so an insert
  above the row changes no stored frame, and a scroll inside the row
  updates that frame and the ones nested in it alone. Each frame's
  index of the scroll containers in it (`frames_nested.rs`) is in that
  frame's coordinates, so a row's index is untouched by an insert and
  culling finds a row's scrollers through the row's frame. The frames
  are told which frames are lane frames when they are made
  (`FrameInputs`), as placement decided them, so making a frame does
  not resolve a lane frame. The scroller sets in `FrameFacts`, the
  isolated owners' noting of them (`note_scrollers`) and the
  `LaneFrameScroller` decline go.
- **The memo across a scroll** (4e): a scroll is an update for the
  frames. `Document::scroll_by`, `scroll_node_by` and `scroll_to`
  change the offsets and note a change, nothing else; the frames move
  only at the next update, in `paint_frames::updated`, which works the
  scrolled frame and the frames nested in it out again
  (`kept_updated`) and then sets the lanes (`Frames::set_lanes`), which
  drops every memoised frame, lane frames and composed scroll frames
  alike. No frame is read between a scroll and that update except
  through the last update's frames, which show the last update's
  offsets, as the scene does. Tested: scroll a container inside a
  framed row, update, and every frame read, the nested one included
  (its clip and its shift), equals the same frame made afresh.
- **Cost** (4e): composing is done on read, once a frame an update,
  for the frames read (those culling, the draw, hits and the a11y take
  reach), so a frame that is never read costs nothing and an update
  adds no per-row work for frames that do not change: an insert above
  a row stores nothing new for the scroll frames in later rows.
- **Batching across lane frames** (4f). Today a batch holds items of
  one frame, and the painter moves the whole batch by that frame's
  shift, a uniform per draw (`painter_runs.rs`, `view.shift` in each
  pipeline's shader). Every framed row is a frame, so the rows page's
  idle frame draws each row's label and button text apart: 51 text
  draws for 17 rows in view where one frame drew 35 (`gpu_rows_draws`,
  relaxed in 4e until this). From 4f:
  - A run of consecutive steps whose frames are lane frames of one lane
    batches together, as it would in one frame. The batch names the
    lane's base frame (the frame the container's content is in, whose
    shift and clip the painter applies as today), and each item keeps
    its row's offset in the lane apart from its positions.
  - Each instance of a batch carries a small index into a table of the
    batch's row offsets, device pixels, whole; the shader adds
    `view.shift + offsets[index]`. The instances are retained as today
    (their positions are in the row's frame, which an insert does not
    move); an insert above changes only the tables of the batches
    drawn, O(rows shown), never a retained instance.
  - Items still batch only under one clip, and each is culled in its
    own row's view, as now. A clip inside a row is batched moved into
    the lane template's coordinates (by the row's offset), so the
    painter's scissor, the template's shift plus the batch's clip, is
    the row's clip, and equal clips in different rows are different
    batches (#407's review, H1).
  - Every pipeline with positions takes the index: quads, shadows and
    texts. Images and surfaces, drawn an item at a time, are batched
    across rows too, each drawn at the batch's shift plus its row's
    offset; a culled surface keeps its place and its row.
  - **Index stability.** A batch's indexes are given when the batch is
    made, in the order its rows first appear, and a kept batch keeps
    them: its table's values change with its rows' offsets, its
    instances never. A row inserted inside a batch's run changes that
    batch's items, so the batch is made again (`KeptBatches`, as for
    any change to a batch's run), its indexes given afresh and its
    instances written: that batch's instances only, O(rows shown).
    Every other batch keeps its instances and indexes.
  - **Table capacity.** The table rides in the batch's uniform, 128
    rows (two offsets to each `vec4<f32>`, 1 KiB), well inside
    WebGPU's guaranteed 64 KiB uniform binding and above the rows in
    view on the screens measured (17 on the rows page, about 46 at the
    Pixel's 915 CSS px). A run over more rows than that is split into
    batches of at most 128 rows each, never truncated or wrapped; a
    test pins the split at the 129th row.
  - **Rounding.** No offset is rounded. A lane frame's shift today is
    its base frame's shift plus the child's start times the scale
    (`lane_child_at`), and framing requires that start to be whole
    device pixels (`LaneFramePixel` declines otherwise), so it is an
    exact integer. A row's offset is that same term, computed by the
    same function, so an item lands on the pixel it lands on today.
    The debug check that compares the kept batches put on the page by
    their frames with the scene batched on the page
    (`kept_batches_check.rs` `assert_same`) adds each instance's row
    offset, item by item. A test at scale 2.75 (the Pixel's) runs it
    with rows whose height is whole in device pixels (framed) and with
    rows 20.5 px tall (56.375 device px, declined), and asserts both
    draw as the scene batched on the page.
  - Accepted when `gpu_rows_draws` is back at 35 text draws (the pin
    4e relaxed), `draw` within 2% across sizes for `--scaling Insert`
    and `FlexInsert`, the kept-batches check (`kept_batches_check.rs`)
    and the Chromium paint comparisons unchanged.
- **The lane as one unit in the search and the sorted steps** (4f-2b,
  after #407's measurement). At 4f-2, draw at 1000 rows is 1.344 M
  instructions a frame for Insert against 1.036 M on main, and 223 k
  for FlexInsert against 156 k. The draw calls are main's (52 on
  Insert). The excess is the `batches` phase. Temporary sub-spans
  split it, for Insert and then FlexInsert, into:
  - the sort, `KeptBatches::of`: 98 k and 51 k;
  - making the batches, `framed_batches`: 138 k and 28 k;
  - the keys: 35 k and 3 k.

  Main's whole `batches` phase is 95 k and 15.5 k. Two causes:
  - Each row is a frame of its own, and each row frame is searched on
    its own: `Frames::meeting` lists every row frame in view (one
    `between` a lane), then the sort reads each row's frame
    (`view_in`), queries the row's index (`candidates`), and tests each
    step (`hidden_items`, another `view_in`). Batching reads each row's
    frame again. A row frame not primed costs one fold of the lane's
    tree a read.
  - An insert splices the lane, and a splice unsorts the kept steps
    (`paint_partial_drawn.rs` `batches_unsorted`), so every insert
    frame sorts every frame in view afresh.

  4f-2b treats a lane as one unit, as batching already does:
  - **One search a lane.** `Frames::meeting` lists a framed lane once,
    not its rows. The sort finds the lane's rows meeting the view in one
    walk of its tree:
    - `between` finds the first row;
    - sequential starts follow to the view's bottom, as
      `prime_started` does, with no fold a row;
    - each row's frame comes from those starts (the template plus the
      row's start, `lane_child_at`).

    Each row's steps are its frame index's candidates in its view,
    tested item by item as now. The rows found, each with its frame, are
    kept with the sorted steps. `hidden_items` and `framed_batches` read
    them, so no row frame is folded in a draw. The cost is
    O(log rows + rows shown + steps shown) a lane, and a patch adds a
    pass over the shown steps.
  - **The batches made with less work a step and a batch** (first, as
    measured after the lane search: making the batches is 139 k of
    Insert's 246 k `batches` phase). Each step's items are read once,
    not once to resolve its frame and again to batch it. A batch in one
    frame allocates no rows. A batch over a lane's rows numbers its rows
    without a hash map a batch: a lane batch holds at most
    `ROWS_PER_BATCH` rows, so a small linear or slot-indexed table
    serves. Measured against main after this step.
  - **A splice that only moves a lane's rows patches the steps, at the
    view's edges.** The lane search above keeps, for each lane, the rows
    it found meeting the view and each shown step's row. A splice whose
    edits are rows put in or taken out of framed lanes, and starts moved
    within them, marks those lanes for the next draw rather than
    unsorting:
    - the spliced steps are taken out of the shown steps, or put in when
      they show;
    - each marked lane is searched again (one walk);
    - a row found both times, and wholly inside the view both times,
      keeps its shown steps;
    - a row that left drops its steps, and a row that entered, or that
      meets a view edge either time, is tested step by step.

    The patch is O(log rows + rows at the edges + spliced steps) a lane,
    not O(rows in view). Steps of other frames stay sorted. A scroll, a
    frame's clip changing, paint order walked or frames made afresh
    still unsort.
  - The debug check that compares the shown steps with testing every
    step (`kept_batches_check.rs` `assert_candidates`) runs after every
    draw, as now. It covers both the lane search and the patch. A test
    splices while rows enter and leave at both edges.
  - Measured as `vapors-perf 60 10 --scaling Insert`, `FlexInsert` and
    `Idle`, each against main at the same settings. Insert's `encode`,
    62 k over main, and its unattributed draw, 70 k over main, are
    broken down in the same pull request.
  - Accepted when `draw` is no worse than main on Insert, FlexInsert
    and Idle at 1000, 3000 and 10000 rows. The kept-batches checks and
    the Chromium paint comparisons must be unchanged. A patched sort
    must give the steps a full sort gives (the check above), and a test
    must splice a row above, inside and below the view.
  - Order: the lane search, then the batches made with less work, then
    the patch at the edges, each measured against main. If the pull
    request passes about 600 lines, the patch goes in 4f-2c, stacked on
    it.
- **The a11y touch of started rows** (4f, or 4g if it is large). An
  insert starts every later row of its lane elsewhere, and
  `AxMirror::touch_slots` expands each started lane into its rows and
  puts each row's node in the touched set, about 100 instructions a
  row in the `update` phase, outside the accessibility phase that
  holds the a11y ceiling. The take then reads those rows (their
  records' bounds are from the parent node, section 5.3). From 4f the
  touch keeps each started lane as its container and first row, and
  the take walks those rows itself when it reads (the walk
  `prime_started` already does), so the update adds no per-row work.
  Accepted when `update` and `layout` hold no per-row term from the
  touch for `--scaling Insert` and `FlexInsert`, and `accessibility`
  stays within its ceiling and no more per row than the base; measured
  before it lands, since the take's own walk moves cost into the
  accessibility phase, where FlexInsert has about 1.3% of headroom.
- **Hits** (4d): the kept hits are by frame; a hit test asks the
  frames that can show at its point (`Frames::meeting`, which finds a
  lane's rows through `between`) and tests their hits alone, the last
  in paint order winning (`kept_hits.rs`).
- **a11y** (4b): the take is told of the lane children whose start
  changed (a `started` list from the lane, which is O(rows after) but
  only slots), so the mirror's records stay right; section 5.3's
  mirror in flow (PR 5) removes that term. Each started child's bounds
  read its lane frame, one fold of the lane each (4b); from 4d the take
  first resolves the started children's frames in one pass along each
  lane, sequential starts filling the frames' memo, O(1) a child.

### 11.3 Slices

| Slice | Content | Accepted when |
|---|---|---|
| 4a | This note (approved by the lead, 2026-11-14; ships with 4b) | reviewed |
| 4b | Lane frames: clips, placement, `Frames::at`, draw, the a11y `started` list, the two declines | Insert at 3000 rows: `boxes_moved` the containers that grow (the list and body) and no row, the same at every size, `boxes_shifted` 0, `boxes_placed` the new row's boxes and their ancestors plus the later rows visited (until 4c); the Chromium paint comparisons and the kept-paint check unchanged |
| 4c | Layout leaves starts out of later children's layouts; the layout check derives them | the layout check derives every start |
| 4d | Culling and hits through `between`; the lane's extents entry | hits through `between` match the scan |
| 4e | Scroll frames under a lane frame composed on read; the `LaneFrameScroller` decline goes | the rows page's list framed (`lanes_framed` above 0 for `--scaling Insert`); for `--scaling Insert`, `layout` and `paint` within 2% across sizes (the 3000- and 10000-row totals each within 2% of the 1000-row total), and `accessibility` within `INSERT_A11Y_PER_ROW` (9600) and no more per row than the base; the Chromium paint comparisons, the frames, placement and kept-paint checks unchanged with a scroller in a framed row; the scroll test above |
| 4f | Batching across lane frames (a row offset table per batch); the a11y touch keeps started lanes, not their rows | `gpu_rows_draws` at 35 text draws again; `draw` within 2% across sizes and `update` with no per-row term from the touch, for `--scaling Insert` and `FlexInsert`; `accessibility` within its ceiling and no more per row than the base; the kept-batches check and the Chromium paint comparisons unchanged |
| 4f-2b | The lane as one unit: one search a lane; the batches made with less work a step and a batch; the steps patched at the view's edges on a splice that only moves a lane's rows (4f-2c if past about 600 lines) | `draw` no worse than main for `--scaling Insert`, `FlexInsert` and `Idle` at every size; `assert_candidates` and the Chromium paint comparisons unchanged; a splice above, inside and below the view, and one with rows entering and leaving at both edges, tested |

4b, 4c and 4d are separate stacked pull requests, each reviewed alone,
then folded into 4d and gated once, so main never carries 4b's added
per-row cost alone. Measured on 4b alone (instructions per extra row,
1000 to 3000 rows, base to 4b): Insert accessibility 9406 to 9849,
layout 964 to 1632, paint 11,883 to 12,319; FlexInsert accessibility
1770 to 3358, layout 605 to 945, paint 2818 to 3297. Each lane frame
read by the take folds the lane once (O(B log n)); an insert's
reshaped walk makes the extents index afresh, now one frame index per
row; and the start writes stay until 4c.

The folded 4b+4c+4d is accepted when, for `--scaling Insert` and
`FlexInsert`, every phase but `accessibility` is within 2% across sizes,
and `accessibility` is within its ceiling (`INSERT_A11Y_PER_ROW`,
`FLEX_INSERT_A11Y_PER_ROW`) and no more per row than the base, without
raising either; any term left over the base is reported before the
gate.

## 12. The per-row terms left after the lane stack

The lane stack (4b to 4f-1) does not finish property 3 alone. Measured
with `target/release/vapors-perf 60 10 --scaling <change>` at 4e
(ce006910), in instructions per extra row from 1000 to 3000 rows, insert
frames unless named. Where a term comes from was found with temporary
spans around the update's steps and `perf record --call-graph lbr`; each
slice below measures its cause again before its code.

| Term | Per row | Where | Removed by |
|---|---|---|---|
| The a11y touch of started rows | about 100; removed by 4i (2) (Insert update 132 to 49 a row) | `update`, `AxMirror::touch_slots`: each started lane expanded into its rows, each row's node put in the touched set; now each lane's range is kept and the take expands it once | 4f-3, folded into 4i (2) |
| Draws apart per lane frame | (draw calls, not instructions: 51 text draws for 17 rows, against 35) | `draw`, a batch per frame | 4f-2 |
| Draw over the merge base, after 4f-2e | Idle +2.6%, Insert +4.8%, FlexInsert +7.5% (flat across sizes) | `draw`: the batches phase (the sort, making the batches, the keys) and the painter's per-batch work, per row in view, not per row in the list | later optimisation |
| Lane batches cut at rows that paint shapes (fold option (b)) | retired in flat-scene F2.3b: shape and mesh items name their rows like every other item, so rows each holding one kind of item are one batch over the rows. A row mixing kinds (a background, an svg icon and text) still adds a batch per kind, as batches keep paint order; each of those batches now holds a lane range, so an insert above the view rewrites a range entry per batch for each row in view where it rewrote one, plus the row frames' entries. That case's table writes are unmeasured (no perf scenario holds shapes) | `framed_batches.rs` `resolved` no longer moves shape steps into their row's frame; `Batched::push` joins only the last batch | joining across rows is reordering: flat-scene F13 (`docs/plans/flat-scene.md`) |
| Scroll layout | 2857 (`--scaling Scroll`) | `layout`, all in placement (`painted_from`): after a scroll the container is settled (`Offsets::settle_box`) and every child visited (`visit_placed_in`), each child's lane start looked up (`LaneTree::start`, O(log n)) | 4g |
| Insert placement | about 14 | `layout`, placement: the same visit of every child of the lane container | 4g |
| Insert box build | about 37; after 4h (A) (#439) Insert layout is 17.2 / 18.7 a row (judged, 1k to 3k / 3k to 10k), from 50.9 / 53.4; after 4h (#443, #444) it is -0.2 a row from 1000 to 3000 and 1.4 from 3000 to 10000 (judged), the step three rows below | `layout`, all in the box rebuild (`Document::rebuilt`; laying out is flat): rebuilding the inserted row's owner enumerates every child of it | 4h; what is left is the three rows below |
| A second insert in one owner in one update | an O(n) search for each insert after the first, up to its place (`lane_children.rs` pins 96 and 396 entries for an insert near the end of 100 and 400 rows) | `layout`, the box rebuild (`Placed::child_position`): the owner's lane takes a row put in only at its next layout, so after the first insert the owner has more children than its lane, and later places are searched for, counted in `UpdateCounts::children_scanned`. The perf scenarios insert one row an update | a known term for updates that insert more than once; not planned |
| The box's children vector | at most about 0.1: remove frames, which move the same tail, measure 0.1 a row from 10000 to 40000 rows (judged). The 0.25 first recorded was an estimate (8 bytes a row) | `layout`, `put_edited`: an insert or removal in an owner edited in place moves the tail of its box's children vector (`Vec::insert` and `remove`) | 4l |
| Insert-frame layout, a step at some sizes | 1.4 from 3000 to 10000 rows, which the flat rule would fail if `layout` were judged for Insert (1000 to 10000: +9.3 k, +6.6%); not a slope: -0.3 from 10000 to 20000 and 0.5 from 20000 to 40000. Per frame: 140.9 k, 140.4 k, 141.7 k, 143.9 k, 150.2 k, 147.1 k and 157.1 k at 1000, 3000, 5000, 7000, 10000, 20000 and 40000 rows, within 300 between four runs. Remove frames: 0.2, then 0.1 | `layout`, in `laid_out`, split by temporary spans: from 3000 to 10000 rows the box rebuild adds 3.9 k and the lay out 5.4 k; the lay out is 75.1 k, 70.1 k and 72.4 k at 10000, 20000 and 40000. Not the lane tree's depth, which is 3 at 1000 rows, 4 from 3000 to 7000 and 5 from 10000 to 40000. Not located further; work that depends on the slot numbers (hash probes) would vary like this, unverified. Reproduce: `cargo build --profile judged -p vapors-perf && target/judged/vapors-perf 60 10 --scaling Insert`, other sizes by editing `SIZES` in `crates/perf/src/scaling.rs` | not located; not planned |
| Resize, after the lane work (2026-11-16) | accessibility 2461, layout 14, update 16 (judged, 1k to 10k, at #434's head); a layout figure of about 33k a row from an earlier count is stale | accessibility: one bounds write per moved row (4i's options); layout and update small | recorded, not a target this iteration |
| Insert accessibility | about 4720 (FlexInsert about 1800); after 4i about 1230 (FlexInsert about 230), judged | `accessibility`: the take made its index afresh on every insert and remove (`AxIndex::of`, 71% of Insert's take; now patched, 4i (1), #432), and read every started row again in full (now each row's bounds alone, 4i (2)). What is left is one bounds write a moved row: the a11y tree carries every node's bounds, each from its parent node | 4i; the rest deferred by Tom (2026-11-16) to a later accessibility iteration, with the two options under 4i and no slices planned |
| An out-of-flow child of a lane container | `--scaling PositionedInsert` (W6): layout about 10,100, paint about 13,200, accessibility about 6,900 (judged, 3000 to 10000); without the page's fixed footer, Insert's figures | the container's lane is declined (`LaneDecline::OutOfFlowChild`) for an absolute or fixed child, so an insert under it costs what it did before lanes. The change is reported, not judged, until 3b (`docs/perf/w6-positioned-2026-11-17.md`) | 3b |
| Remove-frame paint | 0.1 (+3.1% at 10000 rows) | `paint`: most likely `DrawnSteps`' sorted vector, whose removal moves every later entry (a memmove, a fraction of an instruction a row) | 4j |
| Every term, for lists whose rows have fractional heights | as before lane frames: Insert paint about 12,000, layout about 5000, accessibility about 9300 | the lane is declined (`LaneFramePixel`): a frame's shift is whole device pixels, and a fractional start is not. Rows of 19.2 px (a 16 px font at line-height 1.2) are not whole at scale 1 or 2.75, so this is common. A restyle can flip a list either way (the flip 4e's review found; drift between the lane's and Taffy's sums, issue #348) | 4k |

The slices:

- **4f-2, 4f-3** as section 11.2 says (#398).
- **4g. Placement visits only the marked children of a framed lane.**
  A framed lane's children are in their own frames, so a scroll of the
  container or an insert before a child moves none of them in its
  frame; placement visits the children marked (built again, or with a
  descendant that changed) and finds each by its handle, not every
  child by its start. The settle after a scroll settles the scrolled
  container's own range, not its children's boxes. Accepted when
  `--scaling Scroll`'s and `Insert`'s `layout` hold no per-row term from
  placement, and the placement check (`placement_check.rs`) and the
  scroll tests are unchanged.
- **4h. An owner's children edited, not walked.** Measured first (10k
  rows, a cycles profile of `Document::update`): lending from the edited
  owner, the order splice's copy, and the a11y row touch. (A), #439: an
  owner edited in place lends only to its inserted children, O(edits);
  Insert layout 50.9 to 17.2 a row, FlexInsert 50.5 to 17.0. (B), the
  order splice made in place, was measured and dropped: the copy is a
  vectorised memcpy, cheap in instructions (judged layout unchanged,
  50.6 against 50.9) and in layout's median time (about 371 µs either
  way); its weight in the cycles profile was allocation and page faults.
  It is not an instruction term. The a11y row touch was outside the
  layout span (4f-3, removed by 4i (2)). About 17 a row remains in
  Insert's layout, not yet placed.
- **4i. The a11y take after an insert.** Measured first: 71% of
  Insert's take was the index made afresh on every insert and remove,
  26% every started row's record read again in full. (1), #432: the
  index is patched from the elements whose entries can change (an
  indexed attribute written, a subtree inserted, moved or removed),
  checked against a scan in debug builds. (2), with 4f-3: the update
  keeps each lane's started range; the take expands it once and gives
  each row nothing else touched its bounds alone (its record's, or its
  nearest nodes' when it is not a node; before #436 a started row with
  no role left its nodes' bounds stale, and #436 read them in full).
  Judged ceilings
  tightened to Insert 1250, FlexInsert 240, Resize 2480.

  Still per row: each moved row's bounds are rewritten, as the mirror
  carries every node's bounds from its parent node, about 1240 a row.
  Deferred by Tom (2026-11-16) to a later, dedicated accessibility
  iteration; no slices are planned for it now. The two ways recorded to
  make it independent of the list's length:
  - **Bounds relative to a node per lane.** The platform would compose
    each row's place from a lane node and the row's offset in it, so an
    insert writes the lane node and the new row. There is no AccessKit
    here: the platform mirror is the web page's DOM
    (`crates/platform/src/a11y_mirror_*.rs`), each record an element
    placed `position: absolute` from its parent's. The in-flow mirror
    (PR 5 in the comments: rows as static elements of their heights in
    a lane element, which the browser places) is that option there; a
    native platform would need per-node transforms, to be checked in its
    toolkit when one is added.
  - **Fresh bounds only for rows in or near the view.** Rows far
    offscreen would keep stale offsets until they come near the view.
    It needs a check that platforms tolerate it, against the a11y
    oracle (`ax_oracle --check`) and a screen reader's virtual cursor,
    which reads offscreen rows.
- **4j. Drawing steps in blocks.** `DrawnSteps`' vector of drawn steps
  is kept in blocks, as the extents index's entries are (`blocks.rs`),
  so a removal or insert moves one block. Accepted when `paint` is
  within 2% across sizes on remove frames too.
- **4k. Rows of fractional height framed.** A row's frame shift is its
  start in device pixels rounded as an item's edge is (half up), and the
  row's items are painted from that whole origin, so a row is snapped
  as a unit rather than each item on the page. An item can then land a
  pixel from where snapping it on the page puts it. Chromium snaps a
  box's paint offset before its contents, so the first step is the
  Chromium paint comparisons on a fractional-row page, framed and
  declined. If framed rows match Chromium as well as declined rows do,
  4k frames them; if not, they stay declined. In that case the plan
  records the cost (every term above, per row, for such lists) and the
  nightly scaling run reports a fractional-row page. Accepted when the
  comparisons show no new difference, and `--scaling` on a 19.2 px row
  page meets the exit checks the whole-row pages meet.
- **4l. The box's children in blocks.** The box's children vector
  kept in blocks, as 4j keeps the drawn steps, so an insert or removal
  in an owner edited in place moves one block, not the tail. Measured
  at most about 0.1 a row (the table), so recorded, not built.

Order and fold (the lead, on #402; changed 2026-11-16 after Tom's "fine to have optimisations for later"):
- **4f-2, with 4f-2b to 4f-2e**, first: batching across lane frames (#407), one search a lane and batches made with less work (#414), the patch at a lane's ends (#415), places kept between frames (#418), and one table per lane part with stable row slots (#423).
- **Then fold and gate.** #423 becomes the fold pull request against main, and the PRs under it close as included. The correctness checks are unchanged. The draw terms left over the stack's merge base (75e63c84) are later optimisations, not blockers; they are recorded in the fold's pull request and in the table below. At 1 codegen unit, draw at 1000 rows (3000 and 10000 flat):
  - Idle: 907,046 to 930,823 (+2.6%);
  - Insert: 1,004,567 to 1,052,817 (+4.8%);
  - FlexInsert: 152,155 to 163,635 (+7.5%).

  Paint falls from 11.1 M to 189 k (Insert) and from 2.8 M to 123 k (FlexInsert).
- After the fold, each slice lands on main as an ordinary PR with its own exit check in the 11.3 form, in this order:
  - **4f-3**, the a11y touch of started rows, measured before it lands;
  - **4g**, as scroll is what a reader feels;
  - **4i**, the largest term by far. The phases are measured apart, so the layout terms do not confound it.
  - **4h**;
  - **4j**, the smallest;
  - **4k**, which starts with the Chromium evidence and may end with the decline kept and its cost recorded;
  - **4l**, recorded, not planned;
  - the draw terms, as a later optimisation.
