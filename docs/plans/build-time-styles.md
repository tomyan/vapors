# Build-time styles: the route to finish-line property 7

Item 7 of `docs/plans/perf-finish-line.md`: the build-time analysis,
first version, for property 7 ("First frame of a large page: the
winning declarations for each template element and constant shapes
worked out at build time (computed values still depend on viewport,
scale and font size); time measured and published") and the style
part of property 8, with the compiler's mark of list templates from
`docs/plans/splice.md` section (d).

Written 2026-11-12 from `origin/main` at 7d1fe84 (analysis PRs 2a to
2d merged; #240, the styles line in `vapors-perf`, open). Every number
names its source; a number this document does not have says "to be
measured".

## Summary

- After analysis PR 2 the cascade runs once per share key, not per
  element: 4 times on a 10,000-row Mount page against 40,002 restyled
  elements (`docs/perf/mount-shared-2026-11-12.md`, #240). Working out
  winning declarations at build time therefore removes almost nothing
  from that first frame: it replaces 4 cascades.
- Style is still 3.44 G instructions at 10,000 rows, 86,000 per
  restyled element (section 3, measured at 7d1fe84). A sampled profile
  puts 97% of the restyle in `style::used`, the per-element `Used`
  derivation, and most of that in property lookups by name. That is
  the first-frame lever for style, and it is runtime work: share `Used`
  the way PR 2 shares `ComputedStyle`, and read properties by index.
- The build-time part still has a job: it is what property 7 names, it
  moves the cascade's decisions (which rules can match, which
  declarations win, whether a key can share) to where a mistake fails
  the build, and it is the analysis the splice mark (section 4) and
  property 9's scene-node property sets reuse. Its gain shows where
  share-table misses are many: pages with many distinct keys, and
  edits that change a share key (a class toggle).
- Compiled winners refer to rules by sheet identity and position within
  the sheet, each sheet checked by its own hash, and the runtime ranks
  rules from different sheets by their real order, so the compiled
  answer does not depend on which components mounted first (section 2).
- The PR stack (section 6) has 13 slices under about 400 lines each:
  two runtime slices for `Used`, one for the text cache, two for
  refusing Rust-added sheets beside analysed elements, five compiler
  and runtime slices for build-time winners, the splice mark, and two
  measurement slices. Section 7 records Tom's five decisions.

## 1. What the compiler can resolve per template element

### What it knows

Since the analysable-sources decision
(`docs/decisions/2026-11-11-analysable-sources.md`) and analysis PRs
2a and 2b, the compiler emits a `StyleKey` per template element
(`crates/compiler/src/codegen/style_key.rs`): site, scope, tag,
literal classes, the names of its `class:` directives and its literal
`style` text. The runtime refuses any later change to `class`, `style`
or the scope attribute on a keyed element (`crates/elements/src/
dom_keyed.rs`). So for each element the compiler knows:

- **Tag, scope, literal classes, literal `style`:** fixed.
- **Toggled classes:** each `class:name={bool}` is on or off; the
  name is known, the value is a run-time input. At most 64 per element
  (`MAX_TOGGLES`). The enum form (`class={variant}` over a closed set)
  is not built; when it is, it is a group of toggles with one on.
- **Other attributes:** a literal attribute's value is known; an
  expression attribute (`id={expr}`, `hidden={expr}`) has a known name
  and a run-time value.
- **States:** every element can take every state pseudo-class the
  matcher tests (`:hover`, `:focus`, `:focus-visible`,
  `:focus-within`, `:active`, `:checked`, `:disabled`, `:enabled`,
  `:invalid`). The compiler treats each as a run-time input; it does
  not try to prove a state unreachable for a tag.
- **Ancestors inside the template:** known elements with the same
  facts. Ancestors above the component's root are whatever the mount
  site gives, which the compiler does not see from one file.
- **Sheets:** the user-agent sheet (`crates/css/src/ua.css`),
  the component's own `<style>` scoped as the runtime scopes it
  (`crates/css/src/scope.rs`), and the sheets of the other
  files `compile_dir` compiles in the same crate (`build/files.rs`),
  which carry the crate's `:global(...)` compounds and its
  `Styles::Global` sheets. It does not see sheets added from Rust with
  `Document::add_stylesheet` (5 non-test call sites today: examples,
  `scroll_fixture.rs`, the elements corpus), nor other crates' sheets.
- **Custom properties:** a `var()` in a winning declaration names a
  custom property whose value comes from the element's own winners or
  its ancestors. No template form binds a custom property today; the
  decision record allows only a binding with a literal name, so when
  one is added its name is known.

### What it resolves

For each template element the compiler takes the candidate rules from
`vapors-css` itself (`Cascade::rules_for_key`, the bucket filter on
tag, literal classes and directive names) and classifies each rule
against the element as one of:

- **never:** the subject compound requires something the key rules
  out: another tag, a class that is neither literal nor a directive,
  another scope, a literal attribute with another value.
- **always:** the subject compound is satisfied by the key's fixed
  parts alone (tag, scope, literal classes, literal attributes), the
  selector has no other compound, and the rule has no `@media`,
  `@supports` or `@container` condition.
- **conditional:** anything else. The compiler records what the match
  depends on: which directives, which state flags, which expression
  attributes, whether ancestor compounds are involved, which
  conditions. The runtime tests the rule with the same matcher it uses
  today.

Only the key's own sheet (its scope) and the UA sheet supply
**always** rules. A rule from any other sheet that can reach the key
(a `:global(...)` compound, a `Styles::Global` sheet) is classified
**conditional** whatever its selector: its rank against the key's own
rules can tie on specificity, and a tie is decided by
`CascadeRule::order`, which depends on the order sheets were added at
run time (`cascade/mod.rs` `add`; section 2). UA and author
declarations never tie: their ranks differ (`collect.rs` `rank`).

Then two facts per longhand, kept apart:

- **Constant winner:** the highest-standing declaration among the
  **always** rules and the literal `style` wins, and no conditional
  rule declares the longhand with a higher standing or an equal one
  (origin and importance, specificity, order: the same
  `Candidate::beats` the runtime uses, `cascade/collect.rs`). Otherwise
  the winner is **known variable**, and the compiler has the list of
  conditional rules that can override it.
- **Constant value:** a constant winner whose value needs no context:
  not `inherit`, `unset` or `revert`, no `var()`,
  no `em`, `rem`, `%`, viewport unit or `currentcolor`, no
  `light-dark()`. A longhand with no winner at all is a constant value
  when it does not inherit; an inherited one is a constant value only
  when the parent's is, which the compiler can say for parents inside
  the template and not for a component's root.

The same holds for custom properties (`--name`): each has a winner,
classified the same way; its value is never constant when it holds a
`var()`.

These are the brief's two states (`docs/briefs/NATIVE_SVG_BRIEF.md`,
"One analysis for every painted thing"), applied to winners and to
values separately: a constant winner can still have a variable value
(`width: 2em` under a parent whose font size changes). Readers that
need "this cannot change at run time" (the splice mark, IU3 and IU4
skipping work, the shape table) read **constant value**; the cascade
shortcut in section 2 needs only **constant winner**. There is no
third state: a rule the compiler cannot classify more narrowly is
conditional, which is exact because the runtime tests it.

The compiler also decides each key's shareability (no candidate rule
reads siblings or descendants or sits under `@container`,
`vapors_css::cascade::unshareable`), which the runtime decides today
once per key per `Kept` (`restyle_share.rs` `shareable`).

