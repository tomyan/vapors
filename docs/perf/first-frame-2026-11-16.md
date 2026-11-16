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
  (`winners_runtime` 0, pinned by `crates/perf` mount tests). At first
  frame this saves nothing and costs 0.9% more instructions, almost all
  of it bookkeeping added by B3 (#437); see Attribution below.
- **Where the first frame goes:** `App::start` is 92% of the wait at
  10,000 rows warm (833 of 909 ms), and layout is 73% of that (606 ms);
  style is 81 ms. A cold shader cache adds about 120 ms of pipeline
  builds at every size.
- **Constant shapes:** with the shape table (item 8), as the row says.

## Attribution (judged instructions, 10,000 rows)

Each figure is the median of 3 runs of the judged binary (`vapors-perf
--first-frame 10000`), measured at main's revision either side of each B
merge. These were taken under load, which instruction counts tolerate:
the largest spread between runs is 0.67 M in layout and under 0.1 M in
mount, style and paint, except one style spread of 0.35 M. The outputs
are in `first-frame-2026-11-16/attribution/<point>/`; the three
measurement-only patches are beside them.

| point | revision | mount | start: style | start: layout | start: paint | frame 0 |
|---|---|---|---|---|---|---|
| before B2 (#428 merged) | 2f3678ad | 307.52 M | 429.92 M | 2,290.67 M | 326.63 M | 7.42 M |
| after B2 (#429) | 90b86ac7 | 307.54 M | 429.83 M | 2,290.64 M | 326.63 M | 7.43 M |
| before R1a | 8bd8995b | 307.50 M | 429.75 M | 2,290.69 M | 326.63 M | 7.42 M |
| after R1a (#430) | 09543507 | 307.50 M | 429.68 M | 2,290.68 M | 326.66 M | 7.42 M |
| before B3 | b2cabe32 | 307.53 M | 429.80 M | 2,291.40 M | 326.63 M | 7.42 M |
| after B3a and B3b (#437) | 160b9663 | 318.80 M | 448.48 M | 2,291.31 M | 326.64 M | 7.42 M |
| current main (#446) | ff808875 | 318.83 M | 448.13 M | 2,291.40 M | 326.62 M | 7.42 M |

Change from before B2 to current main, by cause (M instructions):

| cause | mount | start: style | start: layout | start: paint |
|---|---|---|---|---|
| B2 (#429) | +0.01 | -0.09 | -0.03 | 0.00 |
| R1a (#430) | 0.00 | -0.06 | -0.01 | +0.03 |
| B3a and B3b (#437) | +11.27 | +18.68 | -0.09 | +0.01 |
| other merges between B2 and R1a (#432, #431, #427, #424, #426, #436) | -0.04 | -0.08 | +0.05 | 0.00 |
| other merges between R1a and B3 (#422, #434, #439, #441, #442, #443) | +0.03 | +0.11 | +0.72 | -0.04 |
| #446 | +0.03 | -0.35 | +0.10 | -0.02 |
| total | +11.30 | +18.21 | +0.73 | 0.00 |

#437's change, split by removing one part at a time at 160b9663 with a
measurement-only patch, each on top of the last:

| part removed | mount | start: style |
|---|---|---|
| release verification (`noverify.patch`: no full-cascade comparison of the first element per key per state, and no runtime sharing answer computed beside the compiled one) | -0.02 | +1.29 |
| the sheet hash check on every mount (`noverify-nohash.patch`: `compiled_text` hashes the sheet's text each time a row mounts) | +11.13 | +0.30 |
| check 1 on every keyed element (`nov-nohash-nocheck1.patch`: `compiled_key` looked up in two SipHash `HashMap` probes before the sharing decision) | +0.05 | +16.71 |
| what is left: styles computed from compiled winners instead of the full cascade | +0.11 | +0.39 |

What compiled winners do and do not buy at first frame:

- They buy no first-frame time. With B3's three added costs removed,
  computing styles from compiled winners is within 0.4 M instructions
  (0.09%) of the runtime cascade it replaces.
- B3 added 30.0 M instructions (0.9%). 11.1 M of that is hashing the
  sheet text on every mount, 16.7 M is looking up the key's compiled
  rules on every keyed element, and 1.3 M is release verification. The
  first two run once per row or element; each could run once per sheet
  or key.
- The earlier comparison in this PR (a work-in-progress commit on
  #428's branch, 555839d3f849) showed layout 2,261 M and paint 312.5 M.
  Merged #428 already measures 2,290.67 M and 326.63 M, so that gap
  came before B2, not from the B steps or the other merges.
- Of the other merges, only the lane layout work between R1a and B3
  moves anything measurable: layout +0.72 M (0.03%).

