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
- The PR stack (section 6) has 9 slices under about 400 lines each:
  two runtime slices for `Used`, four compiler slices for build-time
  winners, the splice mark, and two measurement slices. Section 7 has
  four decisions for Tom.

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
- **Sheets:** the user-agent sheet (`crates/elements/src/style/ua.css`),
  the component's own `<style>` scoped as the runtime scopes it
  (`crates/elements/src/style/scope.rs`), and the sheets of the other
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

Then, per longhand: the highest-standing declaration among the
**always** rules and the literal `style` is the build-time winner. A
longhand is **constant** when no conditional rule declares it with a
higher standing (origin and importance, specificity, order: the same
`Candidate::beats` the runtime uses, `cascade/collect.rs`); otherwise
it is **known variable**, and the compiler has the list of conditional
rules that can override it. These are the brief's two states
(`docs/briefs/NATIVE_SVG_BRIEF.md`, "One analysis for every painted
thing"). There is no third: a rule the compiler cannot classify more
narrowly is conditional, which is exact because the runtime tests it.

The compiler also decides each key's shareability (no candidate rule
reads siblings or descendants or sits under `@container`,
`vapors_css::cascade::unshareable`), which the runtime decides today
once per key per `Kept` (`restyle_share.rs` `shareable`).

All of this calls `vapors-css` (parser, scoping, bucket filter,
matcher pieces, `unshareable`, `Candidate` ranking): no second CSS
implementation in the compiler (the analysis handoff's rule,
`~/.cache/handoffs/fable-analysis.md` section 3). Two pieces live
outside `vapors-css` today and move into it: the scoping in
`crates/elements/src/style/scope.rs` and the UA sheet text.

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
- **Sheets the compiler did not see.** A Rust-added sheet whose rules
  can reach a key's elements (section 7, decision 2).
- **Layout, text shaping, paint.** Text, fonts, viewport and images.

## 2. What the compiler emits and how the runtime uses it

Building on `StyleKey` and the share table; replacing neither.

### Emitted

`StyleKey` gains one field, `winners: Option<&'static KeyWinners>`
(`None` from hand-written keys in tests):

```rust
pub struct KeyWinners {
    /// Hash of the sheets the compiler classified against, in order:
    /// the UA sheet text, each scoped sheet's text and scope, the
    /// vapors-css version.
    pub sheets: u64,
    /// Per constant or build-time-won longhand: (longhand index, rule
    /// index, declaration index); the literal `style` as a pseudo-rule.
    pub base: &'static [(u16, u32, u16)],
    /// The conditional rules, by index into the registered sheets, in
    /// cascade order.
    pub conditional: &'static [u32],
    /// No candidate rule blocks sharing.
    pub shareable: bool,
}
```

Rules are referenced by index into the document's cascade, not copied
as declaration data, so the generated code does not depend on
`vapors-css`'s value types and stays small. The `sheets` hash is what
makes the indices safe: the runtime computes the same hash over the
sheets it holds when the key is first used, and a mismatch means the
indices may point at other rules (handled per decision 2).

### Used at run time

On a share-table miss (`restyle_share.rs` `shared_style`), a keyed
element whose key has `winners` and whose hash matches computes its
winners from `base` plus the conditional rules that match it, instead
of scanning every candidate rule (`cascade/collect.rs` `winners`).
`compute::style` is unchanged: computed values come from those winners
and the environment as today. `shareable` is read from the key instead
of `Kept::shared.shareable`.

On a hit nothing changes: the shared `Rc<ComputedStyle>` is taken.

Unkeyed elements (hand-built pages, tests) take today's path.

### Loud checks

- **Winners oracle:** debug builds compute the full runtime winners for
  every element that took compiled winners and compare; a difference
  panics naming the key's site, the longhand, both declarations and
  the fix (the shape of `check_shared` in `restyle_share.rs`). Release
  builds count.
- **Shareability:** debug builds also run today's per-key decision and
  panic when it disagrees with `KeyWinners::shareable`.
- **Counts:** `UpdateCounts` gains `winners_compiled` and
  `winners_runtime` (a keyed element computed on the runtime path
  because its key has no winners or its hash did not match). The Mount
  workload pins `winners_runtime` 0, so a hash that silently stops
  matching shows.
