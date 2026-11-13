# Insert and Resize, each kind of frame apart (2026-11-13, main at ed4e4704)

PR 0 of `docs/plans/splice-layout.md`. `--scaling Insert` alternates an
insert and a remove each frame, `Resize` a taller and a shorter second
row, `ScrollSize` a longer and a shorter label, `Class` and `HoveredClass`
the middle row's `marked` class added and removed. Before this change the
table was one median over both kinds, which is the cheaper kind's
slowest frame. `--scaling` now prints a table per kind and judges each.

Method: `vapors-perf 60 10 --scaling <change>`, release, Intel UHD 630
(ANV), 640 x 400, scale 1, median instructions per frame, one run each
(instruction counts repeat within 0.1% between runs, section 1.1 of the
plan). The "one median" rows are the harness at ed4e4704 before this
change, run the same hour.

## Insert

| phase | kind | 1000 rows | 3000 rows | 10,000 rows | per extra row, 3000 to 10,000 |
|---|---|---|---|---|---|
| update | one median | 15,210,277 | 46,164,247 | 157,792,498 | 15,947 |
| update | insert | 15,835,154 | 46,764,505 | 158,389,225 | 15,946 |
| update | remove | 15,202,541 | 46,147,133 | 157,770,781 | 15,946 |
| style | one median | 59,814 | 151,279 | 467,898 | 45 |
| style | insert | 620,214 | 721,028 | 1,036,008 | 45 |
| style | remove | 59,757 | 150,876 | 467,510 | 45 |
| layout | insert | 3,566,972 | 10,230,953 | 33,914,917 | 3,383 |
| layout | remove | 3,507,225 | 10,178,879 | 33,873,899 | 3,385 |
| paint | insert | 11,252,775 | 34,796,679 | 119,916,813 | 12,160 |
| paint | remove | 11,239,173 | 34,800,809 | 119,908,897 | 12,158 |
| accessibility | insert | 9,745,623 | 28,582,183 | 96,061,917 | 9,640 |
| accessibility | remove | 9,698,538 | 28,451,437 | 95,686,591 | 9,605 |
| frame (CPU) | insert | 26,635,551 | 76,404,418 | 255,511,610 | 25,587 |
| frame (CPU) | remove | 25,955,949 | 75,656,405 | 254,486,397 | 25,547 |

Reading: the one median was the remove frames' (style matches them to
the instruction). An insert costs about 630,000 instructions more than a
remove at every size, almost all of it in `style` (the new row's
cascade, about 560,000); the per-row growth of every phase is the same
for both kinds. The costlier kind's per-row cost is what the lane
removes, and it is the same for both, so the plan's section 1 numbers
stand.

## Resize

| phase | kind | 1000 rows | 3000 rows | 10,000 rows | per extra row, 3000 to 10,000 |
|---|---|---|---|---|---|
| update | one median | 33,387,523 | 100,900,341 | 344,387,407 | 34,784 |
| update | taller | 33,380,024 | 100,881,240 | 343,627,328 | 34,678 |
| update | shorter | 33,527,298 | 101,354,427 | 345,762,472 | 34,915 |
| layout | taller | 2,544,604 | 7,341,577 | 24,187,807 | 2,407 |
| layout | shorter | 2,544,849 | 7,357,461 | 24,203,090 | 2,407 |
| paint | taller | 29,523,751 | 90,982,555 | 311,872,484 | 31,556 |
| paint | shorter | 29,671,610 | 91,438,031 | 313,996,154 | 31,794 |
| accessibility | taller | 13,464,641 | 39,916,055 | 134,693,545 | 13,540 |
| accessibility | shorter | 13,463,950 | 39,915,930 | 134,692,383 | 13,540 |

Reading: the two kinds are within 1% of each other in every phase.

## The accessibility ceilings fail on main

Both runs exit non-zero on main before this change and after it, on
the accessibility ceilings of `crates/perf/src/scaling_judged.rs`:
Insert's (9500 per extra row) at 9,538 one median (9,591 insert frames,
9,554 remove frames, 1000 to 10,000 rows), and Resize's (13,000) at
13,197 (1000 to 3000 rows) and 13,443 (1000 to 10,000 rows) one median, 13,470 (1000 to 10,000) for both taller and shorter frames. The ceilings were set over the
per-row costs recorded on 2026-11-12 (`docs/perf/a11y-phase-2026-11-12.md`:
8,859 and 12,021); the take's cost per row has grown by about 8% and
12% since, past the room the ceilings left. This change does not move them; the lane's PR 5
(the mirror in flow) replaces both with `EveryPhase`.

## Class and HoveredClass: the add frames grow with the page

Split the same way (review of #270, M1), `--scaling Class` and
`--scaling HoveredClass` fail, where the one median passed: adding the
`marked` class costs `paint` 13.6 more instructions per extra row
(Class: 67,987 / 95,434 / 190,397 at 1000 / 3000 / 10,000 rows;
HoveredClass: 74,979 / 102,650 / 198,088), and `update` and `frame
(CPU)` with it. Removing it is flat (Class `paint` 53,954 / 54,452 /
54,895). The one median was the remove frames'. Which paint step reads
something per row when a class is added is not located here; property
1 of `docs/plans/perf-finish-line.md` is judged by these runs and does
not hold for the add frames.

| Class | 1000 rows | 3000 rows | 10,000 rows | per extra row, 3000 to 10,000 |
|---|---|---|---|---|
| update, add | 760,533 | 787,720 | 883,613 | 13.7 |
| update, remove | 745,721 | 746,360 | 747,653 | 0.2 |
| paint, add | 67,987 | 95,434 | 190,397 | 13.6 |
| paint, remove | 53,954 | 54,452 | 54,895 | 0.1 |
