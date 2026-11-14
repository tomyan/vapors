# Class add frames: paint's two ordered indexes in blocks (2026-11-13, main at adb3b677 with #270's head)

`--scaling Class` and `--scaling HoveredClass` failed on the frames that
add the `marked` class once #270 judged each kind of frame apart:
`paint` grew 13.6 instructions per extra row on the add frames and was
flat on the remove frames (`docs/perf/splice-layout-2026-11-13.md`).
This note locates the step and records the fix.

Method: `vapors-perf 60 10 --scaling <change>`, release, Intel UHD 630
(ANV), 640 x 400, scale 1, median instructions per frame, one run each,
with a temporary `update phase` span around each step of
`paint_partial` (not committed; the names below are the spans').
Machine load was 40 to 60 during every run here (other agents' builds),
so wall times are noisy; the instruction counts repeat within 0.1%.

## Located: two `Vec` inserts

Adding the class gives the middle row's box a background quad, so its
box step starts drawing. Two lists ordered by place or top then had it
put in: `DrawnSteps::drawn` (`drawn_steps.rs`, 24-byte entries) and the
page frame's `entries` in the extents index (`frame_extents.rs`,
48-byte entries). `Vec::insert` moves every entry after it: the half
of the page below the middle row.

| step, Class | kind | 1000 rows | 3000 rows | 10,000 rows | per extra row, 3000 to 10,000 |
|---|---|---|---|---|---|
| p09.drawn (`DrawnSteps::patch`) | add | 10,799 | 20,107 | 51,915 | 4.5 |
| p09.drawn | remove | 6,308 | 6,616 | 6,924 | 0.0 |
| p10.extents (`Extents::patch`) | add | 16,962 | 35,178 | 98,362 | 9.0 |
| p10.extents | remove | 7,799 | 8,001 | 8,209 | 0.0 |
| paint | add | 132,667 | 160,179 | 255,347 | 13.6 |
| paint | remove | 118,689 | 119,332 | 119,839 | 0.1 |

Every other step of paint (order, frames, the kept parts read, drop,
shift, maps, repaint, thumbs, hits, batches) was flat on both kinds.
9.0 / 4.5 is the entry sizes' ratio, 48 / 24.

### The counter saw one side

The remove frames take the same entry out, and `Vec::remove` moves the
same tail the other way. glibc's `memmove` copies a forward,
non-overlapping-destination move of this size with `rep movsb`, which
retires as one instruction, and a backward overlapping move (an insert)
with a vector loop, about one instruction per 5 bytes. The instruction
counter therefore saw the insert and not the remove. Wall time saw
both, before the fix (medians, µs, 1000 / 3000 / 10,000 rows):

| step | add | remove |
|---|---|---|
| p09.drawn | 4.6 / 14.1 / 40.8 | 5.7 / 15.5 / 45.6 |
| p10.extents | 8.2 / 27.3 / 88.9 | 8.2 / 32.3 / 88.9 |
| paint | 52 / 86 / 199 | 53 / 94 / 197 |

So the one-median judge before #270 and the per-kind judge after it
were both blind to the remove frames' tail move; `--scaling` judges
instructions, and an operation that `rep movsb` performs costs it
nothing. A phase held flat by the judge can still move memory in
proportion to the page. Wall time is the check for that, and the timed
run's medians are over both kinds (`docs/perf/timed-2026-11-12.md`).

## Fix: both lists in blocks

`crates/elements/src/blocks.rs`: an ordered list in blocks of at most
128 entries; a put in or take out moves at most a block (a full block
splits first), searches are binary over the blocks' last entries then
within. `DrawnSteps::drawn` and each frame's `entries` are `Blocks`.
`UpdateCounts::index_entries_moved` counts the entries both indexes'
patches moved; `tests/kept_index_moves.rs` holds it under two blocks
when the class is added and removed on 1,000, 3,000 and 10,000 rows
(it moved 3,997 at 1,000 rows with one block, proving the test).

After, same method (instructions; HoveredClass in brackets):

| step | kind | 1000 rows | 3000 rows | 10,000 rows | per extra row |
|---|---|---|---|---|---|
| p09.drawn | add | 6,669 | 6,821 | 7,305 | 0.1 |
| p10.extents | add | 9,611 (9,627) | 9,611 (9,622) | 10,005 (9,977) | 0.1 |
| paint | add | 120,880 (127,669) | 120,738 (128,147) | 122,493 (129,975) | 0.3 |
| paint | remove | 120,564 (127,510) | 120,333 (127,844) | 121,992 (129,326) | 0.2 |
| update | add | 811,316 (818,372) | 811,083 (820,739) | 815,048 (824,219) | 0.6 |

Wall time after (medians, µs): p09.drawn 4.6 / 5.1 / 7.2 add, 4.4 /
5.2 / 7.0 remove; p10.extents 7.4 / 7.6 / 8.5 add, 6.6 / 7.1 / 8.1
remove; paint 87 / 89 / 91 add, 86 / 88 / 94 remove. The 1,000-row paint
is slower than before's 52 µs because this run had the higher load
(the `update` p90s were 6 to 32 ms against 0.3 ms before); the growth
from 1,000 to 10,000 rows, 4 times before in both steps and both kinds,
is gone. A block's move is under 6 KB.

Both `--scaling Class` and `--scaling HoveredClass` exit 0 after;
before, each named 9 growing phases (the two steps, `paint`, `update`
and `frame (CPU)`, each at two sizes, less one under the floor).
Property 1 of `docs/plans/perf-finish-line.md` holds for Class and
HoveredClass on both kinds of frame.

Remaining per-row memory in a patch: the extents index's `at` and the
drawing steps' `places` are per place, read by index, not moved; the
kept hits (`kept_hits.rs`) splice a `Vec` per patched slot, which the
counter saw flat here (p11.hits 1,188 at every size) because the row's
hit count does not change with its class, and which wall time did not
single out. A step that gains or loses a hit would move that list's
tail the same way; it is not in blocks.
