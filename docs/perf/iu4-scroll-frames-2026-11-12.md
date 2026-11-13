# IU4 4-ii: a scroll in scroll-frame coordinates

`vapors-perf 60 10 --scaling Scroll`, release, median instructions per
frame, one run each. Before: main 00678dc. After: the 4-ii branch on R6
PR 5a (#243), whose a11y mirror bounds are relative to the scroll
container (placement and paint in frame coordinates,
`docs/plans/iu4.md` section 3 and "4a to 4c land together"). Load
average 21 (before) and 11 (after) at the start; the counts do not move
with load.

| phase | before, 1000 rows | before, 10,000 rows | after, 1000 rows | after, 10,000 rows | after, per extra row |
|---|---|---|---|---|---|
| update | 27,358,066 | 284,030,825 | 82,099 | 442,158 | 40 |
| layout | 1,156,906 | 11,446,370 | 46,487 | 406,487 | 40 |
| paint | 25,426,910 | 265,556,711 | 21,605 | 21,633 | 0.0 |
| accessibility | 11,564,581 | 116,012,141 | 7,039 | 7,039 | 0.0 |
| draw | 2,866,917 | 2,873,543 | 4,271,674 | 16,779,012 | 1,390 |
| frame (CPU) | 41,815,167 | 402,944,779 | 4,386,897 | 17,254,218 | 1,430 |

At 3000 rows the update phase falls from 83.4 million instructions to
162,212 and paint from 77.9 million to 21,698: a scroll settles the
body's offset (`boxes_placed` 2, html and body), and moves, paints and
shifts no box.

What still grows with the rows on a scroll:

- draw, about 1,390 per row: a frame's shift changed, so the kept
  batches are made again from every drawing step at the next draw
  (`KeptBatches::of`, O(steps + items)); before, the scroll patched
  every row's steps during paint instead, and draw read kept batches.
  IU4 PR 5 (the extents index) makes this O(log n + shown).
- layout, 40 per row: not profiled here; it was 1,144 per row before
  (the placement walk over every row).
