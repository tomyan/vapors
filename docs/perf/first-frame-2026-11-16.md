# First frame of a large page, 2026-11-16 (main at 160b9663)

Property 7's record (`docs/plans/build-time-styles.md`, M1), taken after
B2, R1a, R1b, B3a and B3b merged (#429, #430, #437), so every keyed
element's style comes from its key's compiled winners.

Command: `cargo xtask quiet 30 -- scripts/first-frame-runs.sh <dir>` on a
clean tree at 160b9663; its outputs are kept verbatim in
`first-frame-2026-11-16/` (each run's stdout, `log.txt`, `sha256.txt`)
and `tables.md` was made from them by `scripts/first-frame-table.sh`.

## Setup

- Hardware: Intel UHD Graphics 630 (CML GT2), Vulkan, Mesa 25.0.7 (ANV),
  headless; the dev machine.
- The Mount page: a compiled `{for}` of rows, each a `div.row` with two
  `span`s and a `button`, 640 x 400, scale 1.
- What one run measures, one page per process (`vapors-perf
  --first-frame <rows>`): the mount; `App::start` (painter, pipelines and
  the first update), timed to the GPU's idle; frame 0, timed to the GPU's
  idle. "Mount to frame 0 idle" is each run's own sum of the three, then
  the median over runs.
- Times on release, the shipped profile: three pairs per size, a cold run
  on a new, empty Mesa shader cache, then a warm run on the cache it
  filled. Instructions on the judged profile (release, one codegen
  unit), three runs per size.
- Load: settled to 1.43 (1-minute) before the first run; every run's
  start and end under 1.5 (`log.txt`; the table script refuses
  otherwise).

## Times (release, median of 3 runs, µs)

| rows | cache | mount to frame 0 idle | mount | App::start | start: style | start: layout | start: paint | frame 0 to submit | frame 0 to idle |
|---|---|---|---|---|---|---|---|---|---|
| 1000 | cold | 207,957 | 7,097 | 196,913 | 6,659 | 57,062 | 9,546 | 2,365 | 3,970 |
| 1000 | warm | 97,943 | 6,956 | 83,956 | 6,511 | 57,656 | 9,365 | 2,369 | 3,976 |
| 3000 | cold | 376,784 | 21,308 | 351,628 | 21,806 | 172,566 | 32,417 | 2,238 | 3,847 |
| 3000 | warm | 267,005 | 21,144 | 242,014 | 22,025 | 175,925 | 31,969 | 2,220 | 3,833 |
| 10000 | cold | 1,027,201 | 70,427 | 957,064 | 81,510 | 616,868 | 124,645 | 2,409 | 3,999 |
| 10000 | warm | 908,504 | 69,753 | 832,759 | 80,723 | 606,026 | 126,668 | 2,359 | 3,930 |

## Instructions (judged, median of 3 runs)

| rows | mount | start: style | start: layout | start: paint | start: write | frame 0 |
|---|---|---|---|---|---|---|
| 1000 | 32,776,701 | 46,607,248 | 222,252,022 | 33,193,700 | 112,631 | 7,399,081 |
| 3000 | 94,722,945 | 134,959,196 | 679,645,889 | 97,426,837 | 112,161 | 7,418,678 |
| 10000 | 318,842,126 | 448,313,645 | 2,291,411,341 | 326,721,669 | 111,892 | 7,421,554 |

## Reading against property 7

- **Time measured and published:** this file. Property 7 has no time
  target.
- **Winning declarations worked out at build time:** every keyed element
  of the Mount page takes its style from compiled winners
  (`winners_runtime` 0, pinned by `crates/perf` mount tests). This did
  not make the first frame cheaper. Against the indicative figures taken
  while #428 was reviewed (revision 555839d3f849, a work-in-progress
  commit on #428's branch, before B2; load 6 to 8, so their times are not
  comparable), the judged instructions at 10,000
  rows moved: start style 429.9 M to 448.3 M (+4.3%), layout 2,261.0 M to
  2,291.4 M (+1.3%), paint 312.5 M to 326.7 M (+4.5%), mount 307.5 M to
  318.8 M (+3.7%). The revisions differ by more than B2 to B3b (`git log
  555839d3f849..160b9663` lists other merged work, lane layout among
  it), so this file does not attribute the change; the compiled path still computes each element's
  values, and verifies one element per key per state against the full
  cascade.
- **Where the first frame goes:** `App::start` is 92% of the wait at
  10,000 rows warm (833 of 909 ms), and layout is 73% of that (606 ms);
  style is 81 ms. A cold shader cache adds about 120 ms of pipeline
  builds at every size.
- **Constant shapes:** with the shape table (item 8), as the row says.