All of this calls `vapors-css` (parser, scoping, bucket filter,
matcher pieces, `unshareable`, `Candidate` ranking). This plan adopts
the rule from the lead's analysis handoff of 2026-11-11 (not in the
repository): "the compiler must use `vapors-css` for any selector or
value reasoning, never a second matcher", so the compiler and the
runtime cannot disagree about what a selector means. Two pieces lived
outside `vapors-css` and moved into it in B0: the scoping, with its
refusals (`vapors_css::scope`), and the UA sheet text
(`vapors_css::user_agent`).

### What stays at run time, and why

- **Computed values.** `cascade/compute.rs` resolves `em`, `rem`,
  `vw`/`vh`, percentages of `line-height`, `currentcolor` and colour
  schemes against the parent's computed style and the `Environment`
  (viewport, root font size, colour scheme). These change without a
  rebuild.
- **Conditional rules.** Toggles, states and expression attributes are
  run-time values; ancestors above the root depend on the mount site;
  `@media` and `@supports` read the environment.
- **`var()` values.** Substitution reads the custom properties in scope,
  which inherit from ancestors.
- **Sharing.** Which elements share a computed style depends on their
  parents' styles and run-time states (the share key in
  `crates/elements/src/style/share.rs`).
- **Cross-sheet ranking.** Rules from other sheets that reach a key,
  ranked by the order the sheets were added at run time (section 2).
