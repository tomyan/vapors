# Performance finish line

Decided by Tom, 2026-11-11: performance work stays the main line of
vapors work until every property below holds, measured. The comparative
benchmark, the website and outside funding applications wait for it.

Each property is measured by `crates/perf` (`vapors-perf`) in a release
build on the dev machine (Intel UHD 630, ANV), 1-minute load under 1.5,
three runs, medians, and recorded in `docs/perf/`. A property holds when
the harness shows it, not when the code is believed to do it.

## The properties

Tom, 2026-11-13: properties 1 to 8 end this iteration; property 9 is the next iteration's target, on the target architecture, judged on a minimal test scene rather than the racer (the racer is a later showcase; `docs/plans/racer-scene.md` keeps its design).

| # | Property | Target | On main at bf8a080 (3000 rows) | Brought by |
|---|---|---|---|---|
| 1 | An edit's whole update (style, layout, paint) | under 1 ms, and the same within noise at 1,000, 3,000 and 10,000 rows | Text 3.76 ms, Class 2.74 ms; paint about 0.2 ms | IU3 3b (layout positioning kept), 3c-3e (moved boxes shifted) |
| 2 | Draw (CPU side of a frame) | under 1 ms at any row count; boxes outside the view cost nothing | 1.7 to 1.8 ms | IU3 3c-3f (kept batches), IU4 (GPU side kept) |
| 3 | Inserting or removing a row | about the cost of an edit | redoes paint order, chains and clips over the whole tree | splice step (below) |
| 4 | Scrolling a long list | no repaint; kept paint shifted | every moved box painted again | IU3 3c-3e |
| 5 | Pointer over the window | no per-update walk of the tree | after 3c-3g, HoveredClass update 0.115 M instructions above Class at 3000 rows (0.043 M at 1000): `hit()`'s scan of the kept hits, about 13 µs | IU3 3c-3g (kept hit list) |
| 6 | Idle | no update work and no frames drawn | no frames drawn; an update with no change 2.57 ms | IU3 3b, the idle fix |
| 7 | First frame of a large page | the winning declarations for each template element worked out at build time (computed values still depend on viewport, scale and font size), and constant shapes once the shape table (item 8) lands; time measured and published | not measured | the build-time analysis; shapes by the shape table (Tom, 2026-11-12) |
| 8 | Memory per element | one style record per distinct style, not per element; per-element memory bounded by the per-box layout records | not measured | the build-time analysis |
| 9 | HTML, SVG and 3D in one scene | a minimal test scene (a few 3D objects, an SVG decal on one, HTML on top) drawn as one scene with CSS reaching scene nodes, and no frame over 16.7 ms across a 60 s run | not built | the shape table, 3D slice 2 (PRs 4, 4b, 5) |

Property 1 at 10,000 rows has a 10,000-row workload in `vapors-perf`.

Property 9's run judges every frame from the first, with no warmup
frames excluded, unless Tom decides otherwise (`docs/plans/one-scene.md`,
D7): `vapors-perf`'s default 20 warmup frames would leave out the
mount, pipeline creation and first uploads.

Properties 1 and 6 are judged by `vapors-perf --scaling`, per change:

```
cargo build --release -p vapors-perf
for change in Idle Text FlexText Class HoveredClass; do
  ./target/release/vapors-perf 60 10 --scaling $change || echo "$change grows"
done
```

