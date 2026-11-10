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
  focus. Reports the existing counters per edit and asserts them equal
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
  | resizing label | 0 | 9 | 3 |
  | equal write | 0 | 0 | 0 |
  | hover | 4 | 0 | 0 |
  | focus | 1 | 0 | 0 |

  These are cache misses. Each update still walks the whole tree; a
  debug-build update with nothing changed takes about 0.3 s at 3000 rows
  and a full one about 5 s. PR 2 counts that walk.

## PR 2: traversal and paint counters

Structural counts per update, beside `elements_restyled`,
`boxes_laid_out` and `text_layouts_built`: elements the style phase
visits, boxes built and boxes Taffy visits (cache hits included),
boxes painted, and paint records (scene items) rebuilt. Decision
(lead): a public `Document::update_counts()` returning a
`#[non_exhaustive]` struct. Its doc says the counts describe the last
update, are structural rather than timings, and when they reset; each
field's doc says exactly what one count is. The existing three getters
stay. PR 2 lists it in docs/design/incremental-contracts.md section f.
`sparse_updates.rs` reports them; IU3 then shows which counts its
retained box tree removes.

## PR 3: GPU counters

Upload bytes and draw calls per frame in `ScenePainter`
(`crates/app`), if they can be counted without a synchronous GPU wait.

## PR 4: timing baseline

Recorded in `docs/`: revision, profile (release), hardware and backend,
scale, warmup, sampling; per-phase distributions (median, p90, max), not
averages. No synchronous GPU waits in the normal path; GPU time only
where timestamp queries exist. Timing is reported, never asserted in CI.

## PR 5 on: the remaining workloads

Initial mount and teardown, typing, paint-only and inherited-style
changes, keyed insertion, removal, reorder and filtering, scrolling,
viewport resize, an animated GPU surface beside unchanged UI, fine
item signals against one coarse collection signal, and virtualized
lists separately. Each goes through the oracle.
