# Memory per element, 2026-11-14 (finish-line property 8)

`vapors-perf-memory` (#320, #321 and this branch) measures a page's
live heap after its mount and settle under a counting allocator, one
page to a process: a 0-row page, then 1,000, 3,000 and 10,000 of the
Mount workload's rows (4 elements a row). Bytes are requested sizes,
live after the settle, not peak, not malloc's overhead, not GPU memory.
Each owner is the net change while its phase was open
(`element_memory.rs`). Byte counts do not depend on the machine's load,
so one run each; the build was release.

Command: `cargo run --release -p vapors-perf --bin vapors-perf-memory`.
It exits 1 when the bytes per element grow with the page
(`element_memory_flat.rs`): the total's slope from 3,000 to 10,000 rows
more than 25% over its slope from 1,000 to 3,000, or an owner's more
than 50% and 512 bytes per element over.

Each table's bytes per element are the slope from the page before, so
the 0-row page's 5.7 MB (5.5 MB of it the draw: the painter's pipelines
and glyph atlas) is not charged per element.

## Before: main at 7c4fc70

Exit 1: total 17,077 then 28,316 bytes per element; paint 6,590 then
16,789.

| rows | elements | styles computed | total MB | total B/element | mount B/element | style B/element | layout B/element | paint B/element | update_rest B/element | accessibility B/element | draw B/element | rest B/element |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 0 | 2 | 0 | 5.7 | - | - | - | - | - | - | - | - | - |
| 1000 | 4002 | 4 | 55.1 | 12352 | 1843 | 161 | 7140 | 3253 | -73 | 0 | 34 | -7 |
| 3000 | 12002 | 4 | 191.7 | 17077 | 1818 | 90 | 8688 | 6590 | -110 | 0 | 0 | 0 |
| 10000 | 40002 | 4 | 984.6 | 28316 | 2128 | 123 | 9363 | 16789 | -88 | 0 | 0 | 0 |

Located with dhat, live bytes per call site: `Blocks::from_sorted`
(`crates/elements/src/blocks.rs`) kept each block with the capacity of
the whole list after it, n²/256 slots, for the scroll frames' extents
index and the drawing steps (2.9 KB a row at 1,000 rows, 29.9 KB at
10,000, and 1.5 KB to 15.0 KB). The layout boxes' list
(`crates/elements/src/layout/boxes.rs`) was grown by doubling and kept,
about 2.8 KB a box: 40,002 boxes in 65,536 slots.

## With #327 (Blocks::from_sorted) alone

Exit 0. Paint is flat at about 2.1 KB per element; layout still moves
7.1 to 9.4 KB, the doubling slack.

| rows | elements | styles computed | total MB | total B/element | mount B/element | style B/element | layout B/element | paint B/element | update_rest B/element | accessibility B/element | draw B/element | rest B/element |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 0 | 2 | 0 | 5.7 | - | - | - | - | - | - | - | - | - |
| 1000 | 4002 | 4 | 50.7 | 11260 | 1843 | 161 | 7140 | 2162 | -73 | 0 | 33 | -7 |
| 3000 | 12002 | 4 | 151.3 | 12575 | 1818 | 90 | 8688 | 2088 | -110 | 0 | 0 | 0 |
| 10000 | 40002 | 4 | 534.2 | 13673 | 2128 | 123 | 9363 | 2146 | -88 | 0 | 0 | 0 |

## With #327 and the layout boxes cut to length

Exit 0. Total 10,735 then 11,222 bytes per element (+4.5%); layout 6.8
to 6.9 KB, paint 2.1 KB, mount 1.8 to 2.1 KB (the element and signal
arenas' doubling slack, bounded), style under 170 bytes; 4 styles
computed for 40,002 elements, one per distinct style.

| rows | elements | styles computed | total MB | total B/element | mount B/element | style B/element | layout B/element | paint B/element | update_rest B/element | accessibility B/element | draw B/element | rest B/element |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 0 | 2 | 0 | 5.7 | - | - | - | - | - | - | - | - | - |
| 1000 | 4002 | 4 | 49.1 | 10846 | 1843 | 161 | 6726 | 2162 | -73 | 0 | 33 | -7 |
| 3000 | 12002 | 4 | 135.0 | 10735 | 1818 | 90 | 6848 | 2088 | -110 | 0 | 0 | 0 |
| 10000 | 40002 | 4 | 449.2 | 11222 | 2128 | 123 | 6912 | 2146 | -88 | 0 | 0 | 0 |

## The check shown failing

With `Blocks::from_sorted` restored to main's and the layout fix kept,
the run exits 1 naming `total: 15237 then 25865 bytes per element` and
`paint: 6590 then 16789 bytes per element`; restored, it exits 0.

## Not measured here

A page's memory is not freed when the page is dropped: dhat saw about
all of it live at process end (49.6 MB at 1,000 rows, 979 MB at
10,000), its own slice next.
