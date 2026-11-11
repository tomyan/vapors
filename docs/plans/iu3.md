# IU3: the retained box tree

Decision: touche `docs/decisions/2026-11-10-iu3-retained-box-tree.md`
(Tom): vapors keeps its box tree across updates and keeps Taffy. Brief:
`docs/briefs/INCREMENTAL_UI_PERFORMANCE_BRIEF.md`, slice 3.

## Where the time goes today

At 3000 rows an idle update (an equal write) takes 63 of a 64 ms frame:
style 14.1, layout 25.7, display-list build 19.7 ms (release, after
culling, `docs/perf/baseline-2026-11-10.md`). At 1000 rows it visits
4,002 elements and builds and paints 7,002 boxes (`docs/plans/iu1.md`,
PR 2). Each update `build`s every box again (`layout/boxes.rs`), then
`KeptBoxes::lend` diffs the new tree against the last to hand back
Taffy caches (`layout/keep.rs`). An absolute or fixed box is never lent,
because its containing block's hoisted list names boxes by index in the
dropped tree; that is why the racer HUD lays out 19 boxes on every idle
update.

## Design

1. **Box slots.** The boxes live in a `Vec<LayoutBox>` kept by the
   document, indexed by slot, which is also Taffy's `NodeId`. A slot
   keeps its index while its box exists, so Taffy's caches, the
   out-of-flow candidates in cached `LayoutOutput`s and each containing
   block's hoisted list stay valid. Slot 0 is the root's. A freed slot
   holds `Content::Vacant` (new variant: every `match` on `Content`
   handles it, and box walks from the root never reach it) and is reused
   from a free list. A map gives each element's slot; a text box's key
   stays `(parent element, position among its text boxes)`.
2. **Box marks.** The document marks an element when something its box
   is built from changes: `set_attribute`, `set_property` and the
   person's edits that write properties (checkbox, number, range, text
   field) mark the element; `set_text` marks the text's parent;
   `insert_before` and `remove` mark both parents; every element the
   restyle pass restyled is marked (`Kept::restyled_nodes`);
   `register_image`, `forget_kept`, a scale change and a failed layout
   mark everything. The focused field's caret and selection are compared
   at the update with what its box holds, not marked.
