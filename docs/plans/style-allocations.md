# Style allocations: an edit's style phase without malloc

Finish-line property 1 (`docs/plans/perf-finish-line.md`): an edit's
whole update under 1 ms, the same within noise at 1,000, 3,000 and
10,000 rows. The lead's quiet-window timed run at 9b405f9 (median of
3, Intel UHD 630) puts a Class edit's update at 282 / 303 / 297 µs at
1k / 3k / 10k rows (style 221 / 241 / 221 µs), so the time target
holds. What this document covers is margin: the `style` phase of a
Class edit is 845k instructions, and about a fifth of it is heap
calls in the cascade of the edited row's four elements (#246,
iu1.md "PR 2"). It says where each allocation comes from, what was
measured, and which change is worth its lines.

Written 2026-11-12 from `origin/main` at 9b405f9 (#246 merged). Line
numbers are from that commit.

## Summary

- Measured (section 2): a Class edit's `style` span at 3,000 rows
  makes 842 mallocs, 843 frees and 75 reallocs, 108 KB asked for.
  About 720 of the mallocs are in `Cascade::compute`, almost all of
  them clones of keyword and list values: every longhand's value is
  cloned once into a `specified` vec and once more by `resolve`,
  whether or not the element declares it.
- A prototype of O1 and O2 (section 3; about 90 lines in
  `collect.rs`, `compute.rs`, `buckets.rs`, `css_element.rs`:
  winners referenced instead of cloned, the parent's computed value
  copied without `resolve`, no second copy) takes `style` from
  845,045 to 643,552 instructions (−24%) and the span's heap calls
  from 1,760 to 1,025; `--scaling Class` passes at 646,835 / 648,515 /
  649,320. The css crate's tests and the elements' incremental tests
  pass on it. That is more than jemalloc preloaded on main gives
  (651,613), it holds on every allocator and on the web, and it is
  the one slice this plan recommends (S1).
- What is left after S1 is about 350 clones per edit of keyword and
  list values on the initial-value path, removable only by changing
  `Value`'s representation (S3, about 200 lines, estimated 40k to
  95k instructions), and the `Used` derivation's `format!` calls
  (116 per edit), which U1 of `build-time-styles.md` removes. Neither
  is recommended now that the time target holds; section 6 asks.

## 1. Where the style phase allocates

The `Class` workload (`crates/perf/src/rows.rs`) toggles `marked` on
the middle row's `class` attribute. `mark_changed` (`restyle.rs:81`)
marks the row's subtree, so `restyle_subtree` restyles four unkeyed
elements: `div.row`, `span.label`, `span.value`, `button`
(`tests/support/rows_page.rs:76-91`). Each goes through
`restyle_one` (`restyle_pass.rs:144`): `style_for` (the cascade),
then `keep_used` (the `Used` derivation). The page has no `style`
attributes and no custom properties.

### 1.1 Per element, in the cascade (`style_for`)

`Cascade::compute` (`crates/css/src/cascade/mod.rs:161-170`): the
candidate rules, the winners, the computed values. Counts are per
element restyled, from the code; section 2 has the measured ones.
K is the number of longhands whose computed value holds heap (a
`Value::Keyword` is a `String`; a `List`, `CommaList` or `Function`
a `Vec`): measured 82 of the 104 (section 2: 330 copies over four
elements). W is the element's winning declarations: 8 to 15 here.

