# W6, the positioned rows page: first numbers (2026-11-17)

W6 is a workload from `docs/plans/flat-scene.md` section 6. It runs on
the rows page with these additions, so paint order is not tree order:

- every seventh row `position: relative; z-index: 1`;
- an absolutely positioned box;
- a fixed footer.

`position: sticky` is refused by vapors (`style/position.rs`), so the
page has no sticky header.

The page is the elements tests' `support/positioned_rows.rs`, and
`crates/elements/tests/positioned_rows.rs` checks it. Three `--scaling`
changes run on it:

- `PositionedClass`: the `marked` class toggled on the positioned row
  at or before the middle row, as `Class` does on the plain page.
- `PositionedInsert`: a plain row inserted before the middle row, then
  removed, as `Insert` does.
- `PositionedStackedInsert`: a positioned row inserted before the
  middle row, then removed. Its splice declines for the row's stacking
  (`InsertedStacked`, then `FreedStacked`), and paint order is walked.

Method: `cargo build --profile judged -p vapors-perf`, then
`target/judged/vapors-perf 60 10 --scaling <change>`, three runs of
each change. The tables give the median instructions per frame of the
three runs; the spread is the largest minus the smallest 10,000-row
count. The setup was Intel UHD 630 (ANV), 640 x 400, scale 1. Load
averages at the start were 7 to 16; instruction counts do not move with
load. The tree was main at 9547e1a4 with this change. Remove frames
are within 2% of insert frames for every phase but `style`.

## PositionedClass: flat, judged as Class

| phase | kind | 1000 rows | 3000 rows | 10,000 rows | per extra row, 3000 to 10,000 | spread |
|---|---|---|---|---|---|---|
| style | add | 416,933 | 400,071 | 400,869 | 0.1 | 882 |
| layout | add | 80,582 | 81,993 | 81,777 | -0.0 | 439 |
| paint | add | 82,896 | 83,009 | 83,787 | 0.1 | 40 |
| accessibility | add | 18,766 | 18,814 | 18,820 | 0.0 | 14 |
| draw | add | 975,791 | 975,627 | 974,401 | -0.2 | 482 |

The splice counts are zero: there are no splices, declines or walks. The
change is judged `EveryPhase`.

## PositionedInsert: reported, owner splice-layout 3b

| phase | kind | 1000 rows | 3000 rows | 10,000 rows | per extra row, 3000 to 10,000 | spread |
|---|---|---|---|---|---|---|
| style | insert | 384,332 | 350,438 | 356,571 | 0.9 | 698 |
| layout | insert | 8,312,266 | 27,724,256 | 98,788,481 | 10,152.0 | 39,948 |
| paint | insert | 12,036,697 | 37,384,456 | 129,493,899 | 13,158.5 | 1,934 |
| accessibility | insert | 7,022,189 | 20,660,961 | 68,881,447 | 6,888.6 | 129,226 |

Splice counts at each size: 71 splices, 0 patched, no declines, no
order walks.

The cause is the out-of-flow boxes under the body (the footer and the
absolutely positioned box), which decline the body's lane. Two elements
tests pin this:

- `the_out_of_flow_boxes_cost_the_body_its_lane`: with the rows
  positioned but no out-of-flow boxes, the body's lane frames its rows.
  With the boxes, it frames none and one more container is declined.
- `a_fixed_child_of_the_body_declines_its_lane_as_out_of_flow`: the same
  fixed footer gives `LaneDecline::OutOfFlowChild`.

Reproduce with
`cargo test -p vapors-elements --test tests positioned_rows`.

Without the out-of-flow boxes, the same insert passes Insert's rule
(three runs, medians):

- accessibility: 1211.0 per extra row on insert frames, 1225.8 on remove
  frames, from 3000 to 10,000 rows;
- layout and paint: within 1.0 per row.

That run is made by replacing `positioned_rows::make_positioned` with
`positioned_rows::position_rows` in `Rows::new` (`crates/perf/src/rows.rs`)
and running the same command. On that page the judge fails the run,
because the change now passes its target rule ("now passes the rule it
was reported for"). That is the ratchet in `scaling_reported.rs`.

## PositionedStackedInsert: reported, owner splice slice D

| phase | kind | 1000 rows | 3000 rows | 10,000 rows | per extra row, 3000 to 10,000 | spread |
|---|---|---|---|---|---|---|
| style | insert | 396,326 | 391,070 | 356,326 | -5.0 | 858 |
| layout | insert | 8,318,224 | 27,694,417 | 98,619,794 | 10,132.2 | 25,843 |
| paint | insert | 21,033,779 | 64,849,163 | 222,883,151 | 22,576.3 | 7,567 |
| accessibility | insert | 7,013,609 | 20,598,579 | 68,915,559 | 6,902.4 | 8,930 |

Splice counts at each size and in each run: 0 splices; declined
`FreedStacked` 35 and `InsertedStacked` 36; 71 order walks. Layout and
accessibility are the same as `PositionedInsert`'s, from the same lane
decline. Paint is about 9,400 per extra row higher: the walk of the
whole paint order that each declined splice makes. No slice made a
stacked box's splice flat, so `docs/plans/splice.md` now has slice D
(stacked splice) as its owner. D judges this change as `Insert`
together with splice-layout 3b.

## The absolutely positioned box

Adding the absolutely positioned box (review of #452, L1) left every
figure within run-to-run noise. A first run without it measured layout
10,140.0, paint 13,156.3 and accessibility 6,940.7 per extra row for
`PositionedInsert`; with it, the figures are 10,152.0, 13,158.5 and
6,888.6. The body's lane is already declined for the footer.

## Counts a later slice must move

- splice-layout 3b: the layout and accessibility terms of both insert
  changes. `PositionedInsert` then passes Insert's rule, and the run
  fails until it is judged by it.
- splice D, after 3b: `PositionedStackedInsert`'s splice declines become
  splices and its order walks go to 0, so it passes Insert's rule.
- `PositionedClass` stays flat.