3. **Rebuild where marked.** Each mark resolves to its owner: the
   nearest element, itself or an ancestor, that has a slot and still
   generates a box of its own (not `display: none`, not inline). Owners
   inside another owner are dropped. Each owner's subtree is built
   again; a new box whose key had a slot in the old subtree takes that
   slot, and the cache and layout when its style, content, natural size
   and children are unchanged (today's `Lender` rules, on the subtree);
   other slots in the old subtree are freed. When the owner's subtree
   changed, the caches of its ancestors are cleared up to the root, as
   Taffy's `mark_dirty` does. No marks: no box is built, and Taffy reads
   the root's cache.
4. **Absolute boxes.** With slots stable, an absolute or fixed box is
   lent its cache like any other (PR 2): a change inside its containing
   block clears that block's cache, which lays it out and hoists it
   again; a change elsewhere leaves both cached.

Kept by marks, not by diffing the whole tree: a missed mark leaves a
stale box. The oracle is the check that catches one: every PR keeps
`crates/elements/tests/sparse_updates.rs`, `incremental_random.rs` and
the other `incremental_*` tests green unchanged, and every Chromium
layout and paint comparison passes unchanged. A PR that adds a mark
point adds an oracle case that fails without it.

## PR stack

Each under about 400 lines. Measured on the harness
(`cargo build --release -p vapors-perf && ./target/release/vapors-perf
200 20`, 1-min load under 1.5, load recorded) before and after, in
`docs/perf/baseline-2026-11-10.md`. Counts are `UpdateCounts`, pinned in
`sparse_updates.rs` (`EXPECTED_WALKS`) and `update_counts.rs`, each new
pin proven by breaking what it guards.

| PR | Content | Acceptance |
|---|---|---|
| 1a | Box slots and `Content::Vacant`; `build` writes slots by key; the element-to-slot map replaces `KeptBoxes::element`'s scan. Still rebuilds every box each update | Every count unchanged; oracle and Chromium comparisons green; idle update ms at 1000 and 3000 rows within run-to-run noise of before |
| 1b | Box marks and rebuild where marked | Idle update: `boxes_built` 0 at 100 and 1000 rows; same-size value edit and resizing label edit build only the edited label's boxes (expected 2); class toggle and hover build one row's (expected 7). Layout phase at 3000 rows idle well under today's 25.7 ms (recorded); boxes laid out unchanged except where 1c's absolute boxes stop being relaid out |
| 2 | Absolute and fixed boxes lent their caches; 3D 1c's IU5 check moves from "no more than idle" to 0 (`docs/plans/3d-1c.md`) | Racer HUD idle update lays out 0 boxes (19 today), pinned; a test with an absolute box whose containing block resizes matches a full update |
| 3a | The style phase limited to changed elements: the tree check, style-attribute refusals, calcs and media checks kept per element and redone for marked ones, instead of the walk over every element in `Document::styled` | Idle `elements_visited` 0 (4,002 at 1000 rows today); style phase at 3000 rows idle near 0 (14.1 ms today) |
| 3b | `placed`, `scroll.settle` and `Chains::of` kept, redone under changed boxes | Layout phase at 3000 rows idle near 0 |
| 3c | Display list kept across updates, repainted only for changed boxes (design and split: "3c: kept display list" below) | Idle `boxes_painted` and `scene_items_built` 0; an edit paints only its changed boxes; display-list build at 3000 rows idle near 0 (22.1 ms after 3a) |

3a to 3c each may split further once 1b's measurement shows which walk
dominates.

Requirements from the review of #185, each with a test in its PR:

- 1b: a slot a new key takes resets the box's `cache`, `layout` and
  `hoisted`, so nothing from the slot's last box survives. Clearing an
  ancestor's cache also clears its `hoisted` list; otherwise
  `containing_blocks` reads slots that were freed.
- 1b: marks cover every input a box depends on. An edit inside an svg
  marks the svg element, which owns the boxes. A viewport resize that
  changes `vw` lengths or which media rules apply marks what restyle
  restyles on resize, or every element when that set is not known.
- 2: `lend_from` (`layout/keep.rs`) lends `hoisted` along with `cache`
  and `layout`, or an absolute box lent its cache is missing from its
  containing block's list.
- 3a: unmarked elements keep their `Calcs` entries, since retained
  boxes hold calc handles into them.

## 3c: kept display list

Scope (lead, 2026-11-11): the CPU display list only. GPU-side retention
(paint records, buffers, the painter's per-frame work) is IU4. Culling
(#184) works on the scene at batching and is unchanged.

What one box's paint reads, and how each read is kept correct:

- Its own box and style: a restyle, an edit or a move marks the
  element and its box is built again (1b), so a rebuilt slot repaints.
- Its rect (placed: ancestors' positions and scroll offsets), its
  border and padding, its clip (ancestors that clip it), and for a
  text field its first child's rect (the text origin): compared with
  the values its kept paint was made from; any difference repaints it.
- The canvas background: whether `html` or `body` gave the canvas its
  colour decides whether they paint their own; compared like the rect.
- Paint order (stacking contexts, z-index, outlines last in their
  context, thumbs after positioned descendants): not kept per box. The
  order is walked again on an update that repaints, and kept paint is
  copied in that order, so a z-index change or a box moved into another
  context reorders kept paint without repainting it.
- Scroll thumbs: read the scroll offsets and whether thumbs are shown;
  painted again on every update that repaints (a few items).
- Scale: a change marks every element (1b), so everything repaints.

PRs, each under about 400 lines:

| PR | Content | Acceptance |
|---|---|---|
| 3c-1 | The last update's `Painted` is kept with what it was made from: every slot's rect (scroll offsets applied), whether thumbs are shown, and the scale. An update that built no box and has the same rects, thumbs and scale shows it and paints nothing. Each of the three comparisons and the built-box count is a path that paints again, with a test that fails without it. A debug build paints again on the kept path and asserts the two paints are equal, naming the parts that differ | Idle `boxes_painted` and `scene_items_built` 0, pinned; display-list build at 3000 rows idle near 0; the oracle and Chromium comparisons unchanged |
| 3c-2 | Paint kept per box (its items with ids local to it, its hit, cursor, texts, images, icons, surfaces, range track and text origin) with the values above it was made from; an update that repaints walks paint order, paints the boxes whose values differ or whose slot was rebuilt, and copies the rest, renumbering ids in paint order so the scene equals a full paint | Text change paints the edited label's boxes, class toggle and hover one row's, pinned in `EXPECTED_WALKS`; each compared value proven by breaking |
| 3c-3 | Triggered by 3c-2's profile: paint order, clips, scroll chains, the element maps and the scene kept in place and redone under changed slots ("3c-3: kept paint order and an in-place scene" below) | Text and Class edit paint at 3000 rows under 1 ms |

## 3c-3: kept paint order and an in-place scene

Why (3c-2, #192 at b4befbc): an edit at 3000 rows paints 2 to 7
boxes, yet paint takes 21.2 ms (22.4 on main). Where it goes
(`docs/perf/baseline-2026-11-10.md`, "IU3 3c-2: where an edit's paint
goes"): about 10.6 ms in four passes over every box that main runs too
(paint steps 4.4, scroll chains 2.9, `started`'s rect and padding maps
2.0, clips 1.7), and about 9.6 ms in the per-box copy loop
(`TextCache::touch` hashing about 9000 keys twice; `copy_into`'s 12k
items, 9k `Rc` clones and 36k map and vector entries; taking and
comparing each slot). Goal (lead, 2026-11-11): an edit's paint costs
in proportion to the slots it changes; Text and Class edits at 3000
rows under 1 ms.

### Changed slots

Every kept structure below is patched from one set per update:

- Rebuilt: the slots `KeptBoxes::rebuild` placed. It returns them,
  and the slots it freed, in place of today's count;
  `rebuild_owners` already takes each old subtree before placing the
  new one, so it holds both.
- Moved: the slots whose rect in `PaintedFrom::rects` differs from the
  kept paint's. Found by comparing the two vectors, O(slots), as
  3c-1's idle check does (the whole idle paint at 3000 rows, 21002
  rects compared, is 0.05 ms). When 3b keeps `placed`, it reports the
  moved slots and this comparison goes.
- Shape changed: a slot was freed or newly taken, or a rebuilt box's
  children, `Stacking` or whether it has an outline differ from what
  is kept for its slot. Compared for rebuilt slots only.

A scale change rebuilds every box (1b) and paints everything again, as
today.

### The scene in stable slots

- `Painted` keeps each slot's part (3c-2's `Part`: items with their
  clips, hit, cursor, visible flag, range track, text origin) in place.
  An update takes the last `Painted`, paints the changed slots' parts
  again, drops the freed slots' parts, and returns it. A kept part is
  neither read nor copied.
- Ids: texts, images, icons and surfaces are held in slabs in
  `Painted` (a `Vec<Option<_>>` and a free list). A part's items name
  slab ids, which stay while the part is kept; painting a box again
  frees its old ids and takes new ones. `Painted::text(id)`, `image`,
  `icon` and `surface` keep their signatures. Ids are no longer dense
  or in paint order; the painter only resolves them through those
  accessors (no cache in `crates/app` is keyed by an id).
- Paint order is the kept `order` (below). The scene is the parts
  walked in that order. `Painted::scene()` returns a view in place of
  `&Scene`: `items()` iterates `(Item, Option<Rect>)` through order
  and each part, and `batches(view)` batches that iterator.
  `vapors_scene::Scene` stays the flat list each part is painted into
  and the one the racer builds; batching becomes a function over an
  iterator of items and clips that `Scene::batches` and the view both
  call. 20 uses of `scene().items()` and `clips()` in 13 files move to
  the iterator.
- The painter changes only in calling the view's `batches`. Per drawn
  frame, batching is O(steps + items) where today it is O(items): at
  3000 rows about 21000 steps (order lists an outline step only for a
  box with an outline, where today every box has one) and 12000
  items. Batches kept across frames, or a per-scroller index so a
  frame visits only visible items, is IU4.
- Hit testing walks order backwards through each part's hit: O(steps)
  per query, where today it is O(hits) over a list rebuilt every
  repainting update.
- `cursors`, `visible`, `ranges` and `text_origins` (by `NodeId`) are
  kept: a part painted again removes its old entries and inserts its
  new ones; a freed slot's part removes its own.

### What is kept, and what redoes it

| Kept | Reads | Redone when | How |
|---|---|---|---|
| Paint order (steps), containing blocks, thumb placement | tree shape; each box's `Stacking`; which boxes have an outline | shape changed | whole tree, as today; counted `order_walks` |
| Scroll chains | shape, containing blocks, `Stacking::scrolls` | shape changed | `Chains::of`, as today; `chain_walks` |
| Clips, by slot | containing blocks; each clipping box's padding box and its `clips()` | a changed slot; shape changed | own and inner clip again for each changed slot and its descendants, stopping below a slot whose own and inner clips come out equal unless a descendant is itself changed; whole tree on shape change; `clips_redone` |
| `rects` and `paddings` maps | rect, border, padding | rebuilt, moved or freed slot | entry replaced or removed; `map_entries_patched` |
| Inline elements' entries in `rects` | the spans of each text run's box | the run's slot rebuilt or freed | each span of the taken run removed before paint; paint (or the copied part) puts back those still in a run; not counted |
| A slot's part | its box and style, rect, clip, padding (3c-2's `BoxFrom`) | rebuilt, or `BoxFrom` differs | painted again (3c-2's rule), compared only for changed slots and slots whose clip was redone |
| Thumbs | scroll offsets, whether shown | every repainting update | painted again, O(scroll containers), as today |
| Canvas | `html` and `body` style | every repainting update | O(1), as today |

Whether a box clips is read from its style, so a change to it
rebuilds the box; the body's clipping also reads the root's
`overflow`, and a root restyle rebuilds everything.

The edits in `EXPECTED_WALKS` (text, resizing label, hover, focus,
class toggle) keep shape, so they do no whole-tree walk. An edit that
changes shape (insert, remove, reorder, a z-index or position change)
redoes order, chains and clips over the whole tree, about 9 ms at
3000 rows; its build already covers the owner's subtree (1b), which
for an insert into a list is the list.

### Text cache

`current` and `previous` with `begin` dropping what the last update
did not ask for, plus `touch` and `keep_previous`, become one map
whose entries record the update that last asked for them. A sweep
drops entries older than the last update whose layout no kept part
holds (`Rc::strong_count == 1`). It runs when the map has doubled
since the last sweep, so its cost is amortised O(1) per insert and
the map holds at most twice what the last sweep left. A kept box
costs nothing per update; `KeptBox::text` and `TextCache::touch` go.

### Loud failure

- The debug build's check (`kept_paint_check::assert_same`) stays on
  every update that paints: a full `paint` and `Chains::of`, the text
  cache cloned and restored. It extends to the kept order against the
  fresh steps, kept clips and containing blocks, chains, the `rects`
  and `paddings` maps and the per-element maps, and compares the scene
  item by item with ids resolved (texts, images and icons by
  `Rc::ptr_eq`, surfaces by name), naming the first step and slot that
  differ. It also checks the slabs: each live id named by exactly one
  part, no free id named.
- A step whose slot is vacant panics naming the slot, as `keep` and
  `clip` do since #185.
- One test per path, each proven by breaking it (the commit lists
  each): a child moved before a sibling, same keys (children compare);
  a z-index set on a rebuilt box (`Stacking` compare); position static
  to relative (`Stacking`); an outline added by a class (outline
  flag); a removed element has no border box, cursor or hit (freed
  slots); a clipping container shrinks with its children kept (clip
  descent); a changed descendant below an unchanged clip is still
  redone (the descent's stop rule); `overflow` toggled (chains and
  clips); a scroll (moved slots); a layout held by a kept part
  survives updates that do not ask for it, an unheld one is gone after
  the next sweep (text cache); a repainted box's freed ids are taken
  by a later box and no stale id resolves (slabs).
- New `UpdateCounts`: `order_walks`, `chain_walks`, `clips_redone`,
  `map_entries_patched`, `parts_visited_kept`; pinned in `EXPECTED_WALKS`
  beside `boxes_painted` and `scene_items_built`.

### #192

Of #192, the base for 3c-3: `KeptBox`, `Part`, `BoxFrom` and the rule
for when a box paints again; `painted_again`; the `paint_own.rs` and
`update_layout.rs` splits; `tests/kept_box_from.rs`'s proofs; the
`EXPECTED_WALKS` pins (`boxes_painted` 2 to 7). Replaced: `touch` and
`KeptBox::text` (3c-3a); `copy_into`, `Offsets` and `Walk`'s two slot
vectors (3c-3d).

Recommendation: rebase #192 onto main and merge it before 3c-3a, as a
correct step that is not faster. 3c-3a to 3c-3d each change files #192
changes (`paint.rs`, `kept_paint.rs`, `text_cache.rs`, `painted.rs`),
so stacking them on an unmerged draft of about 870 added lines keeps four PRs
rebasing on it; and its pins become the baseline the new counters sit
beside. Against: until 3c-3d lands, main carries the copy loop, and
#192's harness run had paint at 1000 rows 0.2 to 0.3 ms slower (5.62
to 5.84 Text, 5.61 to 5.91 Class, one run each). The other option,
closing #192 and carrying its parts into 3c-3d, puts about 870 lines
reviewed once already into a PR that is then over 400 lines. The
lead decides.

### PR stack

Each under about 400 lines; harness `vapors-perf 200 20`, 1-min load
under 1.5, edit-frame paint at 1000 and 3000 rows before and after in
`docs/perf/baseline-2026-11-10.md`. Expected drops are the profile's
uninstrumented or single-span figures at 3000 rows.

| PR | Content | Acceptance |
|---|---|---|
| 3c-3h | Hashing (perf profile, `docs/perf/baseline-2026-11-10.md`): SipHash and `hash_one` are about 5 ms of a 21.6 ms edit paint at 3000 rows on #192 and main alike (NodeId lookups about 2, text `Key` about 2.4). `NodeId` maps and the text cache use a fast non-cryptographic hasher (`rustc-hash`); `scroll_containers` and `stacking_of` read per-slot values computed once per update in place of a `NodeId` lookup per walk; the text `Key` stores its hash, computed once | Text and Class edit paint at 1000 and 3000 rows before and after, on the full-repaint path too; no change to any oracle or Chromium comparison |
| 3c-3a | Text cache by age and holder | `text_layouts_built` unchanged on every `EXPECTED_WALKS` case; Text and Class paint at 3000 rows recorded (`touch` is 4.5 ms instrumented, less uninstrumented) |
| 3c-3b | Changed slots (`rebuild` returns rebuilt and freed slots; moved found by comparing rects); `rects` and `paddings` kept and patched | `map_entries_patched` pinned (the changed element slots); `started`'s 2.0 ms gone at 3000 rows |
| 3c-3c | Order, containing blocks and chains kept, redone on shape change; clips kept, redone under changed slots | `order_walks` and `chain_walks` 0, `clips_redone` 2 and 7 for Text and Class; a row insert `order_walks` 1; about 9 ms less at 3000 rows (order 4.4, chains 2.9, clips 1.7). Split at order and clips if over 400 lines |
| 3c-3d1 | Slabs for texts, images, icons and surfaces; parts keep their ids, copied without renumbering; the debug check compares resolved ids | Scene items and ids equal a full paint's after resolving; harness unchanged or better |
| 3c-3d2 | Parts kept in place; the scene view and batching over an iterator; hits by walking order; per-element maps patched | `parts_visited_kept` 0; Text and Class edit paint at 3000 rows under 1 ms; at 1000 rows recorded; draw at 3000 rows recorded before and after |
| 3c-3e | Moved slots shift their kept part by the rect's change (option (a) below) in place of painting it again; a harness workload whose edit resizes a label near the top of the list, and one that scrolls the list | Resize and scroll edits at 3000 rows: `boxes_painted` only the rebuilt slots, pinned; paint recorded before and after |
| 3c-3f | Kept batches ("3c-3f: kept batches" below): the drawing steps with an item in the view, and the culled count of the others, kept for the last view and patched for the slots painted again; the batches kept until a shown step changes or the view does | Draw at 3000 rows at or below 1.0 ms for Text, Class, Idle and HoveredClass; recorded at 1000 rows; update and paint not worse; counts unchanged |

### 3c-3f: kept batches

3c-3d2 batches every drawing step's items on every drawn frame: 12,000
items at 3000 rows, each read through its step's slot, though only the
rows in the window are drawn and the rest are culled. 3c-3f keeps, in
`Painted`, for the view of the last draw:

- the drawing steps that show something in the view: an item whose
  bounds meet the view under its clip, or a surface (a culled surface
  keeps its place in a batch marked culled);
- for every other drawing step, how many items it has, all culled, and
  their sum;
- the batches, made from the shown steps' items, with the sum added to
  their culled count.

A draw with the same view returns the kept batches; with no batches
kept, it batches the shown steps' items only. A draw with another
view, or the first draw after paint order is walked again, finds the
shown steps afresh over every item. An update that paints slots again
patches the kept steps for those slots' steps, through the places
`DrawnSteps` holds: a step hidden before and after changes only the
sum; a step shown before or after drops the batches. Scale changes and
fresh paints start a new `Painted`, which keeps nothing. Thumbs are
painted on every update and patched as any other slot.

The painter takes the batches from `Painted::batches(view)` by
reference in place of `scene().batches(view)`; what it receives is the
same `Batched`. A debug build compares every batches call's result with
`batched` over the scene view's items and names the first differing
batch.

A view that changes on every draw is slower than before this PR:
`KeptBatches::of` tests every item for culling, allocates `hidden` (one
`u64` per order step) and `shown`, then batching tests the shown items
again, where `batched` was one pass. No caller draws one `Painted` at
two views per frame; it happens during a live resize. If a resize is
measured, batch in the same pass as the split when the view changed.

### Lead decisions (2026-11-11)

- #192 is rebased onto main and merged before 3c-3a, as a correct
  step that is not faster; its +0.2 to 0.3 ms at 1000 rows is
  accepted until 3c-3d2 removes the copy loop.
- Moved boxes: option (a), as PR 3c-3e. A scroll or a resized label
  repainting most of a 3000-row list is the scaling case this plan is
  for, and (a) needs no change to the scene format. (b) stays in IU4.
- 3c-3h (hashing) goes first: it lowers both the full-repaint and the
  kept-paint paths, needs no change to what is kept, and makes the
  later PRs' before and after figures smaller and easier to read.
- Batching over steps and items per drawn frame is accepted for 3c;
  3c-3d2 records draw at 3000 rows, and a measurable rise brings kept
  batches into this stack.

### Open points and risks

- After 3c-3, an edit's paint still compares 21002 rects at 3000 rows
  (about 0.05 ms) until 3b reports moved slots. The update outside
  paint stays above 1 ms until 3b: idle update at 3000 rows is 2.05 ms
  with paint 0.05 (3c-1).
- Moved boxes paint again. A label whose width or height changes moves
  everything after it; a scroll moves every box in the scroller; at
  3000 rows either can repaint most boxes. Options: (a) shift a moved
  part's items by the rect's change in place of painting it again,
  O(its items), no text or `Rc` work; (b) parts in scroller-local
  coordinates with the offset applied at batching, which changes the
  scene format, the painter and culling (IU4 territory); (c) leave both
  as they are for 3c. The harness's Text edit keeps its width, so this
  does not block the 1 ms target there; a resizing workload would.
- Shape changes redo order, chains and clips over the whole tree
  (about 9 ms at 3000 rows). Splicing a rebuilt owner's steps into
  order, when its subtree has no positioned, scrolling or outlined box,
  is a later step if a workload shows it.
- Per drawn frame: batching walks steps as well as items, and hit
  testing walks steps per query. 3c-3d2 records draw at 3000 rows; if
  draw rises measurably, keeping batches across frames moves from IU4
  into this stack, and the lead decides.
- Hit gathering (3c-3d2, kept in 3c-3g): the hit list is kept across
  updates and patched for the slots painted again or freed: one splice
  per patched place, its range found by binary search, so the cost is
  O(k log n) plus the tail move when a part's hit count changes. It is
  gathered afresh only after paint order is walked again. `hit()` scans
  the kept hits from the last: at 3000 rows about 0.11 M instructions
  and 13 µs per test, which is what HoveredClass's update still costs
  above Class's and grows with the row count. A spatial index waits
  until a page makes that measurable (above 0.1 ms).
- The debug check's fresh paint (3c-3d2) is `paint_partial` with
  nothing kept, so it compares an incremental paint with a full one
  through the same `Repaint` code: a defect inside `Repaint::painted`
  or `Repaint::thumbs` would appear on both sides and pass. Comparing
  against an independent full-paint path would close that; not built.
- Unknown until 3c-3d2 is measured: whether the slab frees and map
  patches for 7 boxes, plus the rect comparison, fit in 1 ms
  alongside painting the boxes (3c-2 measured 0.01 ms for painting
  the changed boxes at 3000 rows).

## 3b: kept placement

Today every update settles every scroll offset (`scroll.rs`) and
places every box (`layout::placed`): its rect is its location plus its
containing block's content origin, that block's corner less its scroll
offset. Both walk every slot, and `ChangedSlots::moved` and
`PaintedFrom::shows` then compare every rect and padding.

What Taffy tells us. Taffy writes a box's layout only through
`set_unrounded_layout`, which the box that lays it out calls (its
parent, or for an out-of-flow box its containing block in
`compute_oof_layout`; the root in `compute_root_layout`). It lays a
box out only through `compute_child_layout`, which `BoxTree::dispatch`
sees on a cache miss, and it rewrites a box's hoisted list only in
`compute_oof_layout` for a box it lays out. So the tree records, per
update: the slots dispatched, the slots whose layout
`set_unrounded_layout` changed (compared with what the slot held), and
the slots whose hoisted list was written. A box under a cache hit is
neither laid out nor written, so its layout is the one placed last
time.

Kept between updates, beside the rects and paddings in `PaintedFrom`:
each slot's applied scroll offset, the slot whose content origin it is
placed in (`up`), and each out-of-flow slot's containing block,
patched from the hoisted lists written this update (a rebuilt or freed
slot's entry is cleared first; an out-of-flow box is never lent a
layout, so Taffy lays it out and its containing block writes its
hoisted list again). A freed slot's `up` is cleared; a rebuilt slot
keeps its `up`. A rebuilt subtree whose layout is unchanged is lent
its caches and its ancestors' caches are not cleared, so nothing above
it is laid out; the kept `up` of the subtree's top slot (which keeps
its slot and parent) still marks the chain to the root, and the walk
sets `up` again for every slot below it.

Placement. A slot is dirty when it was dispatched, its layout changed,
it was rebuilt, or its scroll offset was moved since the last update
(`Offsets::scroll` notes the element). Each dirty slot marks the slots
up its `up` chain. A walk from the root visits a slot when it or a slot
below it is marked, or its containing block's content origin moved; it
settles the slot's scroll offset if the slot is dirty, places its rect,
and visits every slot placed in it when its own content origin moved,
else only the marked ones. A scroll so moves only the scroller's
descendants, an edit that moves a row moves the rows placed after it,
and an idle update visits nothing. The walk reports as moved each
visited slot, not rebuilt, whose rect or padding changed; that is
`ChangedSlots::moved`, and `PaintedFrom::shows` becomes "nothing
built, nothing moved, same thumbs and scale". With no kept placement
(the first update, after a failed one, or every box built) every slot
is dirty.

Counts: `boxes_placed` (slots the walk visited) and `boxes_moved`.
The debug check settles and places every box afresh with today's
`settle` and `placed`, and compares the rects, paddings and offsets of
every held slot, each held slot's `up` and containing block with a
fresh walk (`chain::by_containing_block`), so a broken chain fails on
the update that broke it, and the moved set with a full comparison,
naming the first differing slot.

## Open points

- Rebuilding an owner rebuilds its whole subtree: a restyle of a large
  container (`:hover` on a list, a class on `body`) still builds every
  box under it, as today. Rebuilding only the owner's own box and
  runs, reusing unmarked child subtrees, is a later step if a workload
  shows it.
- Moving an element to another parent builds its boxes again in a new
  slot; no reuse across owners.
- Inserting or removing boxes keeps other slots in place, so no index
  shifts; the free list keeps the vector from growing past the largest
  tree.

## Progress

- Plan: this file.
- PR 1a (fdb4feb, on #185 with the plan): `layout/slots.rs` places the
  pre-order boxes `build` makes into slots by key (kept slot, else the
  lowest vacant one, else a new one); `KeptBoxes` keeps the slots, and
  `element` and `lend` read them. Counts and the oracle unchanged;
  harness in `docs/perf/baseline-2026-11-10.md` ("IU3 PR 1a"). For 1b:
  `Slots::place` takes a whole tree; rebuilding an owner's subtree needs
  a variant that places only that subtree's keys and frees only its
  slots.
- #185 review fixes: a key placed twice panics naming it (it would
  otherwise loop in `permute` or index out of bounds); a vacant slot
  reached by `keep`, `stacking`, `clip` or `text_paint` panics naming
  the slot; `Slots::place` debug-asserts that a walk from the root
  reaches exactly the non-vacant slots. Slot tests cover text keys and
  three new-root cases. The map-vs-scan test fails alone when the
  slot-to-key map is wrong; a wrong key-to-slot map breaks every slot
  test, since each reads slots through it; a slot given to two keys
  panics naming both, where it would otherwise hang `permute`. A
  `KeptBoxes::element` test fails alone among the unit tests when
  `element` reads the wrong slot. `update`'s `scrolls` panics on a
  vacant slot (`settle` asks only element boxes); `Chains::of` visits
  every slot by index, so its `None` for a vacant slot stays.
- PR 1b (#186): `layout/marks.rs` marks elements at each edit point
  (attributes, properties, the person's edits, text, moves, removal,
  images, restyles whose computed style changed or holds a calc, the
  focused field's text marks); `KeptBoxes::rebuild` builds again only
  the marked owners' subtrees and lends the rest their caches. Boxes
  built per update: idle 0, one text edit 2, a class toggle 7 (702,
  7002 or 21002 before). Layout at 1000 rows idle 7.52 → 0.77 ms,
  update 18.57 → 11.05 ms; at 3000 rows layout 26.87 → 3.30 ms, update
  65.98 → 39.43 ms (`docs/perf/baseline-2026-11-10.md`, "IU3 PR 1b").
  Oracle cases per mark point: #187.
- PR 3a (#189): `kept_checks.rs` keeps which
  elements the tree checks, the `style` attribute check and the checks
  after styling last refused; `style_checks.rs` checks again only the
  elements box-marked, listener-marked, added or restyled since the
  last update, and walks the whole tree for the refusals, in document
  order, only when one refuses (debug builds walk every update and
  assert the two agree). `Calcs` is kept in `Kept`, per element.
  Idle `elements_visited` 0. A compiler flag for "this element's style
  never changes" would plug in at `Document::rechecked`
  (`style_checks.rs`), dropping flagged elements from the set the
  checks after styling and `Calcs::set` run for.
- PR 3c-1 (perf/iu3-3c-paint): `kept_paint.rs`; `Painted` keeps the
  rects, thumbs and scale it was made from, and an update that built
  no box with the same three shows it; a debug build paints again and
  asserts the two paints are equal (`kept_paint_check.rs`), so every
  debug test checks the reuse. The random oracle also changes the
  scale and the thumbs. Idle `boxes_painted` and
  `scene_items_built` 0; idle paint at 3000 rows 21.96 → 0.05 ms
  (`docs/perf/baseline-2026-11-10.md`, "IU3 PR 3c-1"). For 3c-2: the
  text cache keeps layouts only two updates unused (`keep_previous`
  covers a kept paint), so per-box kept paint must touch its layouts'
  keys or a later repaint of a moved box counts them built again.