- **Sheets the compiler did not see.** Another crate's compiled sheets
  (`crates/components` is its own `compile_dir`) are seen at run time
  as other sheets. A sheet added from Rust is refused on a document
  with analysed elements (section 7, decision 2; slices R1a and R1b).
- **Layout, text shaping, paint.** Text, fonts, viewport and images.

## 2. What the compiler emits and how the runtime uses it

Building on `StyleKey` and the share table; replacing neither.

### How the runtime holds sheets, and what that rules out

The runtime adds the UA sheet when a document is made, a Rust sheet
when `add_stylesheet` is called, and a component's scoped sheet the
first time that component mounts (`dom_impl.rs` `add_style`,
`Sheets::add_scoped`), so in mount order. Each rule's
`CascadeRule::order` is its place in that combined sequence
(`cascade/mod.rs` `add`), and `Candidate::beats` breaks a tie of rank
and specificity on it. So the document's sheet list, and the order of
rules from different sheets, depend on which components a page has
mounted and in what order. A compiled answer keyed to one global list
(one hash over all the crate's sheets, rule indices into the combined
cascade) would match only on a page that mounted every component of
one crate in build order: the Mount workload, and not the racer, any
demo page or an app using `crates/components`.

The design below fixes each referenced sheet's identity and contents at
build time and leaves the ranking of rules from different sheets to
the runtime, which knows their real order.

### Emitted

`StyleKey` gains one field, `winners: Option<&'static KeyWinners>`
(`None` from hand-written keys in tests):

```rust
pub struct KeyWinners {
    /// The sheets the references below point into, each by identity
    /// (`SheetId::UserAgent` or `SheetId::Scope(scope)`) with a hash of
    /// its text and the vapors-css version.
    pub sheets: &'static [(SheetId, u64)],
    /// The always rules' winning declarations: per longhand, and per
    /// custom property by name, (sheet, rule index within that sheet,
    /// declaration index); the literal `style` as its own entry.
    pub base: &'static [Winner],
    /// The conditional rules, as (sheet, rule index within that
    /// sheet): the key's own conditional rules and every rule of
    /// another sheet in the crate that can reach the key.
    pub conditional: &'static [(u8, u32)],
    /// No candidate rule blocks sharing.
    pub shareable: bool,
}
```

Rules are referenced by sheet and position within the sheet, not
copied as declaration data, so the generated code does not depend on
`vapors-css`'s value types and stays small, and no reference depends
on the order sheets were added.

### Used at run time

Each time the document's sheets change (a new `Kept`, as today), the
runtime decides per key, once, on the key's first miss:

1. **Referenced sheets equal:** each `(SheetId, hash)` in `sheets`
   that names a registered sheet must match that sheet's hash. A
   mismatch refuses the element under the update-error convention
   (`ElementError::Refused`, as `dom_keyed.rs` does): every update
   returns the refusal while the element is in the tree, naming the
   key's site, the sheet, both hashes and the fix (rebuild the crate
   that registered the sheet). It runs in every build; there is no
   fallback path. A referenced sheet that is not registered (its
   component has not mounted) is not a mismatch: its rules are absent
   from the document, so the runtime cascade would not apply them
   either, and the compiled path skips them. The key's own sheet and
   the UA sheet are always registered before its elements exist
   (`add_style` runs before the component builds its tree).
2. **Other reaching sheets:** the registered sheets the key does not
   reference (another crate's sheets) whose rules are candidates for
   the key (`rules_for_key`). Their candidate rules join the key's
   conditional list for this `Kept`, counted per key in
   `winners_foreign`.

On a share-table miss (`restyle_share.rs` `shared_style`), a keyed
element builds its winners from the `base`
declarations, each given its real runtime standing (rank, specificity
and the `CascadeRule::order` of the rule it points at), and then lets
the conditional and other-sheet rules that match the element compete,
with the same `Candidate::beats`. That is the full candidate scan
(`cascade/collect.rs` `winners`) minus the always rules' matching. The
result does not depend on mount order: every cross-sheet tie is
decided by the runtime's real order. `compute::style` is unchanged:
computed values come from those winners and the environment as today.
`shareable` is read from the key, and is false when an other-sheet
rule blocks sharing (`unshareable`).

On a hit nothing changes: the shared `Rc<ComputedStyle>` is taken.
Unkeyed elements (hand-built pages, tests) take today's path.

### Why a compiled winner cannot differ from the runtime's

The compiled path decides less than it looks. The compiler makes two
judgements per rule and key, and nothing else:

- **never:** the rule's subject compound cannot match any element of
  the key, so it is left out;
- **always:** the subject compound matches every element of the key,
  so its declarations are entered without running the matcher.

Both are judged only on the key's fixed parts (tag, scope, literal
classes, literal attributes), which the runtime refuses to change on a
keyed element (`dom_keyed.rs`), by `vapors-css`'s own matching code
over those parts. Everything else, including every ranking, happens
at run time with the runtime's standing and `Candidate::beats` over
the document's real rule order, as today. Rules whose match depends on
anything else (directives, states, expression attributes, ancestors,
`@media`, another sheet) are tested by the runtime matcher on the real
element.

