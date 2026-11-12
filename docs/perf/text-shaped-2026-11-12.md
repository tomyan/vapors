# Text shaped once per run content, 2026-11-12

Slice T1 of `docs/plans/build-time-styles.md` (#245). The plan's T1
row expected the text cache to be keyed by node, so that the 10,000
"Edit" buttons on Mount would each be shaped. They were not: the cache
key already held the run's text, styles, width and scale, and the node
only through the run's inline elements (`SpanKey::node`), which Mount's
runs do not have. "Edit" was shaped twice per page, at two widths.

What did repeat per row was shaping one run at several widths. Taffy
measures each row's label, a flex item, at max-content (no wrap) and
min-content (width 0), and each width was a new Parley layout, shaped
from scratch. So T1 became: shape a run once per text, styles,
visibility, span positions and scale (`ShapeKey`, no node), keep the
shaped run, and break a copy of it at each width
(`vapors_text::Shaped::lay_out`). `texts shaped` is counted beside
`text layouts` in the perf report.

Min-content and max-content widths can be read from a shaped run
without breaking it (`Shaped::content_widths`, Parley's
`calculate_content_widths`). The measure function still breaks: Taffy
asks it for a width and a height together, and the height at
min-content needs the lines.

## Method

Release build, `cargo run --release -p vapors-perf -- 3 1 --mount` and
`-- 5 1 --scaling Text`, before at origin/main 00678dc and after on
`perf/t1-text-cache`, the same machine. Instruction counts do not move
with load. The load average ran from 9 to 31 during these runs. Times
are not kept. The counts are from the last kept page's first update.

## Mount, 10,000 rows

| | before | after |
|---|---|---|
| text layouts | 30,002 | 30,002 |
| texts shaped | not counted (30,002: one per layout) | 20,001 |
| shapes per row | 3 | 2 |
| layout instructions, 10,000 rows | 2,484,799,268 | 2,041,493,534 |
| layout instructions per extra row, 3000 to 10,000 | 249,815 | 205,087 |
| frame (CPU) instructions per extra row, 3000 to 10,000 | 623,999 | 579,600 |
| anonymous KiB per extra row after the first frame, 3000 to 10,000 | 48.8 | 50.8 |

The 20,001 runs shaped are each row's label and value and one "Edit".
Layout fell by 18% per row. The memory per row rose by 2.0 KiB: the
shaped runs are kept beside the layouts broken from them, under the
same bound (`text_cache.rs`).

## `--scaling Text` (one text edited per frame)

| | before | after |
|---|---|---|
| layout instructions, 10,000 rows | 86,570 | 89,908 |
| update instructions, 10,000 rows | 125,442 | 128,831 |
| per extra row | 0.0 | 0.1 |

The edited run is shaped once and broken once, as before. The copy of
the shaped run that breaking takes is about 3,300 instructions more per
edit. The phase stays flat in rows, and the scaling check passes.
