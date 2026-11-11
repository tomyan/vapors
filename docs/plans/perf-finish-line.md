# Performance finish line

Decided by Tom, 2026-11-11: performance work stays the main line of
vapors work until every property below holds, measured. The comparative
benchmark, the website and outside funding applications wait for it.

Each property is measured by `crates/perf` (`vapors-perf`) in a release
build on the dev machine (Intel UHD 630, ANV), 1-minute load under 1.5,
three runs, medians, and recorded in `docs/perf/`. A property holds when
the harness shows it, not when the code is believed to do it.

## The properties

| # | Property | Target | On main at bf8a080 (3000 rows) | Brought by |
|---|---|---|---|---|
| 1 | An edit's whole update (style, layout, paint) | under 1 ms, and the same within noise at 1,000, 3,000 and 10,000 rows | Text 3.76 ms, Class 2.74 ms; paint about 0.2 ms | IU3 3b (layout positioning kept), 3c-3e (moved boxes shifted) |
| 2 | Draw (CPU side of a frame) | under 1 ms at any row count; boxes outside the view cost nothing | 1.7 to 1.8 ms | IU3 3c-3f (kept batches), IU4 (GPU side kept) |
| 3 | Inserting or removing a row | about the cost of an edit | redoes paint order, chains and clips over the whole tree | splice step (below) |
| 4 | Scrolling a long list | no repaint; kept paint shifted | every moved box painted again | IU3 3c-3e |
| 5 | Pointer over the window | no per-update walk of the tree | HoveredClass 0.85 ms above Class | IU3 3c-3g (kept hit list) |
| 6 | Idle | no update work and no frames drawn | no frames drawn; an update with no change 2.57 ms | IU3 3b, the idle fix |
| 7 | First frame of a large page | the winning declarations for each template element and constant shapes worked out at build time (computed values still depend on viewport, scale and font size); time measured and published | not measured | the build-time analysis |
| 8 | Memory per element | one style record per distinct style, not per element; per-element memory bounded by the per-box layout records | not measured | the build-time analysis |
| 9 | HTML, SVG and 3D in one scene | the racer (3D craft, SVG markings on it, HTML on top) drawn as one scene with CSS reaching scene nodes, and no frame over 16.7 ms across a 60 s run | not built | the shape table, 3D slice 2 (PRs 4, 4b, 5) |

Property 1 at 10,000 rows needs a 10,000-row workload in `vapors-perf`;
it is added with the first PR that claims property 1.

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
8. The shape table and the one-scene proof with 3D slice 2 (property 9).

## After the line

When every property holds: the comparative benchmark. The
js-framework-benchmark operations (create 1,000 rows, update every
10th, swap, select, remove, create 10,000) and 7GUIs, run natively
against GPUI, Blitz, iced, Slint and egui with the same harness method,
published with every result, including where vapors loses. Then the
website and the Codex Open Source Fund application.
