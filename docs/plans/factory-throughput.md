# Factory throughput: where gate and ci time goes, and the cuts

Status: slice 0 is built (#308): the step this document calls "lint,
wasm lint and unit" is now three steps, `lint`, `wasm lint` and
`unit`, and the ci.yml line numbers below are as they were before it.

Written as a design, before anything was built, on 2026-11-13 from
`origin/main` at 00dbc38b, the 65 gate logs in `~/.cache/handoffs`
(`gate-v*.log` and `gate-batch-*.log`, 2026-11-11 21:05 to 2026-11-13
18:27, every one run in `~/.cache/w-gate204/vapors`, whose target
directory persists between gates), the `cargo-timing.html` that gate
271's "build timings" step left in that target, and the machine as it
was at 18:35 on 2026-11-13. Every number says where it came from;
"inferred" marks the ones that are not measured.

Contents

1. Where the time goes
2. The machine, the load, and how many builders it sustains
3. Ranked cuts
4. The order of slices
5. What a run would settle that the logs cannot

## 1. Where the time goes

### 1.1 The gate, step by step

The gate runs the 27 steps of `.github/workflows/ci.yml` in file order
in one worktree (`xtask/src/gate.rs:1-14`), after taking the gate lock
and waiting for the load (`xtask/src/gate_floor.rs:39-44`). Medians over
the logs, split at #233 (2026-11-12 13:25, `[profile.test] opt-level =
1`, root `Cargo.toml:100-101`), which changed two steps; "warm" is the 13
gates whose lint-and-unit step took under 45 s, meaning cargo found
nothing to compile.

| step (ci.yml line) | before #233, n=36 | after #233, n=29 | warm, n=13 | what it is |
|---|---|---|---|---|
| ci: dependencies (cargo-deny) (67) | 1.5 | 1.5 to 10 | 1.5 | advisory fetch and check |
| ci: lint, wasm lint and unit (69) | 104 (28 to 182) | 151 (28 to 285) | 32 | fmt, files, layers, builds, imports, clippy `--all-targets`; clippy for wasm, debug and release; nextest build of every test target in the `test` profile and the run of 3,789 non-gpu tests |
| ci: build timings (72) | 1.1 (0.5 to 10.6) | 61 (0.8 to 195) | 0.9 | `cargo build --workspace --all-targets --timings`, dev profile |
| web: build the web demo with trunk (118) | 37 (22 to 70) | 43 (22 to 109) | 23 | wasm32 release build, wasm-bindgen, wasm-opt, brotli and gzip |
| web: page contract in headless Chrome (124) | 60 (54 to 159) | 60 (55 to 250) | 57 | about 40 headless Chromium launches, one after another (section 1.3) |
| web: the waker in headless Chrome (127) | 7 | 7 | 7 | one launch, timed waits |
| gpu: warm headless Chromium (190) | 3 to 9 | 3 to 9 | 3 | |
| gpu: gpu tier (192) | 37 (32 to 55) | 26.5 (25 to 39) | 31 | 299 `gpu_` tests through nextest |
| gpu: text comparison page (196) | 8 to 11 | 8 to 11 | 8 | |
| gpu: layout oracle (201) | 30 to 50 | 30 to 50 | 30 | Chromium's boxes for every fixture |
| gpu: a11y oracle (205) | 2 | 2 | 2 | |
| gpu: events oracle (210) | 28 | 28 | 28 | |
| gpu: the page's text input (222) | 12.5 | 12.5 | 12.5 | |
| window: the eight steps (274 to 302) | 35 together | 35 | 35 | Xvfb screenshots, waker, surface, clipboard |
| 27 steps in | 369 (264 to 577) | 479 (235 to 890) | 264 | |

Sources: the table is `scratchpad/gate-table.txt`, one row per log,
built from each log's `ok   <job>: <step> <seconds>s` lines
(`xtask/src/report.rs` prints them; `xtask/src/gate_run.rs:46-49` adds
the load after each). The step texts are the ci.yml lines cited.

What the table says:

- The compile steps (lint-and-unit, build timings, trunk) are the
  variable part: 32 + 1 + 23 = 56 s warm, 151 + 61 + 43 = 255 s at the
  median after #233, up to 285 + 195 + 109 = 589 s in gate 270 (load 28
  to 110 throughout, `gate-batch-270.log:45-66`).
- The browser and GPU steps are a floor of about 200 s whatever the
  tree: total minus the three compile steps is 202 s median in the
  warm gates and 217 s in the cold ones after #233. They do not get
  much slower under load: page contract against load at the step's
  start correlates at 0.19 over 65 gates; the compile steps at 0.57.
- #233 cost 60 s a gate at the median: the "build timings" step went
  from 0.5 to 2.1 s in all 36 gates before it to 50 to 195 s in every
  cold gate after it (section 1.2 says why), and the lint-and-unit
  step's median rose 47 s (the test profile now compiles the
  dependency tree at opt-level 1 as well as dev's at 0, and the test
  binaries at 1 are slower to compile). It bought 10 s on the gpu tier
  (37 to 26.5 s) and the kept_shift tests their margin under nextest's
  60 s (#233's message).

### 1.2 What the compile steps compile

**lint, wasm lint and unit.** `cargo xtask lint` is fmt, files, layers,
builds, imports and `cargo clippy --workspace --all-targets -- -D
warnings` (`xtask/src/steps.rs:147-149`, `48-62`); `cargo xtask wasm`
is clippy for `vapors-web-demo` on wasm32, debug then release
(`steps.rs:137-139`); `cargo xtask test --tier unit` is `cargo nextest
run --workspace -E 'not test(/^gpu_/)'` (`xtask/src/steps_tiers.rs:13-23`),
which builds every test target of the workspace in the `test` profile,
gpu ones included (the filter is by test name, not binary), then runs
them. The warm floor of 28 to 29 s (gates v206, v208, v212, v241, v244 and
v245) bounds the lints' own work plus the run of 3,789 tests;
everything above it is compilation of what changed and what depends on
it. The split between clippy, the two wasm clippies, the nextest build
and the nextest run is not in any log: ci.yml runs the three commands as
one step, and no `cargo xtask ci` log with its per-step lines exists on
disk for vapors (agents' ci output goes to their transcripts). Slice 0
gets the split for free.

**build timings.** The step is `cargo build --workspace --all-targets
--timings`, `if: always()`, and its only consumer is the
`upload-artifact` step after it (ci.yml:70-80); nothing reads
`target/cargo-timings/` on the machine. Gate 271's run of it, parsed
from `~/.cache/vapors-targets/w-gate204/cargo-timings/cargo-timing.html`
(build start 2026-11-13T18:16:57Z, profile dev, jobs=12):

| units | count | CPU-s |
|---|---|---|
| test harnesses (`"test" (test)` and `"lib" (test)`) | 488 | 1,070 |
| dependencies (mostly sccache hits) | 198 + 27 | 127 |
| examples | 21 | 42 |
| bins | 6 | 14 |
| build scripts | 14 | 4 |
| all | 754 | 1,257 in 108.8 s wall |

So 85 % of the step compiles the test targets a second time, in the
dev profile at opt-level 0, because since #233 the `test` profile
differs from `dev` and cargo keeps one output per profile: every one
of the 477 executables in the gate target's `debug/deps` is there in
two copies, 63.2 GB together, 33.9 GB if there were one (section 2.3).
Before #233 the two profiles were the same, the test binaries nextest
had just built were up to date, and the step took 0.5 to 2 s.

The step is not useless to the gate: the gpu and window jobs' `cargo
run --example` and `cargo xtask screenshot` steps (ci.yml:190-201,
274-302) need the dev-profile examples and bins, and find them built.
Those are the 21 + 6 units, 56 CPU-s.

**trunk.** `trunk build --release` of `vapors-web-demo` into
`target/web` (`xtask/src/trunk.rs:46`), then wasm-bindgen, wasm-opt and
precompression (`xtask/src/web.rs:109-113`). The 22 s warm floor is the
post-compile tools: with nothing to compile the step still took 21.5 to
23 s in eleven gates. The wasm32 target has its own dependency build
(`target/wasm32-unknown-unknown`, 517 MB in the gate target).

### 1.3 The browser floor

`scripts/check-page-contract.sh` launches headless Chromium once per
case, one after another: 12 `check` pages, 12 `overlap` pages (one per
demo plus index), the as-built screenshot, and one `shoot` per demo per
viewport section of its `.pixels` file (11 demos, lines 122 to 146 and
400 to 481 of the script as `git show origin/main:` prints it). That is
about 40 launches in the 54 to 60 s floor, 1.4 s each; under load they
stretch (250 s in gate 271 at load 17 to 26; 159 s in v207). The pages
are independent: each is its own copy of the built page, the hold server
keys its `/wait/<name>` and `/done/<name>` by page name (script lines
44-53), and the script's only shared state is the `$dom` variable and
the `fail` counter, both read right after each launch.

The layout oracle (30 to 50 s) and the events oracle (28 s in every
gate, so timed waits rather than CPU, inferred) were not read for this document;
whether their fixtures are driven one at a time is the first thing to
measure before touching them (section 5).

### 1.4 ci

`cargo xtask ci` (`xtask/src/steps.rs:119-134`): tools, fmt, files,
layers, builds, imports, deny, clippy, wasm, wasm release, unit, gpu,
fastest first. It takes the gate lock shared (`xtask/src/ci_shared.rs:24-25`),
waits for the load first (`ci_shared.rs:19`), and runs every step at
`nice 10` with `CARGO_BUILD_JOBS=4` (`xtask/src/priority.rs:1-6`, `32`).
Its unit and gpu tiers are the gate's with the same warm floors (about
30 s and 26 s); there is no build-timings step and no trunk.

How long a ci run takes is in the gate logs only indirectly: a waiting
gate prints each ci holder and how long it has held the lock
(`xtask/src/gate_lock.rs:105`). Gate 270 waited 1,395 s for five ci runs
that had held the lock 1,122, 1,230, 1,344 and 1,395 s when they finished
(`gate-batch-270.log:1-24`); gate 271 waited 402 s for two
(`gate-batch-271.log:1-7`); gate 274 222 s for one. So a ci run beside
four others took 19 to 23 minutes; beside one or two, 7 minutes. Inferred:
alone on the machine, with the same change, under 5 minutes (the warm
floor is about a minute, and the crate-structure plan's derived figure
for an elements change is 3,254 CPU-s of test-profile workspace
compilation, `docs/plans/crate-structure.md` 1.3, which at 4 jobs on an
idle core each is about 13 minutes and at 12 jobs about 5; that figure
was derived under load and is an upper bound).

### 1.5 The waits

After #233 the gate's wall clock, waits included, was 566 s median and
724 s mean against 464 s of steps: waits are 36 % of a gate's wall
(lock waits at one line a minute, load waits at one line per 30 s,
summed from the logs). The worst was gate 270: 1,395 s for the lock,
then the full 600 s load deadline at load 28 to 88, then 890 s of steps,
2,885 s in all. Two waits stack:

1. The lock wait: the gate takes `gate.turn` and then waits for every
   ci run that holds the lock shared to finish
   (`xtask/src/gate_lock_shared.rs:5-9`); ci runs that arrive after the
   gate queue behind it. Its length is the longest remaining ci run,
   which section 1.4 puts at 7 to 23 minutes when several run.
2. The load wait, taken after the lock (`gate_floor.rs:43`), waits while
   the one-minute load is over 12, up to 10 minutes, then goes on saying
   so (`xtask/src/load_wait.rs:83-100`; the deadline is Tom's,
   `docs/decisions/2026-11-01-load-wait-deadline.md`). The one-minute
   load is an exponential average with a one-minute time constant: in
   gate 271 it read 69, 43, 28, 18 at 30 s intervals once the ci runs
   had gone (`gate-batch-271.log:8-11`), a decay, not CPU in use. When
   it does not decay (gate 270, 34 to 88 for ten minutes) the CPU is
   taken by work that holds no lock: agents' plain `cargo build`,
   `cargo run` and tests, which the wait cannot stop and the deadline
   lets the gate run beside anyway.

## 2. The machine, the load, and how many builders it sustains

### 2.1 The machine

Intel i5-10500T, 6 cores, 12 threads, 2.3 GHz base (`/proc/cpuinfo`);
62 GB RAM, 10 GB in use, 50 GB page cache, no swap (`free -g`,
`swapon`); one NVMe, 898 GB, 687 GB used, 166 GB free at 18:35
(`df -h /`); `/tmp` a 32 GB tmpfs. No mold, lld or clang installed
(`command -v`); bwrap, Chromium 155, sccache 0.18 and nextest 0.9.149
are (`.config/tools.toml`).

At 18:35 the load was 39 (one-minute), 25, 27; four `xtask ci` runs were
live at nice 10; 40 rustc, cargo, trunk and Chromium processes; cargo
processes in six vapors worktrees; 43 vapors worktrees touched in the
last 24 hours. `/proc/pressure/cpu` read `some avg60=63.8`: for 64 % of
the last minute at least one runnable task was waiting for a CPU.
`/proc/pressure/io` read `some avg60=0.59` and memory `0.02`: the disk
and memory are not where time waits. CPU is.

Load at the start of gate steps, over 324 readings in the 12 batch
logs: deciles 6.0, 7.0, 8.4, 9.5, 11.1, 12.0, 16.7, 20.4, 27.8, max
110.6. Half the gate's steps start with the load under 11 (the load
wait works when the lock wait emptied the machine), a tenth over 28.

### 2.2 How many builders

Each ci run compiles with 4 jobs (`priority.rs:32`) and runs its tests
on nextest's default of 12 threads (`.config/nextest.toml` sets none),
plus a Chromium per gpu test that opens one. Plain `cargo` outside xtask
is not capped: cargo's default is 12 jobs (`priority.rs:15` declares
the limit). The gate runs at 12 jobs, normal priority.

Three ci runs compiling at once are 12 compile jobs on 12 threads of 6
cores: that is the point past which adding a builder adds wall time
to every builder and nothing to total throughput. The evidence is the
ci hold times in section 1.4 (7 min with 1 to 2 others, 19 to 23 min
with 4 others, about 3x for about 2.5x the builders, so throughput did
not rise), the lint-and-unit step's 0.57 correlation with load, and
gate 270 (890 s of steps at load 28 to 110 against 480 s at load 4 to
12 for the same steps). Inferred, not measured: with six physical
cores and hyperthreading, 12 compile jobs are worth about 7 to 8 cores
of rustc; the second thread of a core adds about a quarter. Memory is
not a bound at ten builders (10 GB in use with four ci runs live).

So the box sustains about three concurrent builds at full speed, and
every builder past three only queues in the kernel instead of in a lock,
while the load wait turns each new ci and gate into a 10-minute sleep
before it joins the queue anyway. Section 3's cut C4 makes the queue
explicit.

### 2.3 Target directories

12 vapors target directories hold 402 GB; the other 329 of the 341 are
empty (`du -sh ~/.cache/vapors-targets/*`, 2026-11-13 18:40): w-f0 96
GB, w-gate204 70, w-frames 57, w-lane 54, w-s1 40, w-b0 36, w-quality
36, w-noinc 9.8, four under 2 GB. The gate's 70 GB is `debug/` 69 GB
(release 179 MB, wasm32 517 MB), of which `deps/` is 64 GB: 989
executables, 63.2 GB; 2,239 rlib and rmeta files, 4.6 GB; examples 4.3
GB; `incremental/` 108 MB (off); `build/` 711 MB. The executables are
477 distinct names each present twice (the dev and the test profile,
section 1.2): one copy each would be 33.9 GB. The 174 elements test
binaries are 96 MB each, 16.7 GB in one profile and 31.0 GB in two.

Agent targets have the test binaries once (nextest builds the test
profile; `cargo run` the dev bins and examples) unless the agent runs
`cargo build --all-targets` or `cargo test`, and a wasm32 tree when they
run `cargo xtask web`. Inferred from the gate's composition: an agent's
35 to 90 GB is mostly the 477 statically linked test executables,
about 100 MB each, so the count of test binaries is the size, and the
cut that reduces the count (C2) halves the disk as a side effect.

## 3. Ranked cuts

Each with the saving and its basis, the risk to what the gate proves
(it must still run every step ci.yml runs, as CI runs it), the effort
in pull requests, and how the cut is proved to have kept coverage.

### C1. The timings step stops compiling the test targets again

Change ci.yml:72 from `cargo build --workspace --all-targets --timings`
to `cargo build --workspace --bins --examples --timings`. The step then
builds what the gpu and window jobs' `cargo run` steps need (the dev
bins and examples and their libs, 56 CPU-s of the 1,257 in gate 271)
and still uploads a timings report; the 488 test harnesses, built by
nextest in the step before, are not built a second time in another
profile.

- Saving: 60 s median, 52 s mean, up to 195 s, per gate, and about
  1,000 CPU-s per gate handed back to the builders. Basis: 36 gates
  before #233 at 0.5 to 2.1 s against 29 after at a 61 s median; gate
  271's unit table (section 1.2). Also 30 GB less per target that runs
  it.
- Risk to proof: none. The step checks nothing; `if: always()` and
  `if-no-files-found: ignore` say the artefact is informational. CI's
  `ci` job gets the same cut, which also keeps it clear of its 9-minute
  timeout (ci.yml:34; its lint-and-unit step took 273 s on the last
  successful main run, 43204069045, before #233).
- Effort: one PR, two lines and a comment.
- Proof: the next gate's `cargo-timing.html` lists no `(test)` units
  and the step is under 10 s; the gpu job's first `cargo run` step does
  not grow (the examples were built here either way). Break: run the
  old line in the gate worktree once and watch the 488 units return.

### C2. One integration-test binary per crate

Each file under `crates/<crate>/tests/` is its own binary that compiles
`mod support` (28 files for elements) and links the crate's closure
(`crates/elements/tests/incremental_styles.rs:5`; `docs/plans/
crate-structure.md` 1.3). Counts: elements 174, css 60, app 54,
compiler 46, compiler-examples 22, renderer 19, dom 15, signals 11
(`git ls-tree origin/main`). One binary per crate compiles support once
and links once. nextest still runs each test in its own process, so
isolation between tests does not change; what changes is 174 rustc
invocations and 174 links into one.

- Saving, basis: elements' 173 harness units were 1,435 CPU-s in the
  test profile (crate-structure 1.3, mean 8.3 s) and 675 CPU-s in dev
  (gate 271: 177 units; mean 3.7 s); app 377 and 162; css 183 and 37.
  The single binary's cost is the compile of the test code itself once
  (23,600 lines for elements) plus one link. Expected: the test-harness
  part of the unit tier's build falls from 3,076 to a few hundred CPU-s
  on a clean build (inferred from the per-binary means; the measured
  number is step 1 of crate-structure's plan, `cargo test -p
  vapors-elements --no-run --timings` before and after). On the gate
  this is inside the lint-and-unit step on every elements change, which
  is most PRs; on ci it is the unit tier on every push. Until C1 lands
  it also cuts the timings step. Disk: 174 binaries of 96 MB become
  one, 16.7 GB per profile per worktree.
- Risk to proof: the same test functions run under the same names, so
  the tiers' name filters (`steps_tiers.rs:17`, `29`) and insta's
  snapshot paths (derived from each file's path, unchanged) hold. Two
  things change: a compile error in one test file now fails the whole
  crate's tests instead of one binary (it failed the tier either way),
  and `cargo nextest run <name>` reproduces the same. The real cost is
  the module list: a `tests/main.rs` with one `mod` line per file is a
  merge conflict for every open PR that adds a test file. So the list
  is generated: `autotests = false` plus one `[[test]]` in the crate's
  Cargo.toml, and a `build.rs` that globs `tests/cases/*.rs` into
  `OUT_DIR` as `#[path = "..."] mod name;` lines, which `tests/main.rs`
  includes. Each file's `mod support;` becomes `use super::support;`
  (one `sed`, since every file declares it the same way); the `use
  support::...` lines that follow resolve through that import. That
  move (files into `tests/cases/`) is the one change open PRs must
  rebase over; a PR adding `tests/foo.rs` after the move adds
  `tests/cases/foo.rs` and nothing else.
- Effort: a script in xtask (`cargo xtask one-test-binary <crate>`,
  mechanical work is a script) and one PR per crate: elements first
  (the script, the build.rs, the move: the diff is mostly renames), then
  app, css, compiler, compiler-examples, renderer, dom, signals by the
  same script. Under 400 lines of non-rename diff each.
- Proof: `cargo nextest list -p vapors-elements` prints the same test
  names and count before and after (the PR records both); one test
  broken on purpose fails the tier naming it; the unit tier's time on
  the gate (slice 0 splits it out) before and after.

### C3. The load wait after a lock wait

The gate's load wait after a lock wait is a wait for a decaying
average, 90 to 120 s to fall from 60 to 12 once the ci runs are gone
(gate 271; v126 and v247 waited 22 lines, 660 s, at load 19), and when
it does not decay the deadline runs the gate anyway (gate 270). Change
`load_wait` to read what the wait is for: CPU in use now, not the
one-minute average. `/proc/pressure/cpu`'s `some avg10` is the share of
the last 10 s with a runnable task waiting; `/proc/loadavg`'s fourth
field is the count of runnable tasks now. Wait while the runnable count
is over the thread count, read every 5 s, with the same 10-minute
deadline; say the number waited on.

- Saving: 108 s mean per gate (3.6 load-wait lines of 30 s), 600 s at
  worst; and the same for every `cargo xtask ci`, which waits the same
  way (`ci_shared.rs:19`) and then holds the lock for the gate longer.
- Risk to proof: a gate starting into real load. The page contract's
  correlation with load is 0.19 and gate 270 passed every step at load
  28 to 110 with its longest step at 2.2x the median; the per-step
  deadlines are 2.5x worst measured (ci.yml:30-34, 94-104). A runnable
  count under 12 is a stronger condition than a one-minute average
  under 12, not a weaker one.
- Effort: one PR in `xtask/src/load_wait.rs` and its tests (the wait
  takes a `load` closure, so the fake is in place), the decision doc
  2026-11-01 amended to say what is read and why.
- Proof: the unit test feeds a sequence (over, over, under) and asserts
  the calls and lines; break by feeding a never-under sequence and
  watching the deadline line; the next ten gate logs' wait lines.

### C4. A cap of three concurrent ci runs

Section 2.2: past three builders at 4 jobs each, every builder waits in
the kernel and throughput does not rise. Give the shared lock a count:
a ci run takes one of three slots (three lock files tried in turn, each
exclusive, held for the run) after the turn; a fourth waits, printing
who holds the slots as the gate does now, with the same 40-minute
deadline (`gate_lock.rs:24`). The gate still takes the turn first and
waits for the slots to empty, so its lock wait becomes at most the
longest of three ci runs at full speed instead of the longest of five
at a third of it. Pair it with `[build] jobs = 4` in `.cargo/config.toml`
so plain `cargo` outside xtask is capped the same way (the gate sets
`CARGO_BUILD_JOBS=12` explicitly instead of clearing it,
`priority.rs:73`; on CI's 4-core runner 4 is the default already).

- Saving: inferred, since it moves wall time rather than removing CPU
  work: a ci run beside two others at 7 minutes (section 1.4) instead
  of beside four at 20; the gate's lock wait bounded by one such run
  instead of 1,395 s. The total work per hour is the same or better (no
  oversubscription; the sccache server and the page cache are shared).
  An agent whose ci waits for a slot waits in a lock with a line saying
  so, not in a 20-minute build.
- Risk to proof: none to the gate's steps. The risk is deadlock between
  turn and slots, which the gate's existing turn order already handles
  for one lock; the slot tests cover a gate arriving while three ci
  runs hold slots and two more wait.
- Effort: one PR in `gate_lock_shared.rs` and `gate_lock.rs` with tests,
  about 150 lines; the config line and the gate's jobs line a second,
  small PR.
- Proof: the tests above; the next day's gate logs' lock waits (the
  holder lines now show at most three) and ci hold times.

### C5. The page contract's launches in parallel

Run the independent `check`, `overlap` and `shoot` launches four at a
time (`xargs -P 4` over the case list, each case writing its DOM,
screenshot and verdict to its own file under `$work`, the serial tail
reading the files), keeping the as-built check first since it is the
one that proves the browser and driver work.

- Saving: inferred, 57 s to about 20 s warm (40 launches of 1.4 s, four
  at a time on six cores, with the Python server shared); under load
  the gain is smaller in proportion since the launches then compete
  with builds. About 35 s a gate.
- Risk to proof: the same cases with the same assertions; the risk is
  in the rewrite of a 481-line shell script's bookkeeping (`$dom`,
  `fail`, `console`). The hold server already keys requests by page
  name, so pages do not wait on each other's `/wait`.
- Effort: one PR, about 150 lines changed in the script.
- Proof: the script's final line names every case it ran
  (`check-page-contract.sh:481`); it must list the same cases, and a
  `.pixels` value changed on purpose must fail the matching shot and
  name it.

### C6. mold for native links

No fast linker is installed. Each test binary, bin and example links a
closure of up to 199 packages with ld.bfd through `cc`; cargo's timings
do not separate link from compile, so the link share is not measured.
After C2 the link count falls from 477 to about 30 per profile, so this
ranks after it. Install mold through `cargo xtask setup` (a hash-pinned
download in `[downloads]`, as static-web-server is) and on CI through
the apt-install action; set `-C link-arg=-fuse-ld=mold` for
`x86_64-unknown-linux-gnu` in `.cargo/config.toml` (gcc 14.2 on this machine accepts it; ubuntu-24.04 ships gcc 13,
which does too).

- Saving: to be measured first (section 5); mold links are commonly 3
  to 10x faster than bfd, and the link is a serial tail of every
  `cargo run` an agent does.
- Risk to proof: the gate and CI must link with the same linker, so the
  config applies to both or neither; a rustflags change changes every
  sccache key once (one cold fill across the machine, about the 1,257
  CPU-s of a full dev build plus the test profile's).
- Effort: two PRs: the measurement; then setup, ci.yml, config.
- Proof: the measurement's before and after; `cargo xtask check-tools`
  refuses a missing mold.

### C7. Splitting the gpu tier

The gpu tier is 25 to 27 s after #233 for 299 tests. There is nothing
to split for time; its cost is in its build, which the unit tier's
build already covers (same binaries). Not recommended.

### C8. Sharing a gate target between batches

Already the case: every gate since 2026-11-11 ran in
`~/.cache/w-gate204/vapors` (`gate-train.sh:12`, `gate-batch.sh:9`)
whose target persists, which is why warm gates take 264 s. Nothing to
do beyond keeping it so, and C1 halves it.

### C9. A shared target for the dependency layer

Not recommended. cargo has one target directory per build with one
build lock (`target/debug/.cargo-lock`): a target shared by ten agents
serialises their builds and thrashes the workspace crates whose paths
differ per worktree. The dependency layer is already shared through
sccache (86 % hit rate over the server's life, `sccache --show-stats`),
and per-agent target size is the test binaries (section 2.3), which C2
addresses. One thing to measure: 571k of the server's 797k requests
were "non-cacheable calls" (not compilations, by sccache's count); what
they are needs `SCCACHE_LOG` on one run (section 5).

### C10. Skipping unchanged tiers by content hash

Not recommended now. cargo already skips unchanged compilation
(fingerprints), which is the whole difference between a 264 s and a
480 s gate. What a content hash could skip beyond that is the 200 s
browser and GPU floor, keyed on the built page's bytes, the scripts,
and the browser version, and only when a passing gate recorded that
exact key. That is honest only if the record is written by the gate
alone, keyed by the full input set, and the skip prints the key and the
gate sha that proved it; and it still breaks the rule that the gate
runs every step as CI runs it, since CI skips nothing. Batch gates,
which gate several PRs' changes together, would rarely hit. Revisit if
the floor is the dominant cost once C1 to C5 are in.

### C11. nextest

Already in use, with per-test deadlines and no retries
(`.config/nextest.toml`). Its build phase is cargo's; its run phase is
the warm floor's 30 s. Nothing to gain beyond what is set, and the unit
tier cannot avoid building the gpu binaries (the tiers split by test
name, `steps_tiers.rs:5-10`).

## 4. The order of slices

Smallest first; each under 400 lines; each proved as its cut says.

| slice | cut | lines | what it proves before the next |
|---|---|---|---|
| 0 (done, #308) | split ci.yml's "lint, wasm lint and unit" into three steps, `cargo xtask lint`, `cargo xtask wasm`, `cargo xtask test --tier unit`, same order, same env (the gate stops at the first failure as `&&` did) | about 15 | the gate log measures lint, wasm and unit apart; the baselines for 2 and 4 |
| 1 | C1, the timings step without test targets | 3 | 60 s a gate; the next gate's timings file has no `(test)` units |
| 2 | C3, the load wait reads runnable tasks | about 100 | wait lines in the next gates |
| 3 | C4, three ci slots, `[build] jobs = 4` for plain cargo | about 150 + 10 | ci hold times and gate lock waits in the next day's logs |
| 4 | C2 for elements: the xtask script, build.rs, the move | about 200 non-rename | `nextest list` counts; the unit step's time from slice 0 |
| 5 | C2 for app, css, compiler, compiler-examples, renderer, dom, signals, one PR each by the script | small each | the same |
| 6 | C5, the page contract four launches at a time | about 150 | the case list; a broken pixel fails |
| 7 | C6 measured, then installed | 2 PRs | the measurement |
| 8 | the oracles, if the measurement in section 5 shows serial launches | after measuring | |

Expected gate after slices 0 to 6, cold, at the median: 479 s less 60
(C1), less most of the unit build's test-harness share (C2; not
measured, the crate-structure plan's step 1 gives the number), less 35
(C5), with waits of 724 - 464 = 260 s mean cut to the lock wait alone
(C3) and that bounded by one ci run (C4). The floor stays about 165 s
(browser and GPU steps less the page contract's cut) plus 56 s of warm
compile steps.

## 5. What a run would settle that the logs cannot

Each is one run in a quiet window, or slice 0's logs:

1. The split of lint, wasm and unit (slice 0 gives it on every gate).
2. The unit tier's compile against run: `cargo nextest run` prints
   `Starting N tests` with a timestamp when `--no-capture`-free output
   is kept; or `cargo test --workspace --no-run --timings` then the
   run.
3. C2's number: `cargo test -p vapors-elements --no-run --timings`
   before and after the one-binary change (crate-structure plan, 5.1).
4. The link share: time `touch crates/elements/tests/<one>.rs && cargo
   test -p vapors-elements --no-run` for a small test file (its compile
   is seconds; the rest is the link), with bfd and with mold.
5. The oracles: whether `layout_oracle` and `events_oracle` drive one
   Chromium per fixture (read `crates/elements-corpus/examples/`), and
   where the events oracle's constant 28 s goes.
6. sccache's 571k non-cacheable calls: one ci run with
   `SCCACHE_LOG=debug SCCACHE_ERROR_LOG=<file>`, counted by reason.
7. A ci run alone on the machine, for the base the concurrency numbers
   in 2.2 are inferred against.
