# W6, the positioned rows page: first numbers (2026-11-17)

W6 is a workload from `docs/plans/flat-scene.md` section 6. It runs on
the rows page with every seventh row
`position: relative; z-index: 1` and a fixed footer, so paint order is
not tree order. The page is the elements tests'
`support/positioned_rows.rs`; `crates/elements/tests/positioned_rows.rs`
checks its edits against a full update, and pins that its positioned
rows are stacked. Two `--scaling` changes run on it:

- `PositionedClass`: the `marked` class toggled on the positioned row
  at or before the middle row, as `Class` does on the plain page.
- `PositionedInsert`: a plain row inserted before the middle row, then
  removed, as `Insert` does.

Method: `cargo build --profile judged -p vapors-perf`, then
`target/judged/vapors-perf 60 10 --scaling <change>`. The run used
Intel UHD 630 (ANV), 640 x 400, scale 1, and took the median
instructions per frame from one run. The tree was main at 9547e1a4
with this change.

## PositionedClass: flat, judged as Class

| phase | kind | 1000 rows | 3000 rows | 10,000 rows | per extra row, 3000 to 10,000 |
|---|---|---|---|---|---|
| style | add | 422,178 | 417,537 | 404,365 | -1.9 |
| layout | add | 78,041 | 79,908 | 81,413 | 0.2 |
| paint | add | 82,896 | 82,847 | 83,714 | 0.1 |
| accessibility | add | 19,255 | 18,802 | 18,819 | 0.0 |
| draw | add | 975,932 | 975,388 | 975,505 | 0.0 |
| frame (CPU) | add | 1,616,219 | 1,612,208 | 1,602,138 | -1.4 |

Remove frames are within 0.6% of these numbers. Every phase is within
2% or 5000 instructions of its 1000-row count, so the change is judged
`EveryPhase`, as `Class` is.

## PositionedInsert: not flat, reported with an owner

| phase | kind | 1000 rows | 3000 rows | 10,000 rows | per extra row, 3000 to 10,000 |
|---|---|---|---|---|---|
| style | insert | 387,317 | 372,588 | 357,543 | -2.1 |
| layout | insert | 8,307,748 | 27,714,614 | 98,694,296 | 10,140.0 |
| paint | insert | 12,028,265 | 37,400,390 | 129,494,170 | 13,156.3 |
| accessibility | insert | 7,058,562 | 20,825,338 | 69,410,577 | 6,940.7 |
| update | insert | 21,114,878 | 66,495,473 | 232,031,234 | 23,648.0 |
| frame (CPU) | insert | 29,286,729 | 88,432,444 | 302,554,370 | 30,588.8 |

Remove frames are within 2% of these numbers. Every frame spliced its
paint order (71 splices at each size). No splice was patched.

The cause is the fixed footer. It is an out-of-flow child of the body,
so the body's lane is declined (`LaneDecline::OutOfFlowChild`), and the
insert falls back to the costs from before the lane: a per-row layout,
paint and accessibility term. Running the same insert on the page
without the footer gives these results:

- accessibility: 1220.2 and 1212.7 per extra row on insert frames,
  1237.3 and 1227.4 on remove frames;
- layout and paint: within 1.1 per row;
- the change passes Insert's rule (style flat, accessibility under
  1250).

The positioned rows themselves add no per-row term.

Judging is therefore off for `PositionedInsert`
(`Judged::Reported`). Its owner is splice-layout slice 3b, which
places a lane container's absolute and fixed children from the tree;
3b judges this change as `Insert`
(`crates/perf/src/scaling_judged.rs`, `POSITIONED_INSERT_OWNER`).

## Counts a later slice must move

- 3b: `PositionedInsert`'s layout, paint and accessibility terms, down
  to Insert's (accessibility under 1250 per row, the rest flat). The
  body is then laid out by its lane with the footer in it.
- F3 and later flat-scene slices: `PositionedClass` stays flat.
