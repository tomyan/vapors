# IU1: baseline and correctness oracle

Slice 1 of docs/briefs/INCREMENTAL_UI_PERFORMANCE_BRIEF.md, as a stack
of pull requests under about 400 lines each.

## PR 1: oracle and sparse-update workload (done)

- `crates/elements/tests/support/oracle.rs`: `assert_matches_full`
  compares the settled incremental result with the same document after
  `Document::forget_kept`: border boxes, computed styles, display list in
  paint order with clips, hit element and cursor on a 10 px grid, focus,
  text field selections.
- `crates/elements/tests/sparse_updates.rs` with
  `support/rows_page.rs`: 100 and 1000 rows, the oracle after each edit;
  a same-size value edit, a resizing label edit, an equal write, hover,
  focus, a class toggled on one row (`ToggleClass`, the `.row.marked`
  background, added in PR 4). Reports the existing counters per edit and asserts them equal
  to the table below (`EXPECTED` in the test). A change to a count fails
  the test; the table is updated by hand with `EXPECTED`.
- The baseline run is `VAPORS_SPARSE_ROWS=3000 cargo nextest run -p
  vapors-elements --test sparse_updates`, off by default: 3000 rows took
  23.3 s alone and 29.2 s at -j4 on this machine (2026-11-10, debug),
  against the 30 s slow mark and 60 s kill in `.config/nextest.toml`.
  It checks the oracle once, after the five edits. The 1000-row test
  with the oracle after each edit took 20.4 s alone.
- The oracle compares each text layout's string, size and every glyph
  run (size, colour, glyph ids and positions). Proof: with the colour
  left out of `StyleKey::of` in `text_cache.rs`,
  `VAPORS_INCREMENTAL_SEEDS=30 incremental_random` fails at seed 2,
  step 9.
- `forget_kept` keeps scroll offsets. PR 5's scrolling workload must
  snapshot them before `forget_kept` and restore them, or the oracle's
  full update differs for the wrong reason.
- Counts on this machine (2026-11-10), the same at every row count:

  | Edit | restyled | boxes laid out | text layouts built |
  |---|---|---|---|
  | same-size value | 0 | 5 | 1 |
  | resizing label | 0 | 9 | 2 |
  | equal write | 0 | 0 | 0 |
  | hover | 4 | 0 | 0 |
  | focus | 1 | 0 | 0 |
  | class toggled | 4 | 0 | 0 |

  These are cache misses. Each update still walks the whole tree; a
  debug-build update with nothing changed takes about 0.3 s at 3000 rows
  and a full one about 5 s. PR 2 counts that walk.

## PR 2: traversal and paint counters (done)

`Document::update_counts()` returns `UpdateCounts`
(`crates/elements/src/update_counts.rs`), `#[non_exhaustive]`, beside
`elements_restyled`, `boxes_laid_out` and `text_layouts_built`, which
stay. Every count resets when an update starts and each phase sets its
own, so a refused update reports the phases it ran. The fields, each one
increment per unit:

- `elements_visited`: elements the style phase reads kept styles for
  after restyling, one per element in the tree (the loop in
  `Document::styled`).
- `boxes_built`: layout boxes `build` made, one per element box and
  text run.
- `layout_calls`: calls Taffy makes to a box's layout, cache hits
  included (`BoxTree::lay_out`). Not "boxes Taffy visits": a box can be
  asked more than once, and a box under a cache hit is not asked.
- `boxes_painted`: `Step::Box` steps paint drew (outlines and thumbs
  not included).
- `scene_items_built`: the scene's items. Paint builds the scene from
  empty each update, so this is every item; there are no kept records to
  count separately yet.

`crates/elements/tests/update_counts.rs` holds each count on an update
with nothing changed and at 0 behind an earlier refusal (an unparsed
style attribute, a refused tag, a calc that fails at layout).
`sparse_updates.rs` asserts them per edit at 100 and 1000 rows
(`EXPECTED_WALKS`). Counts on this machine (2026-11-10):

