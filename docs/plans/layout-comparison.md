# Layout comparison spike

Measured 2026-11-19 on `origin/main` at f01246ba plus this slice. No
production change: the harness is `crates/perf/layout-spike/`, a
standalone crate with its own `[workspace]` and `Cargo.lock` (the root
`Cargo.toml` and `Cargo.lock` are untouched; nothing depends on it).

## The options

- (a) lanes: `vapors-elements`' `Document`, CSS in, lanes for flex and
  block containers, Taffy beneath (its own box tree, not `TaffyTree`).
- (b) Taffy 0.15.1 alone: `TaffyTree`, a node per box, fixed sizes.
- (c) compile-time boundary: a hand-written prototype of what the
  compiler would emit for a table whose columns and row height are
  constants (`src/fixed.rs`). Offsets are consts, the last column is one
  subtraction of the viewport width. A leaf size change inside it is not
  a relayout: it is refused when the component compiles.
- (d) generated per component: a hand-written prototype of
  per-component code (`src/generated.rs`): the structure is in the code,
  sizes are runtime values, a change marks a row (table) or the levels
  from the changed one down (deep tree).
- cephal: not measured. It is not available offline.

## Workloads (`src/spec.rs` has them exactly)

- Table(n), n = 30, 300, 3000: a flex column of n rows, 1000 wide; a row
  is a flex row 24 tall of four fixed cells (80, 120, 100, 140) and a
  fifth taking the rest. Change: cell 2 of the middle row, 120 to 150
  wide and back.
- Deep(100): 100 nested flex containers, alternating column and row,
  2 px padding, each holding a 10 px leaf then the next level
  (flex 1 1 0). Change: the leaf of level 98, 10 to 14 px square and back.

