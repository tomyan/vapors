# F3 baselines: batch counts before clips as data (2026-11-17)

The counts `docs/plans/flat-scene-f3.md`'s acceptance compares F3.1
against, taken at main e703e267 (after #457), before anything in the
painter changes; the working tree held #461's elements changes, which
draw nothing differently. They come from the off-path pages that stand for W1
and W4 today: "nested scrollers" (a scroller of scrollers, W1's shape)
and "code blocks" (blocks that each scroll, in a page scroller, W4's
shape). Command: `vapors-perf 20 5 --off-path --only "<page>"`,
release, Intel UHD 630; the outputs are in `f3-baselines-2026-11-17/`.
The counts are of the last frame of each case. Load average at the
start was 10 to 12; counts do not depend on load, only times do.

`crates/perf/tests/gpu_f3_baselines.rs` pins every count in the table
at 1,000 units, one GPU-tier test per page and change. The command
counts frame 24 of a sampled run, after 25 changes; the tests reach the
same frame counted alone (`crates/perf/src/counted_frames.rs`): 24
changes with no frame between them, a priming frame, the 25th change
and the measured frame, with no timing. Their counts equal the sampled
run's (checked when they were switched). The scroll changes' counts
depend on how far they have scrolled, so a different number of changes
or page size gives other numbers. A change to the painter that moves a
count fails there, and this table changes with it.

## A page where clip changes split runs

The rows page (`crates/perf/src/rows.rs`) has a clip change inside a
frame on every row: each row is a lane frame, and its value's text is
under the value's own clip, between the label's and the button's
unclipped texts. Its idle frame at 300 rows draws 52 calls: 35 text
draws (18 unclipped runs, one per row boundary in view plus the first
and last, and one per value for the 17 rows in view) and 17 quad
draws, one per button. `crates/perf/tests/gpu_rows_draws.rs` pins them.
This is the page F3.1's join of batches across clips within a frame
(step 2) is measured on: its text draws should fall when the values'
runs join their row's unclipped ones.

## The prediction for F3.1

F3.1 joins batches across clip changes within one frame only (the
note's item 10). On both pages every scroller is a frame of its own,
and within a frame the items share one clip, so no run is split by a
clip change inside a frame today. The prediction: F3.1 leaves these
counts unchanged (35 on nested scrollers; 14, and 17 on Scroll, on code
blocks). Fewer draws on these pages need batches joined across frames,
which is F3.3's, on a measurement of draws against the fragment work a
wider scissor adds.
