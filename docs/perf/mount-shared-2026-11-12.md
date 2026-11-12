# Mount and first frame with shared styles, 2026-11-12

Analysis PR 2 (finish-line properties 7 and 8), after PRs 2a to 2d
(vapors #235, #237, #238, #239): the compiler gives each template
element a `StyleKey`, and the tree keeps one computed style per share
key (key, parent style, states, toggled classes, selected attributes).
The workload, run and hardware are those of `mount-2026-11-12.md`
("before" below is that file's run at adc1973); this run is at the head
of PR 2e (`analysis/style-keys-2e`, on 813bb4d plus the report's styles
line), release build, load average 14.8 at the start. Instruction
counts do not move with load; no times are kept.

`cargo run --release -p vapors-perf -- 9 --mount`.

## Where the styles came from, first update

| rows | restyled | shared | computed | unkeyed | unshareable | live |
|---|---|---|---|---|---|---|
| 1000 | 4002 | 3996 | 4 | 2 | 0 | 6 |
| 3000 | 12002 | 11996 | 4 | 2 | 0 | 6 |
| 10000 | 40002 | 39996 | 4 | 2 | 0 | 6 |

`restyled` is as before (the walk visits every element). The four
computed styles are the row's four keys (`div.row`, `span.label`,
`span.value`, `button`) under one parent; `html` and `body` are
unkeyed. `live` is flat in rows: the acceptance for property 8's style
part.

## Median instructions, before and after

| phase | before, 10000 rows | after, 10000 rows | before, per extra row | after, per extra row |
|---|---|---|---|---|
| mount | 287,825,653 | 283,819,467 | 28,968 | 28,550 |
| style | 7,557,755,682 | 3,440,300,740 | 756,109 | 344,092 |
| layout | 2,437,503,410 | 2,448,002,996 | 244,764 | 246,180 |
| paint | 292,540,379 | 292,115,399 | 29,300 | 29,257 |
| update | 10,287,927,767 | 6,180,813,843 | 1,030,171 | 619,563 |
| frame (CPU) | 10,315,208,013 | 6,208,066,398 | 1,030,758 | 620,149 |

Per extra row is the 3000 to 10,000 step; the 1000 to 3000 step gives
343,184 for style after (754,583 before). Style fell by 54% per row,
from 189,000 instructions per restyled element to 86,000; the frame to
first draw fell by 40%. Of the 6,491,866,610 instructions from mount to
the end of the first draw at 10,000 rows, style is now 53%, layout 38%.

What the 86,000 per element that remain are: not the cascade (3,996
of 4,002 elements take the table's style), so the share key (the
states, the attribute scan, one hash), the `Used` derivation
(`style/used.rs`, per element, from the 104 computed values), the
refusal check and the kept maps. Not measured apart here; the next
step for property 7 is to measure the `Used` derivation alone and
share `Used` per (shared style, parent used) where the context allows,
or to move layout's 246,000 per row, which is now within a factor of
1.4 of style.

## Anonymous memory, first mount and first frame

| rows | before, anon after first frame KiB | after, anon after first frame KiB | before, KiB per extra row | after, KiB per extra row |
|---|---|---|---|---|
| 0 | 13,196 | 13,188 | baseline | baseline |
| 1000 | 112,692 | 63,148 | 99.4 | 49.9 |
| 3000 | 301,252 | 152,516 | 94.2 | 44.6 |
| 10000 | 982,584 | 488,536 | 97.3 | 48.0 |

The heap high-water after the first frame halved: 48 KiB per row
against 97. The mount alone is unchanged (82,624 KiB after mount at
10,000 rows against 80,364: the key's `Rc` and the toggle mask per
element). What remains per row is the first frame's: the `Used`
styles, the layout boxes, the text layouts and the paint, as
`mount-2026-11-12.md` lists them. Peak (`VmHWM`) at 10,000 rows is
547 MB against 1,041 MB.

## What it decides

- Property 8, style: done for compiled templates. Style memory is
  constant in rows (6 live styles); the per-row memory is layout's and
  text's.
- Property 7: style is still the largest phase, but at 344,000 per
  row it is no longer 71% of the frame; the next measurement splits
  the per-element remainder (`Used`, share key, maps) before choosing
  between sharing `Used` and layout.
- Keyword interning (spec section 6) stays deferred: with 6 live
  styles the keyword strings are not a per-element cost.