| # | site | what | per element | freed |
|---|---|---|---|---|
| C1 | `cascade/buckets.rs:60` | `self.universal.clone()`, the candidate list, grown by the class and tag buckets (`:61-68`) | 1 to 3 | end of `compute` |
| C2 | `cascade/buckets.rs:67` | `tag.to_ascii_lowercase()` | 1 | same call |
| C3 | `elements/src/css_element.rs:73-77` | `class_names` collects a `Vec<&str>`: once in `candidates`, and once per class compound the matcher tests through `has_class` (`:69-71`) | 1 + class tests (measured 7 per edit) | same call |
| C4 | `cascade/collect.rs:68` | `vec![None; PROPERTIES.len()]`, the winners table: 104 `Option<Candidate<Specified>>`, about 10 KB | 1 | end of `compute` |
| C5 | `cascade/collect.rs:131` | `longhand.value.clone()` per longhand a matching declaration sets; keyword and list values allocate | measured 3 | end of `compute` |
| C6 | `cascade/compute.rs:152-157` | `custom_properties`: `parent.custom.clone()` and `raw.clone()`, two deep clones of the parent's `BTreeMap<String, Vec<ComponentValue>>` | 0 here; on a page with N custom properties in scope about 2 × N × (1 + tokens per value): touche's `crates/web/shell.css` sets 18 on `:root`, so every element of a touche page pays about 80 to 100 | one copy lives in the style |
| C7 | `cascade/compute.rs:44` | the `specified` vec | 1 | end of `style` |
| C8 | `cascade/compute.rs:175-189` | `specified_value`: a clone of the winner's, the parent's or the initial value, per longhand | K | end of `style` |
| C9 | `cascade/compute.rs:52` | the `values` vec | 1 | with the style |
| C10 | `cascade/compute_resolve.rs:35` | `resolve`: `other.clone()` for every value that is not a length, calc or colour: the keyword values again | K, less the lists | with the style |
| C11 | `cascade/compute_resolve.rs:30-34` | `resolve_all`: a new `Vec` per list value and a clone of each item; 13 list-typed longhands (`font-family`, the `animation-*` and `transition-*` lists, `box-shadow`, `grid-template-*`) | measured 13 + 21 items | with the style |
| C12 | `cascade/compute.rs:65-69` | the `positions` vec | 1 | with the style |
| C13 | `elements/src/style/styles.rs:47-52` | `Rc::new(Counted { style, live })` | 1 | when the last holder drops |

The style returned holds K + 4 heap blocks; the next restyle of the
same element frees them when `kept.styles.set` (`restyle_pass.rs:149`)
drops the old `Rc`, so an edit pays the frees of the last edit's
styles too (843 frees to 842 mallocs in section 2).

### 1.2 Per element, in the `Used` derivation (`keep_used`)

`used` (`elements/src/style/used.rs:63-111`) reads the computed
style through `Reader` (`values.rs:194-242`):

| # | site | what | per element |
|---|---|---|---|
| U-a | `style/decoration.rs:96` | `format!("border-{side}-{what}")` per side property read | measured 29 |
| U-b | `style/text_style.rs:86` | `name.to_ascii_lowercase()` of the first font family | 1 |
| U-c | `style/grid_style.rs` (`grid`, `auto_tracks`) | taffy's grid track vecs, with the `grid` feature `vapors-perf` builds with | measured 4 |
| U-d | `style/honoured.rs:169`, `restyle_pass.rs:195-200` | `check_honoured`'s vec (empty: no allocation); the `refused` and `used` map inserts (no growth after the mount) | 0 |

U-a is U1's (`build-time-styles.md`: properties read by index, no
`format!`). U-b and U-c go with it (section 4). U-c and the
`check_honoured` walk over the table (about half of `used` per
element, sampled 2026-11-13) are done by slice B: `check_honoured`
visits only the longhands a declaration reached
(`ComputedStyle::declared_ids`), and the grid readers allocate nothing
for the initial values (`docs/perf/style-declared-2026-11-13.md`).

### 1.3 Per edit, around the restyle