- **Build failures:** a component `<style>` that `vapors-css` cannot
  parse fails the build naming `file:line:column`, where today it fails
  at the first update.

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
| B1 to B4: build-time winners | the candidate scan on a share-table miss | about 4 cascades per Mount page: under 0.5 M of 3.44 G instructions (an estimate: about 106,000 per computed style, from 189,000 per element before sharing less the 83,000 of `used`) |

So on Mount, U2 is the step that moves property 7's time; U1 helps
every element that does not hit (unkeyed pages, edits). The `Class`
edit (`vapors-perf 60 10 --scaling Class` at 7d1fe84, the hand-built
rows page, unkeyed) spends 851,594 of its 990,175 update instructions
in style at 1000 rows; U1 is the step that reaches it, and build-time
winners do not, because the page has no keys.

Not in this plan, recorded for the order of work: layout is now the
second phase at 250,000 per row, and 86% of it (28.1 / 32.7 of the
samples) is text shaping in `TextCache::layout`. The text cache key
holds the `NodeId` (`layout/text_key.rs`), so 10,000 identical
"Edit" buttons shape 10,000 times. A content-keyed text cache is a
runtime change outside the build-time analysis.

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
  runtime keeps it on the list's owner. A marked owner whose update
  declines to splice for a stacking reason (`SpliceDecline::
  InsertedStacked`, `FreedStacked`, `Restacked`) means the analysis was
  wrong: debug builds panic naming the `{for}` site and the box; release
  builds count `splice_mark_contradicted`.
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
  the load was 14 to 36. Property 7 holds when its row in the
  finish-line table shows the times from such a run.
- **The build-time part shown by the harness.** The Mount report gains
  `winners: compiled N, runtime N` beside #240's styles line, with
  `winners_runtime` 0 asserted in `gpu_mount_frames` and
  `mount_styles_tests.rs`.
- **Published:** `docs/perf/mount-<date>.md` with the run's commit,
  load, hardware and both tables, and the finish-line table's property
  7 row updated to cite it.

Property 7 has no time target; "measured and published" is the bar.
This plan's slices are ordered so the published time is the one after
U2.

### Property 8 after #240

#240 shows `ComputedStyle` allocations flat in rows (6 live at 1,000,
3,000 and 10,000 rows) and leaves 48 KiB per row of anonymous
high-water unattributed. Property 8's target is "one style record per
distinct style; per-element memory bounded by the per-box layout
records". Still needed:

1. **Attribution.** Which owner holds the per-row memory: `Used` (per
   element today), the kept maps (`Kept::styles`, `used`, `refused`,
   `restyled_nodes`), the share keys, the layout boxes, the text
   layouts, the kept paint. No heap profiler is installed on the dev
   machine; the slice M2 adds a `Document::memory_report()` that sums
   each kept structure's capacity times its element size, read after
   the first frame, plus a `malloc_trim` and second `RssAnon` reading
   to separate what the frame keeps from its high-water.
2. **`Used` shared** (U2): one `Used` per distinct (style, parent
   `Used`, context), not per element.
3. **A judged bound.** `--mount` judges, at each size, that style
   records per element (computed styles, `Used`, style map entries)
   take no more bytes than layout records per element, failing and
   naming both figures otherwise. Property 8 holds when that check
   passes at 1,000, 3,000 and 10,000 rows.

## 6. PR stack

Each slice is under about 400 lines with its tests. "Proven by
breaking" means the guarded behaviour was broken, the test seen to
fail, and restored, stated in the commit.

