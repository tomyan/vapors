# F3 baselines: batch counts before clips as data (2026-11-17)

The counts `docs/plans/flat-scene-f3.md`'s acceptance compares F3.1
against, taken on main at the start of F3.1, before anything in the
painter changes. They come from the off-path pages that stand for W1
and W4 today: "nested scrollers" (a scroller of scrollers, W1's shape)
and "code blocks" (blocks that each scroll, in a page scroller, W4's
shape). Command: `vapors-perf 20 5 --off-path --only "<page>"`,
release, Intel UHD 630; the outputs are in `f3-baselines-2026-11-17/`.
The counts are of the last frame of each case.

| page | case | draw calls | batches written | batches kept |
|---|---|---|---|---|
| nested scrollers (1,000 to 10,000) | Edit | 35 | 0 | 35 |
| | Scroll | 35 | 2 | 33 |
| | ScrollInner | 35 | 1 | 34 |
| | Insert | 35 | 0 | 35 |
| code blocks (1,000 to 10,000) | Edit | 14 | 0 | 14 |
| | Scroll | 17 | 3 | 14 |
| | ScrollInner | 14 | 1 | 13 |
| | Insert | 14 | 0 | 14 |

Every count is the same at 1,000, 3,000 and 10,000: the draws are of
what shows, not of the page.

## The prediction for F3.1

F3.1 joins batches across clip changes within one frame only (the
note's item 10). On both pages every scroller is a frame of its own,
and within a frame the items share one clip, so no run is split by a
clip change inside a frame today. The prediction: F3.1 leaves these
counts unchanged (35 on nested scrollers; 14, and 17 on Scroll, on code
blocks). Fewer draws on these pages need batches joined across frames,
which is F3.3's, on a measurement of draws against the fragment work a
wider scissor adds.
