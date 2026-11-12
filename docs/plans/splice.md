# Splice: inserting or removing a row without walking the tree

Property 3 of `docs/plans/perf-finish-line.md`: inserting or removing
a row costs about what an edit costs. Order of work item 5. Runtime
first (Tom, 2026-11-11): an insert or remove builds only the row's
boxes, and paint decides at runtime, from what `ChangedSlots` already
compares, whether the row's steps can be spliced into the kept order
in place of walking the whole tree. The compiler's mark of list
templates moves to the build-time analysis (item 7), section (d).

## What an insert costs today

Read from the code at 1657b27 (IU3 3b merged); not yet measured with
an insert workload (PR C below uses one).

1. `insert_before` and `remove` mark the parent (`layout/marks.rs`), so
   the owner is the list element and `rebuild_owners`
   (`layout/rebuild.rs`) builds every row's boxes again. Every box put
   under the owner is in `Rebuilt::slots`, so every row's slot counts
   as rebuilt and its part is painted again (`kept_box.rs`, 3c-2's
   rule).
2. The list's children differ, so `shape_changed` is true
   (`layout/rebuilt.rs` `reshaped`) and `KeptOrder::updated`
   (`kept_order.rs`) makes order, clip tree, clips and scroll chains
   afresh over the whole tree: about 9 ms at 3000 rows before 3c-3
   (`docs/plans/iu3.md` "What is kept, and what redoes it").
3. Every row after the inserted one moves. Placement (3b) reports
   them as moved; until 3c-3e each moved box is painted again, after
   it each is shifted, O(rows after the insert).

PR A removes 1, PR B removes 2. 3 stays O(rows after the insert) with
3c-3e's shift, and is recorded as known until IU4 (section (e)).

## (a) A row that cannot change paint order

Paint order is `paint_order.rs`: CSS 2 Appendix E reduced to what
vapors lays out. A box enters it through four places:

| Predicate | Where | What it changes |
|---|---|---|
| `Stacking::positioned` (`position` not static, or a z-index on a flex or grid item) | `stacking.rs` `stacking_of_style`, `style/used.rs` `applied_z_index` | the box leaves its parent's flow and is painted with the context's layers (`Walk::collect`, `Walk::layer`) |
| `Stacking::z` and `Stacking::fixed` (`roots_context`) | `stacking.rs` | the box roots a stacking context: its layers and outlines are its own |
| `Stacking::scrolls` (`overflow: scroll`; `hidden` clips only) | `style/used.rs` `Used::scrolls` | a `Step::Thumb`, moved by `thumb_order.rs`, and a scroll chain (`scroll_chains.rs`) |
| `Placement` (`contains_absolute`, `contains_fixed`, `out_of_flow`) | `stacking.rs`, Taffy's `is_containing_block` (absolute: `position` not static; fixed: never in Taffy 0.15) | containing blocks of out-of-flow boxes, and the clip tree (`clip_tree.rs`) |

Outlines are not a predicate: every box has a `Step::Outline`
(`kept_order.rs` module doc), pushed into the innermost open frame
(`Walk::boxed`) and flushed at the frame's end.

A row is the nodes one iteration of a `{for}` body creates. Inserting
or removing it on an update cannot change the order of the other
boxes' steps when, on that update:

1. Every box in it has `Stacking::default()`: not positioned, no
   z-index, not fixed, not scrolling, default `Placement`. Then its
   boxes are in-flow descendants of the enclosing frame, painted by
   `Walk::flow` in tree order: its `Step::Box` steps are one
   contiguous run, after the previous row's and before the next
   row's, and its `Step::Outline` steps are one contiguous run in the
   enclosing frame's outlines, in the same order. The splice inserts
   or removes those two runs.
2. Each top-level node of the row makes a box of its own or none: an
   element whose `display` is never `inline` or `contents`. An inline
   top-level node joins its siblings' inline content in one anonymous
   box (`layout/boxes.rs`), which inserting the row changes. Text at
   the top level is the same case; whitespace-only text collapses and
   is allowed.