| # | Slice | Content | Acceptance | Depends on |
|---|---|---|---|---|
| U1 | Properties read by index | `Reader::value` takes a `PropertyId`; ids as constants (a `const` table generated from or checked against `PROPERTIES`); `side_property` without `format!`; `check_honoured` over precomputed ids, allocating nothing when nothing is refused | Mount and `--scaling Class` style instructions before and after, in the PR; every `incremental_*` test and Chromium comparison unchanged; the table check proven by breaking (a misspelt id fails the test naming it) | none |
| U2 | Shared `Used` | `Kept::used` holds `Rc<Used>`; a table keyed by (computed style address, parent `Used` address, context bits: `flex_item`, `in_inline`, `text_like_field`, `surface`, `to_viewport`), pinning both like `ShareKey`; cleared with the share table when the environment changes; per-element refusals still built per element on the refusal path; debug oracle derives each hit's `Used` per element and panics naming the site and field; counts `used_shared`, `used_derived` | Mount at 3000 rows: `used_derived` equals the distinct entries (4 plus the unkeyed), style per element recorded before and after; oracle proven by breaking (a context bit left out of the key fails); Chromium comparisons unchanged | none (after U1 if both are open: same files) |
| B0 | Compiler reads sheets | `vapors-compiler` depends on `vapors-css`; `scope.rs` scoping and the UA sheet text move into `vapors-css` (the runtime calls the moved code); the compiler parses and scopes each `<style>` and refuses a parse error naming `file:line:column` | A malformed `<style>` fails `compile_dir` with the position; runtime behaviour unchanged (all tiers green); refusal proven by breaking | none |
| B1 | Classification | Per template element: candidate rules from `rules_for_key` over the UA sheet and the crate's sheets, each classified never / always / conditional with what it depends on; per longhand constant or known variable; `wisp-check --styles <file>` prints it | Table-driven tests per selector shape (tag, class, directive, state, attribute literal and expression, descendant, `:global`, `@media`, inline `style`, `!important` across origins); each class of result proven by breaking | B0 |
| B2 | Emitted winners | `StyleKey::winners` (`KeyWinners`), the sheet hash in both compiler and runtime (one function in `vapors-css`); snapshot tests of the generated keys updated | Generated code compiles for every demo and the racer; hash equality between compiler and runtime for each demo's sheets pinned by a test; hash proven by breaking (a changed sheet text changes it) | B1 |
| B3 | Runtime takes compiled winners | On a miss, winners from `base` plus matching conditional rules; `shareable` from the key; the winners and shareability oracles; `winners_compiled`, `winners_runtime`; the Mount report's winners line | Mount: `winners_runtime` 0 at every size, asserted; oracle proven by breaking (a forged `base` entry panics naming the site and longhand); Chromium comparisons and `incremental_*` unchanged | B2 |
| S1 | Splice mark | Section 4: the mark per `{for}` from B1's classification and splice.md (d)'s rules, passed through `keyed(..)`, kept on the owner; the debug panic and `splice_mark_contradicted`; `wisp-check` lists unmarked lists with reasons | Mount's list marked; a list whose row holds a positioned element, one with `:nth-child`, one with an inline top-level node each unmarked with its reason; the contradiction check proven by breaking (a forged mark on a positioned row panics) | B1; splice PR B (merged) |
| M1 | Property 7 published | Mount run per the finish-line method on a quiet machine after U2 and B3; `docs/perf/mount-<date>.md`; the finish-line row for property 7 | Times and instructions at 1,000, 3,000, 10,000 rows, three runs, medians, load under 1.5 recorded; table row cites the file | U2, B3 |
| M2 | Property 8 attributed and judged | `Document::memory_report()`; `--mount` prints it and judges style bytes per element against layout bytes per element; `malloc_trim` second reading | The bound passes at all three sizes after U2, and fails (proven by breaking: `Used` unshared) naming both figures; the doc records the per-owner split | U2 |

Order of work: U2, U1, M2, B0, B1, B2, B3, S1, M1 (recommendation 1
in section 7). B0 to B3 can run in parallel with U1 and U2 once their
files do not overlap: the B slices touch `crates/compiler`,
`crates/css` and `restyle_share.rs`; the U slices touch
`crates/elements/src/style/` and `restyle_pass.rs`.

## 7. Decisions for Tom

1. **Order: the measured lever first, or the property's letter
   first.** (a) U2 and U1 before the build-time slices: property 7's
   published time then reflects the step that moves it (section 3:
   97% of style is `used`). (b) B0 to B3 first: the property as worded
   is met sooner, with a first-frame time within about 0.01% of
   today's. **Recommend (a).**
2. **A sheet the compiler did not see.** A Rust-added sheet
   (`add_stylesheet`, before any analysed mount) can reach keyed
   elements; adding one after an analysed mount is already to be
   refused (decision record, item 3). (a) Refuse the mount when the
   document's sheet hash differs from a key's, naming the sheet; pages
   that mix hand-added sheets with compiled components (examples,
   fixtures) then need their sheets compiled in. (b) Keys whose hash
   does not match take the runtime cascade, exact as today, counted in
   `winners_runtime`, and the Mount workload pins it at 0. **Recommend
   (b):** the runtime path is the current exact cascade, not an
   unknown, and the count makes it visible.
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