Flat in rows; about 50 calls per edit (measured): `dirty_roots`'s
set and `chain` vecs (`restyle_pass.rs:98-123`), `elements`
(`check.rs:19-30`), the new `restyled_nodes` set (#246),
`touched` and `rechecked` (`style_checks.rs:21-63`), and
`check::describe` strings from `media_checks::refusals` (14 per edit,
184 bytes: a description built for elements that refuse nothing).

### 1.4 The keyed path

A keyed element whose key is shareable (`restyle_share.rs:38-57`)
takes the share table's style by `ShareKey` (`style/share.rs:19-31`);
a hit allocates nothing (the selected-attributes box is empty when no
sheet selects an attribute the element has). A miss runs
`compute_own`, the whole of 1.1. On the compiled rows page
(`crates/perf/ui/mount_row.wisp`) `marked` is a toggle
(`class:marked={marked}`), so after the first toggle each way every
element of the row hits: the children's keys hold the row's shared
style address, which the table pins (`share.rs:33-37`). The cascade's
allocations are then paid once per distinct share key per `Kept`;
the `Used` derivation (1.2) is still per element per edit until U2.
Property 1 is judged on the hand-built, unkeyed page; that is where
S1 applies, and on every miss of a keyed page.

## 2. Measured

Release builds on this machine (load 20 to 35; instruction counts do
not move with load). Heap calls from a scratch example
(`crates/perf/examples/style_allocs.rs` on a scratch branch, not
merged): a counting `#[global_allocator]` over `System` that counts
mallocs, frees, reallocs and bytes asked inside each `update phase`
span of the 3,000-row page over 10 Class edits after 3 warm-up edits,
and records the backtrace of every malloc in the `style` span for 3
of them, aggregated by the innermost `vapors_*` frame and its caller
(symbols only, no line tables, so inlined callees fold into their
callers). Instructions from
`vapors-perf 60 10 --instructions --only "3000 rows, Class"`.

### Heap calls per edit at 9b405f9

| phase | mallocs | frees | reallocs | bytes asked |
|---|---|---|---|---|
| style | 842 | 843 | 75 | 108,266 |
| layout | 59 | 59 | 6 | 23,112 |
| paint | 48 | 48 | 2 | 5,045 |

`style` mallocs and reallocs by site, per edit (four elements):

| per edit | bytes | site (section 1) |
|---|---|---|
| 289 | 1,405 | `Value::clone` in `compute_resolve::resolve` (C10) |
| 246 | 1,187 | `Value::clone` in `specified_value` (C8) |
| 116 | 2,688 | `decoration::side_property` (U-a) |
| 84 | 2,644 | `Value::clone` inside a list clone (C8, C10: items) |
| 52 | 9,984 | `Vec<Value>` from `resolve_all` (C11) |
| 16 | 288 | `grid_style::grid`, `auto_tracks` (U-c) |
| 16 | 89,856 | the vecs of `compute::style` and the winners table (C4, C7, C9, C12) |
| 14 | 184 | `check::describe` from `media_checks::refusals` (1.3) |
| 13 | 289 | `buckets::candidates` and the tag string (C1, C2) |
| 12 | 75 | `Winners::consider` (C5) |
| 7 | 448 | `has_class`, `class_names` (C3) |
| 4 | 512 | `compute_own` (`Rc::new`, C13) |
| 4 | 20 | `text_style::face` (U-b) |
| about 45 | | `chain`, `elements`, `children`, the sets, `to_css` (1.3) |

So of 917 heap calls that allocate (mallocs and reallocs), 720 are
the cascade's clones and vecs, 136 the `Used` derivation's, about 60
the per-edit work around them.

### Instructions

| build | update | style | layout | paint |
|---|---|---|---|---|
| 9b405f9 | 990,314 | 845,045 | 79,047 | 44,961 |
| 9b405f9, `LD_PRELOAD` jemalloc | 768,613 | 651,613 | 61,016 | 43,945 |
| 9b405f9 + prototype of O1 and O2 | 777,146 | 643,552 | 72,082 | 44,977 |

The prototype's heap calls in `style`: 477 mallocs, 478 frees, 70
reallocs, 64,017 bytes asked. Its `--scaling Class`: `style` 646,835 /
648,515 / 649,320 at 1,000 / 3,000 / 10,000 rows, pass (0.1 per extra
row from 3,000 to 10,000). The css crate's tests and the elements'
`incremental_styles`, `incremental_inherit`, `incremental_random` and
`sparse_updates` pass on it, with the copy oracle (O2) running as a
`debug_assert` in the test profile.

Per heap call: the jemalloc run says glibc costs about 110
instructions more than jemalloc per call on this heap (193k over
1,760 calls); the prototype removes 735 calls and 201k instructions,
about 270 per call removed, the difference being the `resolve` work
it skips for the 104 − W properties with no declaration.

## 3. Options

Ranked by instructions saved per line changed. Savings are measured
where the prototype covers the option; "estimated" is calls × 110 to
270 instructions, the per-call range measured above.

| option | sources | change | calls removed per edit | instructions | lines |
|---|---|---|---|---|---|
| O2 Values copied once | C5, C7, C8, C10 (part) | `Winners<'a>` holds `Candidate<&'a Specified>` and `&'a [ComponentValue]`: a reference to the declaration, not a clone. `compute::style` builds `values` straight from a per-longhand `Source`: the winner's value or the initial one, resolved; or the parent's computed value, copied without `resolve` when there is no declaration (or `inherit`, or `unset` on an inherited property). A computed value resolves to itself (lengths px, colours rgba, calcs absolute, `line-height` percentages px), which a `debug_assert` checks on every copy. `font_size` and `color` take the same `Source`. | 735 (with O1; O1 is under 10 of them) | 201k measured (with O1) | about 90 (prototype), about 150 with tests |
| O1 Match without allocating | C2, C3 | `has_class` tests `split_ascii_whitespace().any(..)`; `candidates` lowercases the tag only when it has an upper-case byte, and reserves the list once. `class_names` stays: the css crate's own tree keeps classes apart from attributes, so the matcher cannot read the attribute. | about 8 | about 2k estimated | 15 |
| O4 Custom properties shared | C6 | `ComputedStyle::custom` as `Rc<BTreeMap<..>>` (or `Arc`): an element with no custom-property winner shares the parent's map; one with winners clones once and overlays | 0 on the rows page; about 320 on a touche page (18 properties, four elements) | 0 here; 35k to 85k estimated on touche's pages | 40 |
| O3 Keyword and list values that clone without allocating | C8, C10, C11's remainder after O2 | `Value::Keyword(Arc<str>)` (the initial values live in a `OnceLock` static, so `Rc` is out; `Arc<str>` equality checks the pointer first) and `Arc<[Value]>` for `List`, `CommaList`, `Function`. 39 `Value::Keyword` and 53 list sites outside tests change their constructors; `k == "auto"` comparisons keep working through `PartialEq<str>` | about 350 (the initial-value clones and the lists) | 40k to 95k estimated | about 200, two PRs |
| O7 Global allocator | all | `tikv-jemallocator` native (`#[cfg(not(target_arch = "wasm32"))]`): jemalloc does not build for `wasm32-unknown-unknown`, nor does `mimalloc`'s crate (both need a libc); the page keeps dlmalloc either way. About 300 to 400 KiB more native binary (estimated). | 0 (cheaper calls) | 193k measured on main; after O2 at most the prototype's 1,025 calls × 110, about 110k | 10, plus a dependency |
| O5 One vec for `values` and `positions` | C9, C12 | `Box<[(Value, Option<(usize, Position)>)]>` | 4 | under 2k | 30 |
| U1 (build-time-styles.md) | U-a, U-b, U-c | properties read by index, no `format!`; `face` by `eq_ignore_ascii_case`; grid tracks left empty when the display is not grid | 136 | 15k to 37k estimated, plus the by-name lookups U1 is for | U1's |

O2 is the one with a measured saving above 100k for under 200 lines.
O7 saves less than O2 on main and about half as much after it, costs
binary size, and does nothing on the web. O3 is the only way to the
remaining clones and is the costliest change for the least certain
saving.

## 4. Relation to build-time styles

`docs/plans/build-time-styles.md` plans U1 (properties read by index,
no `format!`), U2 (shared `Used`), the B slices (compiled winners)
and M2 (a counting global allocator in `vapors-perf` for live bytes).
This plan:

- Covers the cascade (`style_for`, 1.1), which none of those slices
  touch: U1 and U2 are the `Used` derivation (1.2); the B slices
  replace the always rules' matching on a share-table miss and leave
  `compute::style`, where 720 of the 917 calls are, as it is; they do
  not reach the unkeyed rows page at all.
- Leaves 1.2 to U1, and adds U-b and U-c to its list.
- Does not add a heap counter to `vapors-perf`: the acceptance below
  is in instructions, which the harness already counts. When M2's
  allocator lands, the example's per-phase counts are a few lines on
  top of it.
- Touches `crates/css/src/cascade` (`collect.rs`, `compute.rs`,
  `buckets.rs`) and `css_element.rs`; U1 and U2 touch
  `crates/elements/src/style` and `restyle_pass.rs`; the B slices
  `crates/compiler`, `dom_keyed.rs`, `restyle_share.rs`. S1 runs
  beside any of them. O3 (if chosen) touches every crate that matches
  on `Value::Keyword`, including `crates/elements/src/style`, so it
  waits for U1.

## 5. Slices

| # | slice | content | accepted when | depends on |
|---|---|---|---|---|
| S1 | Cascade values copied once | O2 and O1 as the prototype has them, with tests: a unit test per `Source` path (winner, `inherit`, `unset` on an inherited and a non-inherited property, `initial`, a `var()` winner), and the copy oracle proven by breaking (a `Source::Parent` forged for a declared `currentcolor` border fails the `debug_assert` naming the property) | `vapors-perf 60 10 --instructions --only "3000 rows, Class"`: `style` under 660k from 845k (the prototype: 643,552); `--scaling Class` passes; `cargo xtask ci` green, the Chromium comparisons and `incremental_*` unchanged | none |
| S2 | Custom properties shared | O4, with a rows page variant carrying 18 custom properties on `body` in `vapors-perf` (`--only "3000 rows, TokensClass"`) so the saving is measured in the PR | the variant's `style` within 10k instructions of the plain page's; the plain page unchanged | decision 2 |
| S3 | `Value` without heap clones | O3 in two PRs (keywords, then lists) | the remaining clones gone: `style` heap calls per edit under 150 (from 1,025 after S1), `style` instructions recorded before and after | S1, U1; decision 1 |

Not proposed: a global allocator (decision 3); a keyed Class
workload (1.4: the finish line judges the unkeyed page, and the keyed
path after warm-up is four table hits, which the Mount workload's
`styles_shared` already shows).

## 6. Decisions for Tom

1. **S3, the `Value` representation.** The time target holds; S1
   takes the measured 24% and leaves about 350 clones per edit that
   only a representation change removes. (a) Do S3 after U1, about
   200 lines for an estimated 40k to 95k instructions. (b) Leave
   `String` and `Vec` in `Value`; revisit if a workload shows the
   clones. **Recommend (b).**
2. **S2, custom properties shared.** Nothing the finish line
   measures changes; touche's pages pay about 80 to 100 calls per
   element per restyle for their 18 tokens, estimated from the code.
   (a) Do S2 now, 40 lines, measured by its own workload variant.
   (b) Defer until a touche-side measurement asks for it.
   **Recommend (a),** after S1, as the next smallest measured saving.
3. **Global allocator.** (a) None: S1 removes more than jemalloc
   does on main, on every allocator. (b) jemalloc native only, for
   about 110k more after S1 and a few hundred KiB of native binary.
   **Recommend (a).**
