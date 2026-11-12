# Text measure: one break per run per layout

Design for the text slice after T1 (#253, #254). After T1 a run is
shaped once per content and scale and broken once per width layout
asks for. This plan is about which widths layout asks for: on a flex
row two of the three asks read only a width, and the shaped run can
answer a width without breaking.

Where it matters (`docs/plans/perf-finish-line.md`): the first frame
of a large page (property 7: layout is 23% of Mount's instructions,
`docs/perf/mount-2026-11-12.md`) and a new row's text (property 3).
Property 1 holds already: at main 9b405f9 a Text edit's update is
62, 60 and 60 µs at 1,000, 3,000 and 10,000 rows, about 41 µs of it
layout (the lead's timed run, 2026-11-12), and that edit's text is
laid out at a definite width, which this plan does not change
(section 1.3).

References to `measure.rs`, `text_cache.rs`, `text_key.rs` and
`shaped.rs` are to #254's head, `perf/t1-text-cache` at 48d6e90.
Taffy references are to taffy 0.15.1 as vendored
(`~/.cargo/registry/src/*/taffy-0.15.1/src`).

## Summary

- Per layout of a Mount row, the label's run is broken twice before
  the layout that is drawn: once with no width (the flex basis) and
  once at width 0 (the automatic minimum size). Taffy asks both with
  `axis: Horizontal` and reads only the width; vapors breaks the run
  because `measure_text` (`measure.rs:14-31`) is not told the axis and
  answers a height with every width.
- Taffy already carries the axis to the leaf (`LayoutInput::axis`,
  `tree/layout.rs:131`), keys the leaf's cache by it
  (`tree/cache.rs:116-141`), and its own block algorithm answers
  width-only queries with `height: 0.0` (`compute/block.rs:441-445`,
  `609-612`). vapors builds the measure closure where `inputs` is in
  scope (`layout/tree.rs:98-118`), so no Taffy change is needed.
- Option A: a width-only query is answered from
  `Shaped::content_widths` (`shaped.rs:55-57`, Parley's
  `calculate_content_widths`, a walk of the shaped clusters), memoised
  on the kept shaping; the run is broken once, at its final width.
  Measured with a prototype on #254's head (section 3, A): Mount's
  text layouts at 10,000 rows 30,002 to 20,001 (3 per row to 2),
  layout instructions per extra row 205,095 to 189,282 (−7.7%), the
  frame's 579,067 to 564,624 (−2.5%), anonymous memory per row 50.2
  to 47.1 KiB; `--scaling Text` and `Insert` unchanged, as expected.
  1,206 of 1,210 elements and text tests pass at the prototype; the 4
  that fail are count pins that this change moves (section 4).
- Option C, after A: the cache key is built once per run instead of
  on every call (today 6 text clones and hashes per label per layout,
  `text_key.rs:85-94`, and 4 key clones in `text_cache.rs:116`).
- Two slices, each under 400 lines. Decisions for Tom in section 6.

## 1. How a row's text is measured today

Mount's row (`crates/perf/ui/mount_row.wisp`) is
`div.row { display: flex; height: 24px; gap: 8px }` holding
`span.label` (no size), `span.value { width: 80px; height: 20px;
overflow: hidden }` and `button { width: 60px; height: 20px }` with
the text "Edit". Each span and the button is a flex item laid out by
Taffy's block algorithm (`layout/tree.rs:76-78`), and each holds one
anonymous text box, a leaf with the default style
(`layout/boxes.rs:162-171`) and the run as its content.

### 1.1 The label's run, per layout of the row

What reaches `measure_text` for `span.label`'s run, in order. Read
from the code and confirmed on 2026-11-12 by a trace of every
`measure_text` call and text cache outcome in `tests/shaped_once.rs`'s
`rows(1, "Row 1")` (a flex row of two spans, no row height: the
available height there is `MinContent`, on Mount it is definite).

| # | Who asks, where | Input to the text leaf | `measure_text` width | Text cache |
|---|---|---|---|---|
| 1 | Flex basis: `flexbox.rs:1138-1146` asks the span with `axis: Horizontal`, available main `MaxContent`; the span's block algorithm takes its intrinsic-width path, `block.rs:1051-1060`, asking the leaf `measure_child_size(.., known {None, None}, available {MaxContent, ..}, InherentSize, Horizontal)` | `ComputeSize`, `Horizontal`, known none | `None` (`measure.rs:21-25`) | miss: shaped, broken with no width (`text_cache.rs:79-82`) |
| 2 | Automatic minimum size: `flexbox.rs:1176-1188`, available main `MinContent`, `Horizontal`; the same block path | `ComputeSize`, `Horizontal`, known none | `Some(0.0)` | miss: broken at 0, every soft break taken |
| 3 | Hypothetical cross size: `flexbox.rs:2013-2033`, `axis: Vertical`, known width = the item's target width W; the span's block lays its child out (`block.rs:1439-1491`): `ComputeSize`, `axis: Both`, known width W | `ComputeSize`, `Both`, known width W | `Some(W)` | `Key(W)` miss; `unwrapped` (`text_cache.rs:111-118`) finds the no-width layout, start-aligned and fitting W, and returns it |
| 4 | Final layout: `perform_child_layout` on the span, `PerformLayout`, known W x H; the leaf gets `PerformLayout`, and `compute_leaf_layout` passes known none and available `Definite(W)` (`leaf.rs:140-147`) | `PerformLayout`, `Both` | `Some(W)` | as 3 |
| 5, 6 | Paint: `paint_own.rs:90` asks at the box's width; the trace shows two asks per run after the layout | | `Some(W)` | as 3 |

On Mount W equals the max-content width (the row has free space and
the items do not grow), so 3 to 6 reuse the no-width layout. Per
label per layout: 1 shaping, 2 breaks, 6 cache calls each building a
`Key` (`text_key.rs:70-77`: the text and spans cloned and hashed) and
4 of them cloning the `ShapeKey` again for the `unwrapped` lookup
(`text_cache.rs:116`).

Why 1 and 2 break: `measure_text` returns `Size { width, height }`
and reads the height from a layout (`measure.rs:26-30`). Taffy reads
only the width: `measure_child_size` returns `.size.get_abs(axis)`
(`tree/traits.rs:397-421`), and the span's block path discards the
height (`block.rs:611`, `height: 0.0`). The width-0 layout of 2 is
kept in the text cache until swept, one per row.

### 1.2 The value and the button

`span.value` has a definite width, so its flex basis is the style
width (`flexbox.rs:1063-1066`), its automatic minimum size is 0
(`overflow: hidden`, `flexbox.rs:1172-1175`) and its cross size is
known: no size query reaches its text. The final layout asks the leaf
at the span's inner width: 1 shaping, 1 break at 80 px, reused by
paint. The button is the same shape; "Edit" is shaped once per page
and its layouts are shared by every row (#254: 30,002 layouts at
10,000 rows, 3 per row and 2 for "Edit"; with option A, 1 for
"Edit": one of its two was a width-only answer).

So per Mount row: 2 shapings and 3 breaks, of which the label's break
at 0 is read for its width only, and the label's break with no width
is read for its width in 1 and then reused as the drawn layout.

### 1.3 What a text edit asks

`--scaling Text` edits a value (`crates/perf/src/rows.rs:156-161`):
1 shaping, 1 break at 80, as 1.2. `FlexText` edits a row of a flex
column: the column's main axis is vertical, so the basis query is
`axis: Vertical` with the width known from stretch, and the height is
needed: 1 break, and this plan does not change it. The edit this plan
changes is one that resizes a label in a flex row (`sparse_updates`'
"resizing label", `docs/plans/iu1.md`: 2 text layouts built per edit,
1 with option A).

An insert (`--scaling Insert`, a row inserted and removed on
alternate frames) pays no text work at all in the steady state: the
removed row's shaping and layouts are kept by content for one update
and are back the next. So the prototype leaves Insert's counts where
they were (section 3, A). A row with new text, as every row of a
mount or a genuinely new insert has, pays what a Mount row pays, and
option A takes the same share off it.

## 2. What Taffy already gives

- `LayoutInput::axis: RequestedAxis` (`tree/layout.rs:82-89,131`):
  `Horizontal`, `Vertical` or `Both`. `measure_child_size` sets it from
  the axis the caller reads (`tree/traits.rs:397-421`);
  `perform_child_layout` and `measure_child_size_both` set `Both`.
- The per-node cache keys measure entries by the axis, packed into the
  sign bits of `parent_size` (`tree/cache.rs:125-141`), and
  `size_is_valid_for` (`cache.rs:116-122`) returns an entry only for
  the axis it was computed for, or for any axis when it was `Both`:
  "Sizes computed for a single axis may contain garbage values in the
  other axis". So an answer that fills the other axis with a
  placeholder is what the cache is built for.
- Taffy's block algorithm does exactly that for its own width-only
  answers: `height: 0.0` at `block.rs:443` and `block.rs:611`.
- `compute_leaf_layout`'s measure closure does not receive the axis
  (`compute/leaf.rs:27`, called at `leaf.rs:140-147`), but vapors
  builds that closure in `BoxTree::measured_leaf` with `inputs` in
  scope (`layout/tree.rs:98-118`), so `inputs.axis` can be passed to
  `measure_text` without touching Taffy.
- What the leaf does with a placeholder height on a `Horizontal`
  query: `clamped_size` and `size` take it (`leaf.rs:148-156`);
  `margins_can_collapse_through` (`leaf.rs:182-185`) becomes true for
  a block-level text box with no padding when the height is 0, and the
  cache then declines to store the entry (`cache.rs:302-312`). That
  costs a repeat of the width-only answer if the same query is asked
  again within one layout, which section 1.1 shows does not happen
  (the span's own cache answers for it), and is memoised by option A
  in any case. No block reads `margins_can_collapse_through` from a
  width-only answer: the intrinsic-width path reads the width alone
  (`block.rs:1051-1062`). A NaN placeholder would not be louder:
  `f32_max` (`leaf.rs:155`) turns it into 0.0 in the output.
- vapors' own replay (`layout/asked.rs`, `isolated.rs`) records a size
  query's answer and compares a rebuilt box's answer with it. A
  placeholder that is a constant keeps the answer a pure function of
  the input, so recorded answers still compare equal and `disowned`
  does not rise.

## 3. Options

### A. Width-only queries answered from the shaped run

`measured_leaf` passes `inputs.axis` to `measure_text`. When the axis
is `Horizontal`, `measure_text` answers the width without a layout:
`known.width` if it is known, else `Shaped::content_widths().max`
for `MaxContent` or `.min` for `MinContent` (a `Definite` available
width with the axis `Horizontal` is answered as today, by a layout:
the width of a wrapped run at a definite width is not a content
width), each in device pixels divided by the scale as `measure_text`
does today, and `height: 0.0` as Taffy's block does. `Vertical` and
`Both` queries are answered as today.

The text cache gains `content_widths(fonts, scale, run)`: the kept
shaping (`shaped_run`, `text_cache.rs:87-101`) with its content widths
memoised beside it (a `OnceCell<ContentWidths>`: 8 bytes; computed on
the first width-only query, so a run only ever laid out at a definite
width, like the value, never pays the walk). `texts_measured` counts
width-only answers beside `texts_shaped` and `text_layouts_built`
(`crates/elements/src/text_counts.rs`), and the perf report's counts
line shows it.

Which layout is kept for the label changes: today the no-width layout
is built by query 1 and reused at W; after A the first layout asked
for is at W (query 3). `TextCache::layout` keeps the no-width layout
in that case too: when `width` is `Some(W)`, the run is `reusable`
(`text_cache.rs:148-153`) and `content_widths().max` fits W, it builds
and keeps the layout at no width and returns it, as `unwrapped` reuses
one today. A later ask at any width that fits reuses it, so a resize
does not break again, as today. The width-0 layout is not built at
all.

What it removes on Mount: the label's break at 0 (a line per word,
`Layout::broken`'s glyph runs and ink over them, `layout.rs:47-56`),
one `Shaped` clone per row, and 1 of 3 kept layouts per row; it adds
one content-widths walk per label.

Measured on 2026-11-12 with a prototype of A (about 60 lines, on
#254's head 48d6e90, not pushed): release build, `nice`, 4 jobs,
`vapors-perf 3 1 --mount` and `5 1 --scaling Insert|Text`, 1-minute
load 25 to 34 (instruction counts do not move with load; times are
not kept). Before is 48d6e90 itself.

| Mount, 10,000 rows | before | prototype |
|---|---|---|
| text layouts | 30,002 | 20,001 |
| texts shaped | 20,001 | 20,001 |
| layout instructions per extra row, 3,000 to 10,000 | 205,095 | 189,282 (−15,813, −7.7%) |
| frame (CPU) instructions per extra row | 579,067 | 564,624 (−2.5%) |
| style and paint instructions per extra row | 343,857 and 29,476 | 344,516 and 30,175 (+0.2% and +2.4%; their code is untouched, so the difference is the allocator's state with fewer kept layouts, to be checked in the slice) |
| anonymous KiB per extra row after the first frame | 50.2 | 47.1 |

So a break at 0 of a two-word run, with a `Shaped` clone and the
`Layout::broken` work, is about 16,000 instructions; a shaping is
about 48,000 (`docs/perf/text-shaped-2026-11-12.md`). After A a Mount
row's text is 2 shapings, 2 breaks and 2 width-only answers: an
estimate from those two figures puts it at 110,000 to 125,000 of the
row's 189,000 layout instructions, the rest being Taffy's passes over
the row's 13 boxes and vapors' per-box records. A sampled profile of
the prototype would settle the split; TM1's PR should include one.

`--scaling Insert` (reported, not judged): layout 3,400 to 3,398 per
extra row and 3.52 M to 3.52 M per frame at 1,000 rows, unchanged, as
section 1.3 says. `--scaling Text`: pass; layout 89,908 to 90,388 at
10,000 rows, within noise.

What A does not change: a `Vertical` or `Both` query still breaks;
a run at a definite width (the value, `--scaling Text`) is unchanged;
`FlexText` is unchanged (section 1.3).

### B. A measure memo keyed by (run, available space, axis)

A third kept map from (shape key, axis, available width) to the
answer. After A, a width-only answer is one shape-key lookup and a
memoised pair of floats: a memo in front of it would save only the
key construction, which is C. Not recommended: a third `Kept` map
with its own sweep for no saving A and C do not give.

### C. The key built once per run

`Key::of` (`text_key.rs:70-77`) and `ShapeKey::of` (`text_key.rs:85-94`)
clone the run's text and spans and hash them on every cache call:
5 per label per layout (section 1.1) and one in paint, plus the
`ShapeKey` clone in `unwrapped` (`text_cache.rs:116`). The run's text
and styles are fixed for the life of its box, and `Collector::finish`
(`layout/inline.rs:84-96`) already owns the text, so the box can hold
its `ShapeParts` once, without the scale, in an `Rc`, and the shape
map can be keyed by `(Rc<ShapeParts>, scale bits)` with lookups by
reference. Each kept layout's key then shares the parts rather than
holding a copy of the text and spans (today both maps hold a copy per
entry). Expected saving: the clones, hashes and frees per call;
small per call, 6 per label per layout and 10,000 labels on Mount.
(measured in slice 2.)

### D. Changes in how Taffy is called

- Taffy's only way to give a leaf's intrinsic sizes is the measure
  function; there is no API to hand it min- and max-content widths
  up front, and `compute_leaf_layout` is already the leaf path vapors
  calls directly.
- A `min-width: 0` or `overflow: hidden` on items would skip the
  minimum-size query (that is why the value costs one break), but it
  is the page's CSS that decides those, not the framework. Rejected.
- Bypassing `compute_leaf_layout` for `Content::Text` on a
  `Horizontal` query (the anonymous text box has the default style:
  no padding, border, size or aspect ratio to resolve) would save the
  style resolution per query. Not worth a second leaf path; A's
  change is inside the closure.

### E. Upstream Taffy

Nothing here needs a Taffy change: the axis reaches the closure's
builder. A `TaffyTree` user (the high-level API, measure function
`(known_dimensions, available_space, node_id, context, style)`) has
no way to see the axis, and an upstream issue could ask for
`RequestedAxis` in the measure function's inputs, citing the cache's
own axis handling. The rule is an issue, never a drive-by PR
(`upstream-issues-not-prs`). Section 6 asks whether to file it; the
recommendation is not now.

### F. A vapors-side layout path for text-only rows

A row whose items are text-only could be laid out from content widths
without Taffy's flex passes. That reproduces flex semantics (gap,
stretch, min and max sizes, baseline alignment) beside the path the
Chromium oracle guards, for the part of a row's cost that is Taffy's
own passes over 13 boxes. After A and C the remainder is measurable;
if Taffy's passes dominate, the next design is about them (the
`Asked` record, `note_asked`, the per-node cache), not about text.
Recommend deferring until A and C are measured.

## 4. Correctness oracle

Must not change:

- The Chromium text comparisons (`crates/text/src/comparison`) and
  the layout fixtures (`cargo run -p vapors-elements-corpus --example
  layout_oracle -- --check`): every border box the same.
- The incremental oracle (`crates/elements/tests/support/oracle.rs`,
  `assert_matches_full`): `sparse_updates`, `incremental_random`.
- `layout_check.rs` in debug builds: every narrowed update laid out
  against a fresh tree with an empty text cache.

Added by slice 1:

- `content_widths` equals the broken widths over the corpus: for
  every run in the layout fixtures and `support/random_css.rs`'s pages
  (spans, weights, sizes, letter spacing, no-break spaces, a long
  unbreakable word, an empty-after-collapse run), at scales 1, 1.5
  and 2, `Shaped::content_widths()` equals the width of the layout
  broken at 0 and at no width. #253's `shaped_tests.rs` has this for
  three strings; the slice extends it to the corpus, because A's
  answer is exactly this equality.
- A `Horizontal` query never builds a layout: counts pinned on a
  flex-row page (`tests/shaped_once.rs`'s `rows`): per label
  `texts_shaped` 1, `text_layouts_built` 1, `texts_measured` 2.
  Proven by breaking: answering the `Horizontal` query by breaking
  fails the layouts count; answering `.max` for `.min` fails the
  corpus equality.
- `disowned` in the perf report and `LayoutWork` does not rise on
  Mount and `--scaling` (a placeholder that is not constant would show
  there first).
- The count pins the prototype moved, updated with the new counts and
  no other change: `sparse_updates`' `EXPECTED` ("resizing label"
  text layouts 2 to 1; `docs/plans/iu1.md`'s table with it),
  `incremental_text_widths`' clock tick (2 to 1: the clock's
  max-content and min-content answers no longer build layouts) and
  `shaped_once`'s "broken more than shaped" assertion, which becomes
  `text_layouts_built == texts_shaped` and `texts_measured == 2` per
  run. At the prototype these were the only 4 failures of the 1,210
  elements and text tests; the incremental oracle, `incremental_random`
  and the text comparisons passed, and `size_answers_disowned` stayed
  0.

## 5. Slices

| # | Slice | Content | Accepted when | Depends on |
|---|---|---|---|---|
| TM1 | Width-only queries answered from the shaped run | `measured_leaf` passes `inputs.axis`; `measure_text` takes the axis and answers `Horizontal` from `TextCache::content_widths` with `height: 0.0`; content widths memoised on the kept shaping; `layout(Some(W))` keeps the no-width layout when it fits and the run is reusable; `texts_measured` count in `text_counts.rs` and the perf counts line; the corpus equality test; counts pinned on a flex row, proven by breaking | Mount at 10,000 rows (`vapors-perf 3 1 --mount`, in the PR): `text layouts` 30,002 to 20,001, `texts shaped` 20,001 unchanged, layout instructions per extra row 205,095 to about 189,000 (the prototype's 189,282), anonymous KiB per row to about 47; the style and paint per-row counts explained if they move (the prototype's +0.2% and +2.4%); `vapors-perf 60 10 --scaling Text`, `FlexText` and `Insert` pass or report with `layout` within noise of before; `disowned` 0; the Chromium comparisons, `layout_oracle --check` and the incremental oracle unchanged; the 4 count pins of section 4 updated and no other test changed; the new counts proven by breaking | #254 merged |
| TM2 | The key built once per run | `ShapeParts` without the scale, owned by the run's box in an `Rc` from `Collector::finish`; the shape map keyed by `(Rc<ShapeParts>, scale)` and the layouts map by that and the width, both looked up by reference; `Key::of` gone | Mount layout instructions per row before and after in the PR; every count in TM1 unchanged; anonymous KiB per row not up; the incremental oracle unchanged; `keys_with_the_same_hash_and_different_parts_are_not_equal` kept | TM1 |

Both are inside `crates/elements/src/layout/` (`tree.rs`,
`measure.rs`, `text_cache.rs`, `text_key.rs`, `text_counts.rs`) and
`crates/text/src/shaped.rs`; no overlap with the U and B slices of
`docs/plans/build-time-styles.md` (`style/`, `restyle_pass.rs`,
`crates/wisp-*`).

## 6. Decisions for Tom

1. **When TM1 runs.** (a) Now, beside U1 and U2: no shared files, a
   PR of about 60 lines of change plus tests, measured at 2.5% of a
   Mount frame's instructions per row and 7.7% of its layout, and
   the same off every new row's text (property 3 once the splice
   step is built). (b) After the U slices, so property 7's first
   published time has the style change alone. **Recommend (a).**
2. **An upstream Taffy issue** proposing `RequestedAxis` in the
   measure function's inputs (option E). (a) File it now, as a note
   for `TaffyTree` users. (b) Not now: vapors does not need it, and an
   issue without a user waiting on it is noise. **Recommend (b).**
3. **Option F, a vapors-side path for text-only rows.** (a) Design it
   after TM1 and TM2's measurements, if Taffy's passes are then the
   larger part of layout per row. (b) Not at all: the Chromium oracle
   guards one layout path. **Recommend (a): decide on the measurement,
   not now.**
4. **T1's 2 KiB per row of kept shapings.** #254 names an alternative
   (keep shapings only for the update that made them) if property 8
   needs the memory back. TM1 drops one kept layout per row, which
   returns part of it. (a) Measure in TM1 and revisit only if
   property 8's budget, once set, is short. (b) Take the alternative
   in TM1. **Recommend (a).**
