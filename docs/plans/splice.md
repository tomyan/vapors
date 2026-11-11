# Splice: inserting or removing a row without walking the tree

Property 3 of `docs/plans/perf-finish-line.md`: inserting or removing
a row costs about what an edit costs. Order of work item 5, brought
forward from the build-time analysis (Tom, 2026-11-11). The compiler
marks `{for}` blocks whose rows cannot change paint order; paint
inserts or removes such a row's steps in the kept order in place of
walking the whole tree.

## What an insert costs today

Read from the code at f84812a; not yet measured with an insert
workload (PR 4 below adds one).

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
3. Every row after the inserted one moves; until 3c-3e each moved box
   is painted again, after it each is shifted, O(its items).

Splicing order removes 2. Property 3 also needs 1 narrowed to the
inserted row's boxes; 3 stays O(rows after the insert) even with
3c-3e's shift. The PR stack below includes the narrowing; the
remaining cost of 3 is an open point.

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

A row is the nodes one iteration of a `{for}` body creates. It cannot
change paint order when, in every state it can reach:

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
   that changes 1 or 2 for it: no rule setting a property of 1 or 2
   depends on siblings (`+`, `~`, `:first-child`, `:last-child`,
   `:only-child`, the `-of-type` and `nth-` forms) or on the parent's
   content (`:empty`, `:has()`).

The anchor `{for}` leaves is an empty text node; it collapses and
makes no box.

## (b) How the compiler decides it

For each `{for}` block, over the row template (the block's children,
both branches of each `{if}`, and the bodies of nested `{for}`s):

- Elements: each element's tag; its literal `class` and `id`; and for
  each expression-valued `class`, `id` or other attribute, "unknown".
  States: every element can take every state pseudo-class (`:hover`,
  `:focus`, `:focus-visible`, `:focus-within`, `:active`, `:checked`,
  `:disabled`, `:enabled`, `:invalid`). Today `class={expr}` is a
  Rust string, so its classes are unknown; when classes come from
  enums (`docs/briefs/NATIVE_SVG_BRIEF.md`, "One analysis") the set
  becomes known and more rows are marked.
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
  3 do not exclude it. Combinators to the left of the subject only
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
point 2.

## (c) The runtime contract

- vapors-dom: `Dom::mark_splice_rows(&mut self, anchor: NodeId, site:
  &'static str)`, emitted by the compiler right after a marked
  block's anchor is created, with the block's `file:line:col`. Every
  `Dom` implements it: `RecordingDom` records it, vapors-elements'
  `Document` keeps the anchor and site until the anchor is removed.
  No default method, so a new `Dom` cannot ignore it.
- vapors-elements, on an update whose shape changed: it splices only
  when every inserted or removed run of nodes lies directly before a
  marked anchor, and the check below passes; otherwise it walks the
  whole tree as today.
- The check, both builds, O(the row's boxes): each new slot's
  `Stacking` is `Stacking::default()`; the parent's children differ
  from its kept children by contiguous runs of the row's boxes; no
  other rebuilt slot is restacked or reshaped. It reads the same
  `Stacking` the walk reads, so passing it means the splice is exact.
- In release, a row that fails the check (a global sheet added with
  `Document::add_stylesheet`, a `:global` rule in another component,
  a custom property in a `var()` the compiler saw as setting
  nothing) falls back to the whole walk for that update, and
  `UpdateCounts::splices_refused` counts it with the anchor's site
  kept for devtools. Fallback, not refusal: the whole walk is the
  exact answer, so the user loses nothing, and refusing would take
  away paint because of a speed-up. The count keeps it loud: the
  harness and `EXPECTED_WALKS` pin it at 0 on their workloads.
- In debug, every spliced update also walks the whole order
  (`kept_order_check.rs`) and panics on a difference, naming the
  anchor's site, the row's first slot and the first step that
  differs. A difference after the check passed is a defect in the
  splice or the check, never a sheet.

## (d) PR stack

| PR | Content | Acceptance |
|---|---|---|
| 1 compiler marking | The analysis of (b) in `crates/compiler` (a `splice` module using `vapors-css`'s parser); `Dom::mark_splice_rows` in vapors-dom, `RecordingDom` and `Document` keeping it; the call emitted after a marked anchor | Unit tests over example templates: marked; unmarked for each reason in (b) (positioned, z-index, scrolling, inline top-level, text top-level, `:hover` rule that positions, structural selector, `:global`, expression `style`, component tag); each proven by breaking. Generated code of `compiler-examples` snapshots shows the call |
| 2 narrowed rebuild | An insert or remove before a marked anchor rebuilds the list's own box and the row's subtree, keeping the other rows' boxes in their slots (`layout/marks.rs`, `layout/rebuild.rs`) | An insert at 3000 rows: `boxes_built` the row's boxes plus the list's, pinned; layout unchanged against the oracle |
| 3 order splice (after 3c-3e) | The check and splice of (c) in `kept_order.rs`: steps, clip tree, clips and chains patched for the row's slots; `splices_refused`; the debug comparison | Insert and remove at 3000 rows: `order_walks` and `chain_walks` 0, `splices_refused` 0; a test per check failure (positioned, scroller, inline row, a global sheet that positions the row) showing the whole walk and the count |
| 4 harness | `vapors-perf` insert and remove workloads (a row at the middle and at the top) | Update, layout and paint at 1000, 3000 and 10,000 rows recorded in `docs/perf/` |

PR 1 does not change behaviour: nothing reads the mark until PR 3.

## (e) Open points

1. The runtime check in (c) decides splicing exactly from what
   `ChangedSlots` already compares, without the compiler's mark. The
   mark adds: unmarked blocks skip the check (O(row boxes), small), and
   a build-time list of which lists splice, for the analysis slice to
   extend. Building the runtime check alone first, and the mark with
   the analysis (item 7), is the other order. The lead decides.
2. Rows holding a component or a `{render}`: the build
   (`build::compile_dir`) sees every component, so a whole-build
   analysis can mark them; the per-file compiler cannot.
3. The compiler has no CSS parsing today (`<style>` is kept as text,
   `codegen/shell.rs`). PR 1 makes `vapors-compiler` depend on
   `vapors-css` and adds a subject-compound "can match" (about 100
   lines); the runtime matcher (`vapors_css::selector::matches`) is
   boolean over a concrete element and does not fit unknown classes.
   The UA sheet is read by path from vapors-elements
   (`include_str!`), or moved to vapors-css so both crates name it.
4. Rows after an insert move: after 3c-3e each is shifted, O(its
   items). At 3000 rows with the insert in the middle, about 1500 rows
   shift; whether that fits "about the cost of an edit" is measured in
   PR 4. Parts in scroller-local coordinates (IU4) would remove it.
5. Until 3b reports moved slots, `ChangedSlots::moved` compares every
   rect, O(slots) (0.05 ms at 3000 rows).