So a compiled winner differs from the runtime's only through (i) a
sheet whose text is not what the compiler read, which check 1 refuses
in every build, or (ii) a defect in the never/always judgement code.
(ii) is guarded twice: the debug oracle compares every element on
every update, over every test and demo in `cargo xtask ci`; and
release builds verify each key's first miss per `Kept` (below), which
turns a defect into a refusal, never into a silently wrong render.

### Loud checks

- **Winners oracle, debug builds only:** every element that took
  compiled winners also runs the full candidate scan, and the two
  `Winners` (longhands and custom properties) must be equal; a
  difference panics naming the key's site, the property, both
  declarations and the fix (the shape of `check_shared` in
  `restyle_share.rs`).
- **First-miss verification, every build:** the first share-table miss
  of each template key in each `Kept` also runs the full candidate scan
  and compares. A difference refuses the element under the update-error
  convention, naming the site, the property, both declarations and the
  fix. Cost: one extra cascade per distinct template key per sheet
  change, not per element: 4 on a Mount page, about 0.4 M instructions
  (section 3's estimate of 106,000 per computed style). A defect that
  shows only for a toggle or state combination the first miss did not
  have is caught by the debug oracle, not by this check; the never and
  always judgements do not read toggles or states (above), which is
  why one combination per key exercises them.
- **Sheet hashes, every build:** check 1.
- **Shareability, debug builds only:** today's per-key decision runs
  as well and must agree with the key's.
- **Counts:** `UpdateCounts` gains `winners_compiled`,
  `winners_runtime` (keys with no winners: hand-written keys in tests)
  and `winners_foreign` (keys with other-sheet rules added). The Mount
  workload, the racer and one demo page pin `winners_runtime` 0, so
  generated code that stops emitting winners shows. The counts report;
  the checks above are what refuse.
- **Build failures:** a component `<style>` that `vapors-css` cannot
  parse, or that holds a `:global()` the runtime refuses, fails the
  build naming `file:line:column`, where today it fails at the first
  update.

## 3. The remaining per-element style cost

### Measured on main

`cargo run --release -p vapors-perf -- 3 --mount` at 7d1fe84 (load
average 12.8 at the start; instruction counts do not move with load,
`mount-2026-11-12.md`):

| phase | 1000 rows | 3000 rows | 10000 rows | per extra row, 3000 to 10000 |
|---|---|---|---|---|
| style | 345,136,734 | 1,031,401,128 | 3,439,269,005 | 343,981 |
| layout | 239,573,769 | 736,072,146 | 2,484,873,775 | 249,829 |
| paint | 29,917,311 | 88,452,830 | 295,937,679 | 29,641 |
| mount | 29,508,461 | 86,184,579 | 291,761,810 | 29,368 |
| frame (CPU) | 636,659,844 | 1,879,241,846 | 6,247,656,526 | 624,059 |

Style per restyled element: 343,981 / 4 = 86,000, as #240 reports
(86,063). Anonymous memory after the first frame: 49.3 KiB per extra
row from 3000 to 10,000 rows (#240: 47.9).

So the handoff's figures hold on current main: style about 86,000
instructions per element, layout about 250,000 per row (246,000 in
the handoff; 249,829 here).

### Where the 86,000 go

One sampled profile of one 10,000-row page, mount and first frame
(`perf record -e instructions:u --call-graph dwarf` on
`vapors-perf --mount-memory 10000`, 8,846 samples; shares are of the
whole process, teardown included):

| function (inclusive) | share of samples |
|---|---|
| `Document::restyle_one` (the restyle) | 45.4% |
| `style::used::used` (the `Used` derivation) | 44.1% |
| `PropertyId::from_name` (binary search over names) | 27.0% |
| `ComputedStyle::get_by_name` | 19.0% |
| `style::decoration::decoration` | 18.6% |
| `style::box_style::box_style` | 13.0% |
| `alloc::fmt::format` | 8.6% |
| `style::honoured::check_honoured` (closure) | 7.2% |
| `BoxTree::lay_out` (layout) | 32.7% |
| `TextCache::layout` (text shaping, inside layout) | 28.1% |

`used` is 97% of `restyle_one` (44.1 / 45.4). The share key, the table
lookup, the kept maps and the 4 cascades are the other 3% together;
they are inlined and not split further. Inside `used`, every property
read goes through `Reader::value(&'static str)` →
`ComputedStyle::get_by_name` → `PropertyId::from_name`, a binary search
with string compares over about 104 names (`style/values.rs`,
`property/mod.rs`); `decoration::side_property` builds each border
property name with `format!` and looks it up again
(`style/decoration.rs`); `check_honoured` walks the whole property
table per element and looks each deferred property up by name
(`style/honoured.rs`). The shares overlap (inclusive), so they do not
add up; how much each fix removes is to be measured.

### What each step removes

| step | removes | expected on Mount |
|---|---|---|
| U1: properties read by index (`PropertyId` constants; no `format!`; `check_honoured` over precomputed ids) | the by-name lookups and string building, for every element whose `Used` is derived | to be measured; the lookups are the largest part of `used` in the profile |
| U2: `Used` shared per (shared computed style, parent's `Used`, context) | the `Used` derivation for every share-table hit | style per element from 86,000 toward the share key's cost (the 3% above, about 2,600 per element, an estimate from the shares); to be measured |
| B0 to B3: build-time winners | the candidate scan on a share-table miss | about 4 cascades per Mount page: under 0.5 M of 3.44 G instructions (an estimate: about 106,000 per computed style, from 189,000 per element before sharing less the 83,000 of `used`) |

So on Mount, U2 is the step that moves property 7's time; U1 helps
every element that does not hit (unkeyed pages, edits). The `Class`
edit (`vapors-perf 60 10 --scaling Class` at 7d1fe84, the hand-built
rows page, unkeyed) spends 851,594 of its 990,175 update instructions
in style at 1000 rows; U1 is the step that reaches it, and build-time
winners do not, because the page has no keys.

Layout is now the second phase at 250,000 per row, and 86% of it
(28.1 / 32.7 of the samples) is text shaping in `TextCache::layout`.
The text cache key holds each span's `NodeId` (`SpanKey::node`,
`layout/text_key.rs`), so 10,000 identical "Edit" buttons are shaped
10,000 times. A cache keyed by content is runtime work outside the
build-time analysis; it is slice T1 in section 6. On Mount only the
button's text repeats (labels are `Row {i}`, `crates/perf/src/mount.rs`),
so T1 removes at most the button's share of shaping there; how much
is to be measured, and pages with repeated labels gain more.

## 4. The list-template paint-order mark

`docs/plans/splice.md` section (d) specifies it: for each `{for}`, over
the row template (both branches of each `{if}`, nested `{for}`
bodies), the compiler decides whether any rule that matters (`position`,
`z-index`, `overflow*`, `all`, `display` on a top-level element, with
the listed safe values excepted) can match a row element, whether a
matching selector uses a sibling or parent-content form, and whether a
top-level node is inline, inline-block, non-whitespace text, a
component tag or a `{render}`. Unmarked when any holds.

With section 1's classification the "can match" test is the
never/always/conditional result for each row element: a rule that
matters and is not **never** for some row element unmarks the list.
That is the over-approximation (d) asks for: conditional counts as
matching.

What the mark does at run time: the runtime splice conditions
(`splice.md` (c), built in PR B) stay authoritative and decide every
update exactly. The mark adds:

- **A build-time report:** `wisp-check` lists each unmarked `{for}`
  with the reason and the rule's `file:line`, so an author learns at
  build time that inserting into that list walks the whole paint order.
- **A loud check:** the generated `keyed(..)` call passes the mark; the
  runtime keeps it on the list's owner. The mark is computed from the
  crate's sheets only, so the runtime keeps it only while no other
  sheet (another crate's, section 2 check 2) has a rule that matters
  among the row keys' candidates; otherwise it drops the mark for that
  `Kept`, counted in `splice_marks_dropped`, and does not panic. A mark
  that is kept and whose owner's update declines to splice for a
  stacking reason (`SpliceDecline::InsertedStacked`, `FreedStacked`,
  `Restacked`) means the analysis was wrong: debug builds panic naming
  the `{for}` site and the box; release builds count
  `splice_mark_contradicted`. Nothing renders wrong in release either
  way: the mark decides nothing at run time, the runtime conditions do.
- **Skipping the check for unmarked lists:** possible, but the runtime
  check is O(the row's boxes) per update and splice.md (d) already says
  property 3 does not need it. Not built in this plan.

## 5. Measuring and publishing property 7; property 8 after #240

### Property 7

The Mount workload (`crates/perf`, `--mount`) already measures the
first frame of a compiled `{for}` at 1,000, 3,000 and 10,000 rows, per
phase, in instructions and times. What is missing:

- **Times on a quiet machine.** The finish line's method is a release
  build, 1-minute load under 1.5, three runs, medians
  (`perf-finish-line.md`). Both Mount documents keep no times because
  the load was 14 to 36, and the dev machine runs at 12 to 36 while
  agents build. M1's quiet window is made by the lead: no new builds,
  gates or tests start; running ones finish; the run starts when the
  1-minute load is under 1.5. Three runs of `vapors-perf 9 --mount`
  take about 5 minutes (one 3-page run took 36 s here), so the window
  is about 10 minutes with the wait for the load to fall. Property 7
  holds when its row in the finish-line table shows the times from
  such a run.
- **The build-time part shown by the harness.** The Mount report gains
  `winners: compiled N, runtime N, foreign N` beside #240's styles
  line, with `winners_runtime` 0 asserted in `gpu_mount_frames` and
  `mount_styles_tests.rs`, and in tests mounting the racer and one
  demo page.
- **Published:** `docs/perf/mount-<date>.md` with the run's commit,
  load, hardware and both tables, and the finish-line table's property
  7 row updated to cite it.

Property 7 has no time target; "measured and published" is the bar.
This plan's slices are ordered so the published time is the one after
U2.

### Property 8 after #240

#240 shows `ComputedStyle` allocations flat in rows (6 live at 1,000,
3,000 and 10,000 rows) and leaves 48 KiB per row of anonymous
high-water unattributed. The decision record states property 8 as
"one style record per distinct style; total memory per element bounded
by the per-box layout records". Still needed:

1. **Live bytes, not high-water.** M2 gives `vapors-perf` a counting
   global allocator (live bytes and peak, no profiler needed; none is
   installed on the dev machine). Live bytes after the first frame,
   less the 0-row page's, divided by elements, is the per-element
   total. `RssAnon` after `malloc_trim` is printed beside it as a
   cross-check; the two must agree within 10% or the report fails
   naming both, since a gap that size means the counter misses an
   allocator path.
2. **Attribution by dropping.** After the first frame, the report drops
   each kept owner in turn (computed styles and the share table, `Used`,
   the kept maps, layout boxes, text layouts, kept paint, the arena)
   and records the live-bytes fall for each. That counts what each
   owner holds on the heap, `String`s, `Vec`s, taffy's caches and
   shaped layouts included, which a capacity-times-size sum would
   miss. What no drop accounts for is printed as unattributed.
3. **`Used` shared** (U2): one `Used` per distinct (style, parent
   `Used`, context), not per element.
4. **A judged bound,** in the form decision 5 picks. `--mount` judges
   it at each size, failing and naming the owners and figures.
   Property 8 holds when it passes at 1,000, 3,000 and 10,000 rows.

## 6. PR stack

Each slice is under about 400 lines with its tests. "Proven by
breaking" means the guarded behaviour was broken, the test seen to
fail, and restored, stated in the commit.

| # | Slice | Content | Acceptance | Depends on |
|---|---|---|---|---|
| U1 | Properties read by index | `Reader::value` takes a `PropertyId`; ids as constants (a `const` table generated from or checked against `PROPERTIES`); `side_property` without `format!`; `check_honoured` over precomputed ids, allocating nothing when nothing is refused | Mount and `--scaling Class` style instructions before and after, in the PR; every `incremental_*` test and Chromium comparison unchanged; the table check proven by breaking (a misspelt id fails the test naming it) | none |
| U2 | Shared `Used` | `Kept::used` holds `Rc<Used>`; a table keyed by (computed style address, parent `Used` address, context bits: `flex_item`, `in_inline`, `text_like_field`, `surface`, `to_viewport`), pinning both like `ShareKey`; cleared with the share table when the environment changes; per-element refusals still built per element on the refusal path; debug oracle derives each hit's `Used` per element and panics naming the site and field; counts `used_shared`, `used_derived` | Mount at 3000 rows: `used_derived` equals the distinct entries (4 plus the unkeyed), style per element recorded before and after; oracle proven by breaking (a context bit left out of the key fails); Chromium comparisons unchanged | none (after U1 if both are open: same files) |
| T1 | Text shaping kept by content | The text cache keyed by text, style, width and scale with span positions relative to the run, not `NodeId` (first read why `SpanKey` holds the node, and keep what needs it per node); a `texts_shaped` count beside `text layouts` | Mount: `texts_shaped` per row and layout instructions per row before and after, in the PR; a page of 1,000 equal labels shapes once, pinned; text oracle comparisons with Chromium unchanged; the count proven by breaking (node back in the key) | none |
| R1a | Compiled global sheets | Generated code for a `Styles::Global` file registers its sheet itself under `SheetId::Global(file)`; the elements corpus (`events_vapors.rs`) moves from `add_stylesheet` to it | Corpus results unchanged; a test that a global fixture's sheet is registered once per document | none |
| R1b | Rust sheets refused beside analysed elements | `add_stylesheet` on a document that has created a keyed element is refused naming the fix (compile the sheet into a component); `create_keyed_element` on a document holding a Rust-added author sheet is refused under the update-error convention, naming the sheet | Both refusals pinned and proven by breaking; the 5 non-test `add_stylesheet` sites (no keyed elements) unchanged | R1a; decision 2 |
| B0 (done) | Compiler reads sheets | `vapors-compiler` depends on `vapors-css`; `scope.rs` scoping, its `first_global` and `first_nested_global` refusals, and the UA sheet text move into `vapors-css` (the runtime calls the moved code); the compiler parses and scopes each `<style>` and refuses what the runtime refuses, naming `file:line:column` | A malformed `<style>` and a nested `:global()` each fail `compile_dir` with the position; runtime behaviour unchanged (all tiers green); refusals proven by breaking | none |
| B1 | Classification | Per template element: candidate rules from `rules_for_key` over the UA sheet and the crate's sheets, each classified never / always / conditional with what it depends on (other sheets' rules always conditional); per longhand and custom property: constant or known-variable winner, constant or variable value; `wisp-check --styles <file>` prints both | Table-driven tests per selector shape: tag, class, directive, `:not()` over a directive class, state, structural pseudo-classes in the subject (`:first-child`, `:root`, `:empty`: conditional), attribute literal, expression, absent from the template, and case-insensitive values (`type`), descendant, `:global`, `@media`, inline `style`, `!important` across origins, `all` and a shorthand in a conditional rule over an always longhand, `var()` fed from an always rule and from a conditional one, an equal-specificity rule from another sheet; each class of result proven by breaking | B0 |
| B2 | Emitted winners | `StyleKey::winners` (`KeyWinners`: per-sheet identity and hash, base winners for longhands and custom properties, conditional rules); the sheet hash in one function in `vapors-css`, used by both; snapshot tests of the generated keys updated | Generated code compiles for every demo and the racer; compiler and runtime hashes equal for each demo's sheets, pinned; hash proven by breaking (a changed sheet text changes it) | B1 |
| B3a | Runtime takes compiled winners | Checks 1 and 2 per key per `Kept`, check 1 refusing; on a miss, winners from `base` at their runtime standing plus matching conditional and other-sheet rules; the debug winners oracle and the every-build first-miss verification over longhands and custom properties; `winners_compiled`, `winners_runtime`, `winners_foreign` | Racer and one demo page mounted in two orders give equal styles; in a release-profile test, a forged `base` entry and a sheet registered with other text under the key's scope each refuse the element naming the site (not a wrong render); the debug oracle proven by breaking; Chromium comparisons and `incremental_*` unchanged | B2, R1b |
| B3b | Shareability from the key; the report | `shareable` from the key (false when an other-sheet rule blocks sharing), with its debug oracle; the Mount report's winners line | `winners_runtime` 0 asserted on Mount at every size, the racer and one demo page; shareability oracle proven by breaking | B3a |
| S1 | Splice mark | Section 4: the mark per `{for}` from B1's classification and splice.md (d)'s rules, passed through `keyed(..)`, kept on the owner while no other sheet reaches the row keys with a rule that matters; the debug panic, `splice_mark_contradicted`, `splice_marks_dropped`; `wisp-check` lists unmarked lists with reasons | Mount's list marked; a list whose row holds a positioned element, one with `:nth-child`, one with an inline top-level node each unmarked with its reason; a `Styles::Global` sheet with `:global(.row) { position: relative }` drops the mark at run time without a panic; the contradiction check proven by breaking (a forged mark on a positioned row panics) | B3a; splice PR B (merged) |
| M1 | Property 7 published | Mount run per the finish-line method in a quiet window (section 5) after U2 and B3b; `docs/perf/mount-<date>.md`; the finish-line row for property 7 | Times and instructions at 1,000, 3,000, 10,000 rows, three runs, medians, load under 1.5 recorded; table row cites the file | U2, B3b |
| M2 | Property 8 attributed and judged | Counting allocator in `vapors-perf`; drop attribution; `malloc_trim` cross-check; the bound of decision 5 judged by `--mount` | The per-owner split recorded in `docs/perf/`; the cross-check within 10% at each size; the bound passes at all three sizes after U2 or fails naming the owners, proven by breaking (`Used` unshared) | U2; decision 5 |

Order of work: U2, U1, M2, T1, R1a, R1b, B0, B1, B2, B3a, B3b, S1, M1
(recommendation 1 in section 7). The B slices can run in parallel
with the U slices and T1 once their files do not overlap: the B and R
slices touch `crates/compiler`, `crates/css`, `crates/dom`,
`dom_keyed.rs` and `restyle_share.rs`; the U slices touch
`crates/elements/src/style/` and `restyle_pass.rs`; T1 touches
`crates/elements/src/layout/text_*.rs`.

## 7. Decisions

Decided by Tom, 2026-11-12, taking the lead's recommendations: 1 (a)
U1 and U2 at run time first; 2 (a) Rust sheets beside analysed elements
refused, both ways; 3 (a) ancestors above a component's root at run time;
4 (b) shapes move to the shape table, and property 7 is reworded to match;
5 (a) property 8's bound as the record words it. The options are kept
below as the record of what was weighed.

1. **Order: U2 and U1 before B0 to B3, or after.** (a) Before:
   property 7's published time then includes the step that moves it
   (section 3: 97% of style is `used`). (b) After: the property as
   worded is met sooner, with a first-frame time within about 0.01% of
   today's. **Recommend (a).**
2. **Sheets added from Rust beside analysed elements.** The
   analysable-sources record (item 3) refuses every way in the analysis
   cannot see, naming "a stylesheet added after an analysed mount";
   main does not refuse it yet (`Sheets::add_author` accepts at any
   time), and a sheet added before the mount is not named. (a) Apply
   the record to both: R1b refuses `add_stylesheet` once a keyed
   element exists and refuses a keyed element in a document holding a
   Rust-added author sheet. The one mixed use, the elements corpus,
   moves to compiled global sheets (R1a); the other 5 sites have no
   keyed elements. (b) Amend the record: allow a Rust sheet added
   before the first analysed mount, ranked at run time like another
   crate's sheet and counted in `winners_foreign`; refuse only after.
   **Recommend (a):** it keeps the record as decided, and costs one
   corpus migration.
3. **Ancestors above a component's root.** (a) First version: any
   selector with an ancestor compound is conditional and tested at run
   time. (b) Index every mount site of each component in the crate and
   resolve ancestor compounds across files. **Recommend (a):** the
   Mount rows' selectors are single compounds, and (b) can follow when
   a page shows many conditional rules.
4. **"Constant shapes" in property 7.** The only shapes in templates
   today are icon `<path d>` literals, parsed at paint time by
   `vapors-text`'s `IconPath` (`svg_paint.rs`); no workload has icons,
   so the cost is unmeasured. (a) Parse literal `d` at build time in
   item 7, which needs `IconPath` moved out of `vapors-text` so the
   compiler does not link the text stack. (b) Move it to item 8, the
   shape table, which handles all shapes. **Recommend (b),** and
   restate property 7's row to say shapes come with item 8.
5. **Property 8's bound.** (a) As the record words it: all
   per-element live bytes other than the layout records (styles,
   `Used`, kept maps, text layouts, kept paint, the arena) together
   no more than the layout records per element. (b) Narrower: style
   records alone no more than the layout records, with the rest
   reported and not judged. **Recommend (a):** it is the decided
   property; if text layouts or paint exceed it, M2's split names the
   next work item.