`cargo xtask scaling` runs every change (`--scaling all`) on a clean
tree; it runs nightly on origin/main on the dev machine
(`scripts/nightly-scaling.sh`), not in the gate (Tom, 2026-11-13).
It counts instructions at 1000, 3000 and 10,000 rows and exits non-zero,
naming the phase and its growth per extra row, when an edit's phase
(Text, FlexText, Class, HoveredClass; every phase) or an idle update's total
(Idle; `update`) is both over 2% and over 5000 instructions above its 1000-row count at a larger size.
At 1657b27 every one of the four failed
(docs/perf/baseline-2026-11-10.md, "Scaling, 1657b27"); after M10b
(2026-11-12) Text and Idle pass, and Class and HoveredClass failed only
on `style`, 2 instructions per extra row; they pass since the style
phase's kept sets stopped keeping the mount's capacity (2026-11-12;
HoveredClass `style` 843k / 847k / 870k instructions at 1,000 / 3,000 /
10,000 rows before, 840k / 848k / 850k after, docs/plans/iu1.md, PR 2).
Since #270 (2026-11-13) `--scaling` judges the frames that add the class
apart from those that remove it, and Class and HoveredClass failed on the
add frames: `paint` 13.6 instructions per extra row
(docs/perf/splice-layout-2026-11-13.md); the one median had been the
remove frames'. The cost was two `Vec` inserts, the drawing steps and
the extents index taking in the row's step as it started to draw; both
lists are in blocks since 2026-11-13 and both changes pass on both
kinds of frame (docs/perf/class-add-paint-2026-11-13.md). The remove
frames moved the same bytes with `rep movsb`, one retired instruction:
an instruction judge misses a memmove in that direction, and wall time
is the check for it.
The 5k to 9k instructions HoveredClass `style` still rises from 1,000
rows (once 1 under the judge's 5000 floor, passing on 2%) are glibc's
malloc costing more per call on a larger heap: with jemalloc preloaded
`style` is flat at 733k, and allocation is about 170k of the cascade's
430k (follow-up: fewer allocations in `Cascade::compute`, or an
allocator decision; iu1.md, PR 2).
FlexText, an edit in a flex-column list whose rows have a vertical
margin, passes since M10c (2026-11-12; `layout` 5.6 M / 16.6 M / 55.3 M
instructions at 1,000 / 3,000 / 10,000 rows before, 123 k / 126 k /
127 k after, docs/plans/iu1.md).
Property 1's
time target (under 1 ms) is read from the timed run; at 9b405f9
(2026-11-12, docs/perf/timed-2026-11-12.md) it holds for Text, Class,
HoveredClass and FlexText at 1,000, 3,000 and 10,000 rows. `--scaling Insert` reports for property 3
and does not judge until the splice step is built.

Text shaping (T1, vapors #253 and #254,
`docs/perf/text-shaped-2026-11-12.md`): a run is shaped once per text,
styles, visibility, span ranges and scale, whichever node holds it, and
each width is a break of that shaping. Mount's layout went from 249,815
to 205,087 instructions per row (3 shapings per row to 2). Remaining:
Taffy's measure function still breaks a run at min- and max-content as
well as at its final width. The widths alone can be read from the
shaped run (`Shaped::content_widths`), but the measure function returns
a height too, and Taffy's caches are not keyed by the requested axis.
Follow-up: pass the axis through the measure path so an intrinsic
width costs no break; measure what that removes from an edit's layout.

## Order of work

1. IU3 3c-3f: kept batches (property 2).
2. IU3 3c-3g: kept hit list (property 5); the idle update rise from
   #201 (+0.4 ms).
3. IU3 3c-3e: moved boxes shift their kept paint (property 4).
4. IU3 3b: layout positioning kept, moved boxes reported (properties 1
   and 6).
5. Splice step (property 3), runtime first (Tom, 2026-11-11;
   `docs/plans/splice.md`): (A) an insert or remove builds only the
   row's boxes and keeps its siblings'; (B) paint decides at runtime,
   from what `ChangedSlots` compares and each new box's `Stacking`,
   whether to splice the row's steps, clips and chains into the kept
   order, debug builds checking every splice against the whole walk;
   (C) the insert workload measures it. Rows after the insert still
   move, O(rows after) once 3c-3e shifts them: layout writes every
   row's location when the list is laid out again, and IU4 bounds only
   the GPU side of that to the visible rows (`docs/plans/iu4.md`,
   section 3); a layout-lane design would move the floor. The
   compiler's mark of list templates moves to item 7.
6. IU4 (`docs/plans/iu4.md`): GPU-side paint kept between frames,
   parts in scroll-frame coordinates and a per-scroller visible range
   (property 2, and the scroll remainder of property 4: after 3c-3e a
   scroll still shifts every moved part, O(rows in the scroller)).
7. The build-time analysis, first version (properties 7 and 8),
   with the compiler's mark of list templates whose rows cannot
   change paint order (`docs/plans/splice.md` section (d)).
8. The shape table and the one-scene proof with 3D slice 2 (property 9;
   `docs/plans/one-scene.md`).

## After the line

When every property holds: the comparative benchmark. The
js-framework-benchmark operations (create 1,000 rows, update every
10th, swap, select, remove, create 10,000) and 7GUIs, run natively
against GPUI, Blitz, iced, Slint and egui with the same harness method,
published with every result, including where vapors loses. Then the
website and the Codex Open Source Fund application.
