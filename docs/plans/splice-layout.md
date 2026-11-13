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
| Flex column, `flex-wrap: nowrap`, every item `flex-grow: 0`, `flex-shrink: 0` or the container's main size auto, `justify-content` `flex-start` or `normal`, no `order` | lane (PR 6) | the sum-plus-gaps fold; `align-items` is cross-axis per item |
| Flex row (a horizontal list) | lane (PR 6, same code, axis swapped) | |
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
| 0 | Measurement: `--scaling Insert` reports insert and remove frames apart (two medians per phase; the judge reads the worse); a `Remove` workload is not needed. Section 1.4 filled with the per-phase split and the counts | `docs/perf/splice-layout-<date>.md` with the split; no code change beyond the harness | this plan's author; first |
| 1 | The splice follow-up (`docs/plans/splice.md` "As built"): `build_shallow` builds the owner's children from the kept box's children with the inserted slots put in after their previous element sibling's slot and the removed ones dropped (the children mark carries the nodes, `Marks::mark_child_inserted`), no walk over the DOM children; `Rebuilt::subtrees` keeps a count, not a clone; `plan` reads the old children by rank from the kept order; `spliced_steps` edits in place; drawn steps, hits and batches patched by the splice's edits instead of made afresh; `ClipTree::spliced` regroups the touched parents only | Insert at 3000 rows: `boxes_built` unchanged, build-phase and paint-phase instructions within an edit's except the shift (counted); `rebuild_check` and `kept_order_check` green | splice lane; before PR 3's acceptance number, not before its code |
| 2 | `layout/lane_tree.rs`: the sum tree (B-tree, fanout 16, handles, parent links): insert after, insert first, remove, set, `start`, `between`, `total`, in-order iteration; the block fold of section 2.1 as a `Summary` type with `combine`; a property test over random edits against a sequential fold; the collapse-through join declined and tested | Unit tests; `cargo xtask lint`'s file sizes; no behaviour change | layout lane; with PR 0 |
| 3 | The block lane in layout: `LaneDecline` from the used styles at build; `LayoutBox::lane: Option<Lane>` on the container, lent with its cache; `lane_block.rs`, the per-child layout mirroring taffy `block.rs`; `dispatch` routes a lane container to it; a lane child's `Layout.location` on the axis is zero and `BoxTree::location(slot)` derives it; `written.changed` not pushed for lane children whose summary is unchanged; `settle` updates a lane ancestor in place of clearing it; the layout check's tolerance; counts `lanes`, `lanes_declined`; a decline test per row of section 4 | Insert at 3000 rows: `layout_calls` the new row's plus a constant, `boxes_laid_out` the same; `--scaling Insert` `layout` phase within 2% across sizes; the Chromium layout fixtures unchanged; every decline reason tested | layout lane; after 2 |
| 3b | The collapse-through join in the fold, with its test; absolute children of a lane container from the tree | the two declines of section 4 become lanes, each with a Chromium fixture | layout lane; any time after 3 |
| 4 | Derived positions: `Placement` leaves a lane child's axis coordinate unstored and `rect(slot)` adds the chain's starts; the walk does not enter a lane child whose summary is unchanged; `moved` empty for them; a `restacked` set reported to the a11y take until PR 5 (5.3); parts, entries and hits of a lane child in its frame (its slot), on #248's naming; `Frames::at` lazy and memoised (5.4); `between(view)` for culling and for hit tests inside a lane (5.2) | Insert at 3000 rows: `boxes_placed` the new row's boxes plus the chain to the root, `boxes_moved` 0, `boxes_shifted` 0; `--scaling Insert` every phase but `accessibility` within 2% across sizes; pixels unchanged (the Chromium paint comparisons, the kept-paint debug check with translations); hit tests on the rows page unchanged (`sparse_updates` hover) | layout lane with the paint lane for `paint*.rs`, `kept_*.rs`; after 3 and IU4 4-ii |
| 5 | The mirror in flow for lane children (5.3): `AxBounds::Stacked`, the mirror's `position: static` elements, the touched set after an insert; `--scaling Insert` judged `EveryPhase` | `accessibility` within 2% across sizes for Insert; `RESIZE_A11Y_PER_ROW` back to 13,000 (#289); the 5b oracle's bounds unchanged; `a11y_mirror_scroll.rs`-style tests for an insert | R6 owner; after 4, or in parallel on `vapors-a11y` and `vapors-platform` with the elements side stubbed |
| 6 | Flex lanes: column and row, the sum-plus-gaps fold, the flex per-item layout (base size, hypothetical main size, stretch cross size) for the no-grow, no-shrink, nowrap case; `FlexInsert` workload on `rows_list.rs` | `--scaling FlexInsert` passes every phase; `sparse_updates_flex` counts | layout lane; after 4 |
| 7 | Acceptance: Insert, Remove and Resize at 1000, 3000 and 10,000 rows, instruction counts, wall clock, in `docs/perf/`; property 3 marked held or not in `perf-finish-line.md`; `splice.md` (e) and `iu4.md` 8.5 closed | `--scaling Insert` and `Resize` pass `EveryPhase`; update under 1 ms at every size | last |

Order: 0, 2, 3 can start now (2 and 3 touch `crates/elements/src/
layout/` only, no file #248, #243 or the splice follow-up change); 1 is
the splice lane's; 4 waits for IU4 4-ii's frame coordinates; 5 waits
for 4 on the elements side; 6 after 4; 7 last. The estimates in
section 6 assume PR 1 lands before PR 4's acceptance run.

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
