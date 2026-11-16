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

## Every per-slot list grown by a quarter (slot_list.rs)

The 56.4 MB paint left on the first insert was the kept boxes' list
(`painted.kept_boxes`, one `Option<KeptBox>` of 424 bytes a slot:
48.6 MB for 114,690 slots, a slot for each of the 65,538 elements and
49,152 text runs, about 740 bytes per element), sized exactly by the first paint and doubled
by an insert that adds four boxes. The placement's six lists, the clip
lists, the clip tree's two, the stackings and the style slots did the
same. Each now grows by a quarter, as the box list does:

| one row added at 16,384 rows | bytes the insert leaves |
|---|---|
| main at 06b0c94 | 110,180,323 |
| every per-slot list by a quarter | 58,483,505 |

The page itself is unchanged: 12,887 bytes per element on main and
12,863 with this change (the slope from 4,096 to 16,384 rows; main's
layout per element rose to 8,193 bytes since #333's 6,859, from the
work merged in between).

What the 58.5 MB is, from each list's capacity in bytes before and
after the insert (a print added for the run, not kept):

| list | grown by |
|---|---|
| layout's box list (159.6 MB → 199.6 MB) | 39.9 MB |
| the kept boxes, `painted.kept_boxes` (48.6 MB → 60.8 MB) | 12.2 MB |
| the other 13 lists (placement, clips, clip tree, stackings, layout's keys) | 5.0 MB |
| total of the lists | 57.1 MB |

The live heap grew 55.9 MB in that run, less than the lists' 57.1 MB:
their growth is all of it, and the insert freed about 1.2 MB elsewhere
(not located). The kept hits and style slots did not grow: an insert
gathers the hits again, exactly, and the style slots had slack left.
A list never shrinks, and a quarter of 65,538 slots is room for about
4,096 more rows of four boxes before any of them grows again.

Not measured: the time of many single-row adds, quarter growth against
doubling. A list of n slots grown by a quarter grows again after n/4
more slots, where doubled it grows after n, so over many adds it
copies about four times as many entries (n/(1.25 - 1) against
n/(2 - 1)); at 16,384 rows each growth of the box list
copies about 160 MB. Not measured either: that copy's cost on wasm.
One-run wall times are left out of the table: they were taken at
different loads and do not compare.

At about 740 bytes per element, the kept boxes' list is the next per-element
target after layout's boxes (taken below).

## A kept box's outline, thumbs and rarer entries boxed apart

A kept box's slot (`Option<KeptBox>`) was 424 bytes: what it was made
from (64), its own part (216: a scene and six lists) and two more
scenes, outline and thumbs (72 each). Most boxes have no outline or
thumbs, and no inline element's rect, range track or text origin. Those
five now sit in two boxes made only when one of them has entries
(`kept_box.rs` `Marks`, `kept_part.rs` `Rare`). Cursors stay in the
slot: every element paints its cursor, `auto` included. A slot is now
224 bytes (`kept_box_size_tests.rs`).

Main at 854ccb8 against this change, the first two rows from `cargo run
--release -p vapors-perf --bin vapors-perf-memory -- --rows 16384`, the
third from `... -- --add-row --rows 16384`:

| | main | boxed apart |
|---|---|---|
| total bytes per element | 12,941 | 12,591 |
| paint bytes per element | 2,158 | 1,808 |
| one row added at 16,384 rows, bytes left | 58,483,505 | 52,749,105 |

Paint fell by 22,938,000 bytes at 16,384 rows: 114,690 slots (elements
and text runs) of 200 bytes less each, which is 350 bytes per element.

Paint's instructions on the rows page (`cargo run --release -p
vapors-perf -- 60 10 --scaling <change>`, median per frame, the same to
within 0.1% between runs):

| change | paint at 10,000 rows: main | boxed apart | per extra row, 3000 to 10,000: main | boxed apart |
|---|---|---|---|---|
| Insert, insert frames | 123,437,530 | 120,896,863 | 12,512.7 | 12,258.6 |
| Insert, remove frames | 123,428,526 | 120,888,470 | 12,510.5 | 12,256.5 |
| Class, add frames | 58,013 | 56,047 | 0.1 | 0.1 |
| Class, remove frames | 57,423 | 55,562 | 0.1 | 0.1 |
| Scroll | 13,386 | 13,312 | 0.0 | 0.0 |

The first version cost Insert about 150 instructions more per row: a
shifted box's checks and move walked its own scene and outline as one
chained iterator, and the outline now came through an `Option`. Two
plain loops, the outline's skipped when there is none (`kept_shift.rs`),
make Insert's paint about 260 instructions per row cheaper than main.
Scroll's paint then took 52 instructions a frame more than main's.
Spans around each step of `paint_partial` (a run not kept) put every
step at or under main's, so the 52 were in how the steps compiled
together: with the outline and thumbs accessors and `scene_of` always
inlined, Scroll's paint is 74 instructions a frame under main's.

A scroll container keeps its marks box once its thumbs are painted,
while they are hidden and shown again, so a touch that toggles them
allocates and frees no box per container; a repaint of its box makes
it afresh (`kept_box_marks_tests.rs`).

## A dropped page (#353)

Live bytes after a settled 10,000-row page is dropped, three times in
one process, with #353 (a top-level component disposes its owner when
dropped) and a measuring mode kept out of the tree:

| | held | left after the 1st drop | 2nd | 3rd |
|---|---|---|---|---|
| with #353 | 449.2 MB | 18.1 MB | 0.7 MB | 0.7 MB |
| without | 449.2 MB | 443.7 MB | 443.6 MB | 460.4 MB |

The 18.1 MB after the first drop is the signal runtime's arenas keeping
their slots, which the next page takes.
