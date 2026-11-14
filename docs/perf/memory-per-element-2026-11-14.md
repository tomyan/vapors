# Memory per element, 2026-11-14 (finish-line property 8)

`vapors-perf-memory` (#320, #321, #333) measures a page's live heap
after its mount and settle under a counting allocator, one page to a
process: a 0-row page, then 1,024, 4,096 and 16,384 of the Mount
workload's rows (4 elements a row). The sizes go by fours, so every
list grown by doubling is as full at each size and doubling slack does
not move the slope (the review of #333, M1).

Bytes are requested sizes (`Layout::size`), live after the settle: not
peak, not malloc's overhead, not GPU memory. Natively, glibc backs a
large list by its own mapping, and capacity never written is not
resident, so a cut in requested bytes overstates the resident set's
gain on Linux; on the web (wasm) the capacity is linear memory and the
numbers are what the page holds. Each owner is the net change while its
phase was open (`element_memory.rs`). Byte counts do not depend on the
machine's load; one run each, release build.

Command: `cargo run --release -p vapors-perf --bin vapors-perf-memory`.
It exits 1 when (`element_memory_flat.rs`): the total's slope grows
over 2% from the first pair of sizes to the second; an owner's over 5%
and 64 bytes per element; the styles computed differ between sizes
(one style per distinct style, not per element); or the total per
element is over twice layout's.

Each table's bytes per element are the slope from the page before, so
the 0-row page's 5.7 MB (5.5 MB of it the draw: the painter's pipelines
and glyph atlas) is not charged per element.

## Before: main at 7c4fc70

Exit 1: total 17,409 then 34,736 bytes per element; paint 7,882 then
25,162; the total over twice layout's.

| rows | elements | styles computed | total MB | total B/element | mount B/element | style B/element | layout B/element | paint B/element | update_rest B/element | accessibility B/element | draw B/element | rest B/element |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 0 | 2 | 0 | 5.7 | - | - | - | - | - | - | - | - | - |
| 1024 | 4098 | 4 | 58.2 | 12816 | 2314 | 143 | 7138 | 3274 | -79 | 0 | 33 | -7 |
| 4096 | 16386 | 4 | 272.1 | 17409 | 2312 | 132 | 7162 | 7882 | -80 | 0 | 0 | 0 |
| 16384 | 65538 | 4 | 1979.5 | 34736 | 2314 | 132 | 7207 | 25162 | -80 | 0 | 0 | 0 |

Located with dhat, live bytes per call site: `Blocks::from_sorted`
(`crates/elements/src/blocks.rs`) kept each block with the capacity of
the whole list after it, n²/256 slots, for the scroll frames' extents
index and the drawing steps. The layout boxes' list was grown by
doubling and kept, about 2.8 KB a box.

## With #327 (Blocks) alone

Exit 0. Paint 2,158 bytes per element at every size.

| rows | elements | styles computed | total MB | total B/element | mount B/element | style B/element | layout B/element | paint B/element | update_rest B/element | accessibility B/element | draw B/element | rest B/element |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 0 | 2 | 0 | 5.7 | - | - | - | - | - | - | - | - | - |
| 1024 | 4098 | 4 | 53.6 | 11700 | 2314 | 143 | 7138 | 2158 | -79 | 0 | 33 | -7 |
| 4096 | 16386 | 4 | 197.2 | 11685 | 2312 | 132 | 7162 | 2158 | -80 | 0 | 0 | 0 |
| 16384 | 65538 | 4 | 773.9 | 11732 | 2314 | 132 | 7207 | 2158 | -80 | 0 | 0 | 0 |

## With #327 and the layout boxes cut to length once placed

Exit 0. Total 11,337 then 11,384 bytes per element (+0.4%), layout
6,814 then 6,859, the total 1.66 times layout's; 4 styles computed at
every size. At 16,384 rows the page holds 751.1 MB, against 1,979.5 MB
on main.

| rows | elements | styles computed | total MB | total B/element | mount B/element | style B/element | layout B/element | paint B/element | update_rest B/element | accessibility B/element | draw B/element | rest B/element |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 0 | 2 | 0 | 5.7 | - | - | - | - | - | - | - | - | - |
| 1024 | 4098 | 4 | 52.2 | 11352 | 2314 | 143 | 6791 | 2158 | -79 | 0 | 33 | -7 |
| 4096 | 16386 | 4 | 191.5 | 11337 | 2312 | 132 | 6814 | 2158 | -80 | 0 | 0 | 0 |
| 16384 | 65538 | 4 | 751.1 | 11384 | 2314 | 132 | 6859 | 2158 | -80 | 0 | 0 | 0 |

## The check shown failing

Main's state is the first table: the run exits 1 naming the total's
growth, paint's and the total over twice layout's. The unit tests in
`element_memory_flat_tests.rs` name a style computed per element and a
total over twice layout's, each failing with its check removed.

## One row added at 16,384 rows

`vapors-perf-memory --add-row --rows 16384` settles the page, inserts
one row in the middle, and settles again. The kept box list grows by a
quarter when it is full (`slots_subtree.rs`, lead's decision
2026-11-14), not by doubling:

| growth | bytes the insert leaves | the insert's frames |
|---|---|---|
| doubling (before) | 229,520,455 | 94 to 104 ms |
| by a quarter | 109,783,399 | 97 to 121 ms |

Three runs each, wall time under a 1-minute load of about 31, with the
counting allocator. Of the 109.8 MB, the layout phase left 52.9 MB (the
box list's quarter and the slots beside it) and paint 56.4 MB (another
kept list grown by doubling; not located yet). The three inserts after
it each left about 7 KB and took 99 to 147 ms, so the growing insert's
copy of every box (about 160 MB) does not stand out from an insert's
own time at this size. If it shows in insert's instructions once the
splice step makes inserts cheap, chunked storage for the box list (as
`blocks.rs`, #288) is the follow-up.

## Not measured here

- A page's memory when it is dropped: dhat saw about all of it live at
  process end (the drop-leak slice).