Leaves have fixed boxes and no text. Every option is held to the same
boxes, before and after the change, on every workload that is timed
(Table(30), Table(300), Table(3000), Deep(100)) and on Table(301) and
Deep(3). The expected boxes come from `spec.rs`'s words
(`src/agree.rs`; Deep's worked down the levels in f64, independent of
(d)'s f32 code), and Deep(3)'s are also written out by hand in
`tests/agree.rs`. The check runs in a plain `cargo test` in the spike
crate (all four options are default features; the test refuses to
compile without them), and `bench` runs it on every timed workload
before it times anything and exits 1 on a difference.

## Time to lay out

Machine: Intel Core i5-10500T, 2.3 GHz, 6 cores / 12 threads, Linux;
rustc 1.99.1; the `timing` profile (release, `opt-level` 3, one codegen
unit, LTO). Microseconds, median of 9 first layouts (each on a freshly
built tree, the build not timed) and of 41 relayouts (the change and the
layout both timed). Three runs of the whole bench; the table is run 2,
and where the runs differ the range is given (run 1, right after the
build, was the noisy one: Table(30) taffy 221, lanes whole update 3 441).

| workload | option | first layout | relayout after one leaf |
|---|---|---|---|
| Table(30) | taffy | 102 (102-221) | 20 (20-32) |
| Table(30) | lanes, whole update | 1 950-3 440 | 34-52 |
| Table(30) | lanes, layout phase only | 224 (224-502) | 12-20 |
| Table(30) | boundary (c) | 0.1 | n/a |
| Table(30) | generated (d) | 0.2 | < 0.05 |
| Table(300) | taffy | 1 144 (1 143-2 635) | 181 (180-274) |
| Table(300) | lanes, whole update | 19 700-27 000 | 38-50 |
| Table(300) | lanes, layout phase only | 2 389-2 464 | 13-14 |
| Table(300) | boundary (c) | 1.0 | n/a |
| Table(300) | generated (d) | 2.3 | 0.2 |
| Table(3000) | taffy | 13 100-13 800 | 2 280-2 830 |
| Table(3000) | lanes, whole update | 280 000-285 000 | 47-51 |
| Table(3000) | lanes, layout phase only | 28 100-32 400 | 20-22 |
| Table(3000) | boundary (c) | 13.4 | n/a |
| Table(3000) | generated (d) | 23.3 | 1.7 |
| Deep(100) | taffy | 360-417 | 332-357 |
| Deep(100) | lanes, whole update | 2 749-2 761 | 615-626 |
| Deep(100) | lanes, layout phase only | 697-747 | 578-582 |
| Deep(100) | boundary (c) | n/a | n/a |
| Deep(100) | generated (d) | 0.8 | < 0.05 |

Reading it:

- "lanes, whole update" is `Document::update`: style, layout and paint
  of every node. "Layout phase only" is the `update phase` span named
  `layout`, summed by a subscriber in `src/clock.rs`. The clock returns
  an error when it saw no such span (the bench prints it and exits 1,
  naming `src/clock.rs`), so a 0.0 cannot appear; `tests/clock.rs` shows
  it reads more than zero after a real lanes layout. Compare (a) to
  the others by the second row; the first shows layout is about a tenth
  of a first update at 3000 rows (28 of 285 ms).
- First layout: Taffy alone beats the lanes layout phase about 2x
  at 300 and 3000 rows, but the phase also builds the box tree from the
  styled DOM, which `TaffyTree` is handed ready made here. Not split.
- Relayout is where lanes earn their place: 3000 rows relayout in about
  20 us against 2.3-2.8 ms for Taffy alone, whose root visits every row
  to find the cached ones. Taffy's relayout grows with rows; lanes' does
  not. On the deep tree both walk the 98 ancestors: Taffy 332-357 us,
  lanes 578-582 us.
- (c) and (d) are upper bounds on what generation buys: they write
  boxes and nothing else (no style, no text, no paint, no dirty list;
  (d)'s 3000-row relayout scans 3000 flags, a generator would keep a
  list). They are two to four orders below the others, because the
  work is arithmetic on known numbers. (c) cannot take Deep(100) (the
  boxes follow flex-grow down the levels) and refuses it loudly; it
  cannot take the leaf change at all.
- The first attempt's numbers held within noise once the clock was
  checked: the clock did see layout spans in the bench. It would not
  have in a test run in parallel with other layouts (below).

A tracing detail the clock has to work around: tracing caches per
callsite whether any subscriber wants it, and a callsite first hit with
no subscriber (an untimed layout, or another thread's) could stay
disabled for the clock. `layout_phase` rebuilds that cache before the
work, and its test lives in its own test binary because another test's
threads laying out at the same moment still lost the race (reproduced
six runs in eight with the test in `agree.rs`'s binary).

## Wasm size

`crates/perf/layout-spike/size.sh` (each build `--no-default-features`
plus the option): a `wasm32-unknown-unknown` binary that lays a 30-row
table out by the options compiled in, built release (`opt-level` "z",
LTO, one codegen unit, `panic = "abort"`, stripped). No `wasm-opt` or
brotli here, so raw and `gzip -9` bytes. Not re-measured in this
revision: the library's option code did not change, only the default
features and the clock (compiled into the lanes build, untouched by the
size program), so these are the first attempt's figures.

| features | wasm bytes | gzip -9 bytes | delta over none (raw) |
|---|---|---|---|
| none (std, black_box) | 15 241 | 6 200 | 0 |
| taffy (b) | 121 647 | 51 574 | +106 406 |
| lanes (a) | 3 241 764 | 1 143 816 | +3 226 523 |
| fixed (c) | 30 465 | 12 999 | +15 224 |
| generated (d) | 30 705 | 13 361 | +15 464 |
| taffy + fixed + generated | 127 780 | 53 967 | +112 539 |

- (a) is the whole element stack: DOM, CSS, style, text shaping, paint
  and the two bundled fonts (about 340 KB raw), so the delta is not
  layout's. `attribute.sh` attributes function bytes of an unstripped
  build by name: Taffy 72 KB, `vapors_elements` functions with `layout`
  in the name 199 KB (of which `lane` 54 KB), of 585 KB for
  `vapors_elements` and 1 930 KB of function bytes in all. So layout
  is roughly 270 KB of 3.2 MB, by name (inlining blurs it). The
  production web build has its own features and `wasm-opt`; these are
  this harness's configuration.
- (b) uses `TaffyTree` (`taffy_tree` feature); the shipped Taffy is
  used through the elements' own tree, 72 KB. Adding Taffy beside lanes
  would be paid twice in part.
- (c) and (d) cost 15 KB each over the floor, which is the `Vec`
  and allocator they pull in; one table, so the growth per component is
  not measured.

## Reproduce

From the repository root (the first command runs the agreement and clock
tests; no feature flags needed):

```
CARGO_INCREMENTAL=0 cargo test --offline --manifest-path crates/perf/layout-spike/Cargo.toml
CARGO_INCREMENTAL=0 cargo run --offline --manifest-path crates/perf/layout-spike/Cargo.toml --profile timing --bin bench
crates/perf/layout-spike/size.sh
crates/perf/layout-spike/attribute.sh
```

The bench exits 2 in a debug build, naming the command; 1 when an
option's boxes differ or the layout-phase clock saw no layout span.

## Not measured, and why

- cephal: not available offline.
- Text in leaves: shaping and measure functions are a cost of their own
  and the same for every option but (c), which would need fixed-size
  cells or a text boundary. Harness leaves have fixed boxes.
- The compiler's real emission for (c) and (d): these are hand
  prototypes, writing boxes only. Nothing here says how much of the
  prototype's gap survives a `.wisp` component through `Document`, whose
  style and paint (about nine tenths of a 3000-row first update) stay.
- Runtime in a browser's wasm: only sizes were measured.
- brotli and `wasm-opt` sizes; memory per option; Taffy grid; other
  tree shapes (wide flat, tables with row-height changes).
- Size growth of (d) with the number of components.
- Variation between machines: one machine, three runs (the ranges above).

## Recommendation

1. Keep lanes. Standalone Taffy is slower to relayout at 3000 rows by two
   orders and adds its own 106 KB; it is the floor lanes sit on, not an
   alternative.
2. Do not spend a slice on (d) yet: its gain over (c) here comes from
   dirty tracking lanes already do (relayout 20 us at 3000 rows), and
   it multiplies code per component.
3. The one that moves a whole-update number is (c), but layout is only
   about a tenth of a 3000-row first update. Next slice: the same
   table as a real `.wisp` component, timing `Document`'s style, layout
   and paint phases (vapors-perf's `PhaseClock`) on it, to decide
   whether a compiler boundary that also skips style and paint for a
   static subtree is worth building; start with the compiler refusing,
   loudly, a `static` table whose cell sizes are not constants.