3. Inserting or removing the row restyles no other element in a way
   that changes 1 or 2 for it. Rules that depend on siblings (`+`,
   `~`, `:first-child`, `:last-child`, `:only-child`, the `-of-type`
   and `nth-` forms) or on the parent's content (`:empty`, `:has()`)
   can; at runtime such a restyle rebuilds the element, and a changed
   `Stacking` shows in `ChangedSlots` (section (c), condition 3).

At runtime these are read per update from the boxes built (section
(c)); the compiler's mark (section (d)) would prove them for every
state a row can reach.

The anchor `{for}` leaves is an empty text node; it collapses and
makes no box.

## (b) PR A: the narrowed rebuild

`insert_before` and `remove` mark the parent's children
(`Marks::mark_children`) in place of the parent's box. The rest of the
marks are as today (`Marks::mark`).

- A children mark on an element that holds a slot and makes a box of
  its own makes it a shallow owner. Otherwise it resolves as a box
  mark on the element does today (an inline or `display: none`
  parent's owner is the nearest block above it).
- A shallow owner's update builds its own box and its inline runs
  again. Each element child whose box was a child of the owner's kept
  box, and which still makes a box, keeps its slots and its kept
  subtree untouched: not taken, not built, not lent, its paint kept
  (`LayoutBox::kept`). Every other child is built in full: an inserted
  row, a child moved in from another parent. The old children no new
  box keeps (a removed row, a child moved away, the old runs) are
  taken and their slots freed.
- A box mark resolves to its owner as today. When that owner is a
  shallow owner and the mark is on a node below it, the node is in a
  child the shallow build builds in full or in the owner's runs, so
  the mark is covered: an inserted row's restyle marks and the
  compiler's `set_text` calls on its nodes do not widen the build.
- A box-mark owner covers the owners inside it, shallow ones too, as
  today. A shallow owner covers only what it builds: an owner inside
  a child it keeps is rebuilt on its own; one inside a child it
  builds in full is dropped.
- Siblings whose style depends on position (`+`, `~`,
  `:first-child`, `:nth-child`, ...) are restyled by the restyle pass
  (`Document::mark_children_changed` dirties the parent's subtree when
  the sheets hold sibling selectors), and the pass marks each element
  whose computed style changed. Those siblings are rebuilt as box-mark
  owners; the others stay kept.
- Debug builds compare every update that built by owners with a build
  of the whole tree from the root: the same keys in the same slots and
  order, and each box's style, content and natural size. A kept box
  that the fresh build makes differently (a sibling wrongly kept)
  panics naming the slot and its key.

### (b2) PR A2: the restyle narrowed on insert and remove

Today `Document::mark_children_changed` dirties the parent's whole
subtree whenever any sheet holds a sibling combinator or a positional
pseudo-class (`Features::siblings`, one flag): O(rows) restyles per
insert at 3000 rows, against finish-line property 3. In its place the
sheets keep, once per sheet added (`style/sibling_deps.rs`), which
siblings of a changed child a selector can reach, and the mark dirties
only those:

- The inserted child's subtree, always. A removed child marks nothing
  of its own.
- A text node inserted or removed moves no element among its siblings:
  only `:empty` (as today, the parent's subtree) applies.
- From each selector, the compounds joined to each other by `+` or `~`
  form a group; a group's rightmost compound is matched against one of
  the parent's children. The group reaches, from the changed child
  (the inserted one in place, or the removed one before it is
  detached): the next `k` element siblings for `k` `+` combinators,
  one more for `:first-child` or `:only-child`; the previous sibling
  for `:last-child` or `:only-child`; every later sibling for `~`,
  `:nth-child()`, `:nth-of-type()`, `:first-of-type` and
  `:only-of-type`; every earlier sibling for `:nth-last-child()`,
  `:nth-last-of-type()`, `:last-of-type` and `:only-of-type`.
  `previous` with a `~` in the group widens to every later sibling;
  `earlier` with any sibling combinator widens to every child. A
  `:not()`, `:is()`, `:where()` or `:global()` whose selector is one
  compound counts as that compound's pseudo-classes; one with
  combinators cannot be placed and widens to every child.
- A group that is not the selector's last restyles each reached
  sibling's subtree (`Dirty::subtree`); otherwise the sibling alone,
  and its children when its own style changes (`Dirty::element`), which
  covers what they inherit.
- `:has()` restyles everything, as today. The reach is an
  over-approximation, never under: what it cannot bound widens.

Debug builds, after an update whose restyle was narrowed this way,
compute every element's style afresh from the root and assert each
equals the kept one, naming the first element that differs, the
property, and the sheets' structural selectors that match it.

Acceptance: `elements_restyled` after a middle insert, each proven by
breaking: no structural rule → the new row's elements only; `+` → and
the next sibling; `:nth-child(odd)` → and the siblings after;
`:last-child` → the old and new last; `~` → the following siblings;
`:has()` → everything; a sheet added between inserts changes the reach.
The count for a middle insert at 3000 rows, with and without a
structural rule: 4 (none), 5 (`.row + .row`), 6004
(`.row:nth-child(odd)`), from one manual run of
`assert_middle_insert_counts(3000)` (`insert_restyle_rows.rs`) on
2026-11-11; the test itself runs 1000 rows only, as 3000 takes 22 s in
debug.

## (c) PR B: the order splice, decided at runtime

On an update whose shape changed and that is not `every`, paint
splices in place of walking the whole tree when all of these hold,
each read from what `ChangedSlots` and `Stackings` already hold:

1. Every slot whose children differ from its kept children is a
   shallow owner whose kept children kept their order: its new
   children are its kept children with runs of new slots inserted
   and runs of freed slots removed.
2. Every newly taken slot, and each slot in a removed run, has
   `Stacking::default()` (not positioned, no z-index, not fixed, not
   scrolling, default `Placement`), and so does every slot below it.
   Section (a) is why this is enough for the steps.
3. No rebuilt slot that was kept has a `Stacking` different from its
   kept one (`ChangedSlots::restacked` false), and the owner's own
   `Stacking` is unchanged.

Then each inserted run's `Step::Box` steps go into the kept order
directly after the previous sibling's last step (or the owner's own
step), and its `Step::Outline` steps into the enclosing frame's
outlines at the matching place; a removed run's steps are taken out.
The clip tree gives each new slot the owner's clip parent; clips are
redone for the new slots as for rebuilt ones today. Scroll chains:
new slots join the owner's chain (none scroll, by 2). Locating the
previous sibling's steps needs a slot-to-step index, kept and patched
with the order; it is O(steps) to build once and O(1) per lookup.

When the conditions fail the update walks the whole tree as today,
which is the exact answer, so nothing is refused. `UpdateCounts` gains
`order_splices` and `splices_declined`; the insert workload pins
`order_walks` 0 and `order_splices` 1, so a splice that silently stops
applying shows in the counts.

Debug builds compare every spliced update with the whole walk
(`kept_order_check.rs`, which already makes order, clip tree, clips
and chains afresh on every update that paints) and panic naming the
first differing step and its slot. A difference is a defect in the
splice or the conditions.

## PR stack

| PR | Content | Acceptance |
|---|---|---|
| A narrowed rebuild | Section (b): `Marks::mark_children`, shallow owners in `layout/marks.rs` and `layout/rebuild.rs`, the build stopping at kept children (`layout/boxes.rs`), `Lender` skipping kept boxes, the debug check against a fresh build | Insert in the middle of a list: `boxes_built` the row's boxes plus the list's own, pinned; remove frees only the removed row's slots; a sibling restyled by `:nth-child`, `+` or `:first-child` after an insert is rebuilt; keyed `{for}` reorder unchanged against the oracle; each proven by breaking. `boxes_built` for a middle insert at 3000 rows before and after |
| B runtime splice | Section (c) in `kept_order.rs` and `changed_slots.rs`: the conditions, the step splice, clip tree, clips and chains for the new slots, the slot-to-step index, the counts, the debug comparison | Insert and remove at 3000 rows: `order_walks` and `chain_walks` 0, `order_splices` 1; one test per declined condition (a positioned row, a scrolling row, a z-index on a row, an owner whose kept children were reordered) showing the whole walk and `splices_declined` 1 |
| C acceptance | Insert and remove at 1000, 3000 and 10,000 rows on `vapors-perf`'s `RowChange::Insert` workload (impl-scaling), a row at the middle and at the top; instruction counts and times recorded in `docs/perf/` | Update, layout and paint within the edit's at 3000 rows except the moved rows' cost (section (e)), recorded with their count |

## (d) Later: the compiler's mark (build-time analysis, item 7)

A compiler mark on `{for}` blocks whose rows cannot change paint
order is not in this stack. The runtime conditions in (c) decide the
same question exactly, from values paint already compares, at O(the
row's boxes) per update. The mark would add a build-time list of which
lists splice, which the analysis can extend and report, and would let
unmarked lists skip the check; neither is needed for property 3. It
moves to the build-time analysis, where the template and sheet
analysis below is wanted anyway (properties 7 and 8).

### How the compiler would decide it

For each `{for}` block, over the row template (the block's children,
both branches of each `{if}`, and the bodies of nested `{for}`s):

- Elements: each element's tag; its literal `class` and `id`; and for
  each expression-valued `id` or other attribute, "unknown". An element's
  classes are known: the literal `class` names plus each `class:name`
  directive's name (`class={expr}` is refused).
  States: every element can take every state pseudo-class (`:hover`,
  `:focus`, `:focus-visible`, `:focus-within`, `:active`, `:checked`,
  `:disabled`, `:enabled`, `:invalid`).
- Sheets: the user-agent sheet (`elements/src/style/ua.css`) and the
  component's own `<style>`, as the runtime parses them
  (`vapors_css::parse`). A component compiled with `Styles::Global`,
  or holding a `:global(...)` compound, has its rows unmarked: its
  rules reach elements the compiler does not see in this file.
- Rules that matter: a declaration of `position`, `z-index`,
  `overflow`, `overflow-x`, `overflow-y` or `all` anywhere, or of
  `display` on a top-level element; except `position: static`,
  `z-index: auto`, `overflow*: visible | hidden | clip`, and `display`
  of `block | flex | grid | none`. Any other value, `var()`, `inherit`
  or a value the compiler does not read, counts as setting it.
- A rule that matters can match a row element when its subject
  compound can: its tag is absent or equal, each required class is
  literal on the element or the element's class is unknown, and
  likewise for id and attributes; pseudo-classes other than those of
  (a)3 do not exclude it. Combinators to the left of the subject only
  narrow a match, so they are not evaluated: the compound alone is an
  over-approximation, never an under-approximation.
- Unmarked when: a rule that matters can match a row element; a
  matching rule's selector uses any form listed in (a)3, anywhere in
  the selector; a top-level node is text with non-whitespace content,
  an inline element by the UA sheet (`span`, `b`, ...), an
  inline-block one (`button`, `input`), a component tag or a
  `{render}`; or a row element has an expression `style` attribute,
  or a literal one setting a property that matters.

A component tag or `{render}` anywhere in the row unmarks it in the
first version: the child's template is another file or snippet. Open
point 4.

## (e) Open points

1. Rows after an insert move: after 3c-3e each is shifted, O(rows
   after the insert); at 3000 rows with the insert in the middle,
   about 1500 rows. Known until IU4, whose parts in scroller-local
   coordinates remove it. PR C records the cost with its count.
2. Rebuilding an owner's whole subtree for a box mark stays as today
   (`docs/plans/iu3.md`, "Rebuilding an owner rebuilds its whole
   subtree"); PR A narrows only children marks.
3. A keyed `{for}` reorder moves kept children within their owner: PR
   A keeps their subtrees, and PR B declines to splice (condition 1),
   so the order is walked as today.
4. Rows holding a component or a `{render}` splice at runtime like any
   other row; only the build-time mark (d) cannot see into them from
   one file.
