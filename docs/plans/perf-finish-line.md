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
| 7 | First frame of a large page | parts the compiler proves constant have styles and shapes worked out at build time; time measured and published | not measured | the build-time analysis |
| 8 | Memory per element | parts the compiler proves constant keep no per-element records (marks, kept styles, kept paint) | not measured | the build-time analysis |
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
5. Splice step, brought forward from the build-time analysis (Tom,
   2026-11-11): the compiler marks list templates whose rows cannot
   change paint order (no positioned, z-indexed, scrolling or
   outline-only descendants), and inserting or removing such a row
   splices it into the kept order (property 3).
6. IU4: GPU-side paint kept between frames (property 2).
7. The build-time analysis, first version (properties 7 and 8).
8. The shape table and the one-scene proof with 3D slice 2 (property 9).

## After the line

When every property holds: the comparative benchmark. The
js-framework-benchmark operations (create 1,000 rows, update every
10th, swap, select, remove, create 10,000) and 7GUIs, run natively
against GPUI, Blitz, iced, Slint and egui with the same harness method,
published with every result, including where vapors loses. Then the
website and the Codex Open Source Fund application.
