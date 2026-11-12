# Mount and first frame with shared styles, 2026-11-12

Analysis PR 2 (finish-line properties 7 and 8), after PRs 2a to 2d
(vapors #235, #237, #238, #239): the compiler gives each template
element a `StyleKey`, and the tree keeps one computed style per share
key (key, parent style, states, toggled classes, selected attributes).
The workload, run and hardware are those of `mount-2026-11-12.md`
("before" below is that file's run at adc1973). Two runs with sharing:

- "first": at 3194f40 (PR 2e's first commit: 813bb4d plus the
  report's styles line), before the review fix rounds of #235 to #239;
  load average 14.8 at the start.
- "after fixes": at 01a5807 (PR 2e with #238's and #239's fix rounds
  merged in: `@container` rules block sharing, the selected state
  flags, the parent in the share key pinned by tests).

Release build, the same command for each:
`cargo run --release -p vapors-perf -- 9 --mount`. Instruction counts
and resident sets do not move with load; no times are kept. Memory is
one process per size, one run.

## Where the styles came from, first update

The counts are the last kept page's; each page is a new app, so its
frame is a first frame and its update the first update. Both runs gave
the same counts.

| rows | restyled | shared | computed | unkeyed | unshareable | live |
|---|---|---|---|---|---|---|
| 1000 | 4002 | 3996 | 4 | 2 | 0 | 6 |
| 3000 | 12002 | 11996 | 4 | 2 | 0 | 6 |
| 10000 | 40002 | 39996 | 4 | 2 | 0 | 6 |

`restyled` is as before (the walk visits every element). The four
computed styles are the row's four keys (`div.row`, `span.label`,
`span.value`, `button`) under one parent; `html` and `body` are
unkeyed. `live` counts `ComputedStyle` allocations only (`Styles::alloc`);
it is flat in rows. `mount_styles_tests.rs` (300 and 3,000 compiled
rows) and `gpu_mount_frames.rs` (the sampled page) assert computed 4
and live 6.

## Median instructions at 10,000 rows

| phase | before | first | after fixes | before, per extra row | after fixes, per extra row |
|---|---|---|---|---|---|
| mount | 287,825,653 | 283,819,467 | 289,929,066 | 28,968 | 29,184 |
| style | 7,557,755,682 | 3,440,300,740 | 3,442,699,485 | 756,109 | 344,326 |
| layout | 2,437,503,410 | 2,448,002,996 | 2,437,124,198 | 244,764 | 244,805 |
| paint | 292,540,379 | 292,115,399 | 291,854,087 | 29,300 | 29,231 |
| update | 10,287,927,767 | 6,180,813,843 | 6,171,695,684 | 1,030,171 | 618,364 |
| frame (CPU) | 10,315,208,013 | 6,208,066,398 | 6,198,963,027 | 1,030,758 | 618,952 |

Per extra row is the 3000 to 10,000 step. The fix rounds moved style
by +0.07% and the frame by -0.15%. After fixes, style fell by 54% from
before, from 188,934 instructions per restyled element to 86,063; the
frame to first draw fell by 40%. Of the 6,488,869,175 instructions
from mount to the end of the first draw at 10,000 rows, style is now
53%, layout 38%.

What the 86,000 per element that remain are is not measured apart. The
cascade runs for 4 of 40,002 elements; the rest is the share key (the
states, the attribute scan, one hash), the `Used` derivation
(`style/used.rs`, per element, from the 104 computed values), the
refusal check and the kept maps, in proportions this run does not
show. The next step for property 7 is to measure the `Used` derivation
alone, then either share `Used` per (shared style, parent used) where
the context allows, or work on layout's 245,000 per row, which is
within a factor of 1.41 of style.

## Anonymous memory, first mount and first frame

After the first frame, KiB:

| rows | before | first | after fixes | before, per extra row | after fixes, per extra row |
|---|---|---|---|---|---|
| 0 | 13,196 | 13,188 | 13,192 | baseline | baseline |
| 1000 | 112,692 | 63,148 | 63,344 | 99.4 | 50.1 |
| 3000 | 301,252 | 152,516 | 153,088 | 94.2 | 44.8 |
| 10000 | 982,584 | 488,536 | 488,564 | 97.3 | 47.9 |

At 10,000 rows:

| | before | first | after fixes |
|---|---|---|---|
| anon after mount, KiB | 80,364 | 82,624 | 82,652 |
| peak (`VmHWM`, all pages), KiB | 1,041,320 | 547 MB (not kept in KiB) | 547,276 |

The anonymous high-water after the first frame halved: 48 KiB per row
against 97. The mount alone grew by 2.8% (234 B per row; the key's
`Rc` and the toggle mask per element are the expected cause, not
measured apart). Which owner holds the remaining 48 KiB per row (the
per-element `Used` styles, the kept maps, the share keys, the layout
boxes, the text layouts, the paint) is not measured.

## What it decides

- Property 8, style: `ComputedStyle` allocations are constant in rows
  (6 live), held by tests. Other per-element style memory (`Used`, the
  kept maps, the share keys) is not measured, so property 8's style
  part is not closed.
- Property 7: style is still the largest phase, but at 344,000 per
  row it is 53% of mount to first draw, not 71%; the next measurement splits
  the per-element remainder (`Used`, share key, maps) before choosing
  between sharing `Used` and layout.
- Keyword interning (spec section 6) stays deferred: with 6 live
  computed styles the keyword strings are not a per-element cost.