| Edit | rows | elements visited | boxes built | layout calls | boxes painted | scene items |
|---|---|---|---|---|---|---|
| same-size value | 100 | 402 | 702 | 109 | 702 | 400 |
| resizing label | 100 | 402 | 702 | 111 | 702 | 400 |
| equal write | 100 | 402 | 702 | 1 | 702 | 400 |
| hover | 100 | 402 | 702 | 1 | 702 | 400 |
| focus | 100 | 402 | 702 | 1 | 702 | 401 |
| class toggled | 100 | 402 | 702 | 1 | 702 | 402 |
| same-size value | 1000 | 4002 | 7002 | 1009 | 7002 | 4000 |
| resizing label | 1000 | 4002 | 7002 | 1011 | 7002 | 4000 |
| equal write | 1000 | 4002 | 7002 | 1 | 7002 | 4000 |
| hover | 1000 | 4002 | 7002 | 1 | 7002 | 4000 |
| focus | 1000 | 4002 | 7002 | 1 | 7002 | 4001 |
| class toggled | 1000 | 4002 | 7002 | 1 | 7002 | 4002 |

A row is 4 elements and 7 boxes and draws 4 items. What the counts show
about the idle update (the equal write, where nothing is restyled, laid
out or rebuilt as text): the style phase visits every element, `build`
makes every box again and paint draws every box and rebuilds every
item, all linear in rows. Taffy's calls are not part of that walk: it
asks the root once and reads its cache (`layout_calls` counts only
those calls). `KeptBoxes::lend`, `placed`, `scroll.settle` and
`Chains::of` still walk every box on every update, and paint's rect and
padding loop does too; none of these is counted. A later counter may
cover them if IU3 needs it. An edit inside one row makes rows + 9
or rows + 11 layout calls: one per row, which fits the body asking each
row for its layout and reading its cache, plus the edited row's
chain. The retained box tree in IU3 targets
`boxes_built` and, with paint reuse, `boxes_painted` and
`scene_items_built`; `elements_visited` needs the style phase to stop
reading every element.

## PR 3: GPU counters

Upload bytes and draw calls per frame in `ScenePainter`
(`crates/app`), if they can be counted without a synchronous GPU wait.

Done: `ScenePainter::frame_counts()` (and `ElementsApp::frame_counts()`)
returns `FrameCounts`, the last frame's `GpuWork` per run kind: bytes
passed to `write_buffer`, bytes passed to `write_texture`, and `draw`
calls, counted where the pipelines issue them (`WorkCounter`, in
`vapors-renderer`), with no readback. Not counted: the clear, and what
a `gpu-surface`'s own renderer does.

What a frame with nothing changed uploads and draws today, at 640x400,
scale 1, measured at 94ff557 plus this PR (the second of two identical
frames). Reproduce with `cargo run -p vapors-app --example frame_counts`,
which prints the first and the idle frame of each demo; the counter's idle
row is also pinned by `gpu_frame_counts_kinds.rs`:

| Demo | Draw calls | Buffer bytes | Texture bytes | First frame's texture bytes |
|---|---|---|---|---|
| hello | 4 (2 quad runs, 2 text items) | 26 624 | 0 | 1 053 856 |
| counter | 3 (2 quad runs, 1 text item) | 2 800 | 0 | 1 048 576 |
| stress | 1 803 (901 quad runs, 902 text items) | 820 784 | 0 | 1 051 420 |

So an idle frame re-uploads every run's instances or vertices and
redraws every run: one draw per quad or shadow run, one per text item,
image and surface, one per icon run. The atlas is not uploaded when no
glyph was added. The first upload of a page is the whole 1024 x 1024
page (1 048 576 bytes); after that, a text item or icon run that adds
glyphs uploads the rectangle bounding the glyphs it added, and a repack
uploads the whole page again. Before the region uploads (measured at
94ff557), every text item that added a glyph uploaded the whole page:
hello's first frame was 2 097 152 bytes and stress's 44 040 192 (42
pages). The first-frame column is from a rerun at 1a102da plus the
region-upload PR. These are the numbers IU4's retained paint targets.

## PR 4: timing baseline

Recorded in `docs/`: revision, profile (release), hardware and backend,
scale, warmup, sampling; per-phase distributions (median, p90, max), not
averages. No synchronous GPU waits in the normal path; GPU time only
where timestamp queries exist. Timing is reported, never asserted in CI.

Done: `crates/perf` (vapors-perf). The baseline and its findings are in
`docs/perf/baseline-2026-11-10.md`: draw is 87% to 95% of every frame.
The painter makes an encoder, a render pass and a submit per draw call;
measured per call, `Queue::submit` takes about 35 µs, the encoder, pass
and bind group about 25 µs, and buffer writes about 10 µs.

## PR 5 on: the remaining workloads

Initial mount and teardown, typing, paint-only and inherited-style
changes, keyed insertion, removal, reorder and filtering, scrolling,
viewport resize, an animated GPU surface beside unchanged UI, fine
item signals against one coarse collection signal, and virtualized
lists separately. Each goes through the oracle.
