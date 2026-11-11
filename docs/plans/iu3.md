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
| 3c-1 | The last update's `Painted` is kept and used again when no box was built, no box was laid out, and the scroll offsets, scale and thumbs shown are those it was painted with. Each of those four is a path that paints again, with a test that fails without it | Idle `boxes_painted` and `scene_items_built` 0, pinned; display-list build at 3000 rows idle near 0; the oracle and Chromium comparisons unchanged |
| 3c-2 | Paint kept per box (its items with ids local to it, its hit, cursor, texts, images, icons, surfaces, range track and text origin) with the values above it was made from; an update that repaints walks paint order, paints the boxes whose values differ or whose slot was rebuilt, and copies the rest, renumbering ids in paint order so the scene equals a full paint | Text change paints the edited label's boxes, class toggle and hover one row's, pinned in `EXPECTED_WALKS`; each compared value proven by breaking |
| 3c-3 | If 3c-2's measurement shows the walk and copy dominate an edit: paint order, clips and the element maps kept and redone under changed boxes | Edit paint at 3000 rows recorded before and after |

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
- PR 3a (in progress, perf/iu3-3a-style): `kept_checks.rs` keeps which
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
