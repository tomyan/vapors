# Gate shared lock: correctness phases overlap, measurements run alone

Today a gate holds two whole-run exclusive locks: the quiet lock
(`quiet.holder`, form `gate`, `quiet_wait::for_gate`) and `gate.lock`
exclusive behind `gate.turn` (`gate_lock::acquire`), both taken in
`gate_floor::begin` and held until the gate exits. So one gate runs on
the machine at a time, and no `ci` or `test` run starts while it runs.
Tom approved narrowing this (2026-11-22, through the lead) so that the
correctness phases of gates in separate worktrees can overlap, while
the steps whose results depend on the machine being quiet still run
alone.

Status: decided (Tom, 2026-11-22, through the lead); implemented in one PR, the three slices below together.

## Inventory

### Resources the gate's steps use

Timings are gate-603 (`~/.cache/handoffs/gate-603.log`, 808.6 s in all).
"Timing" means the step passes or fails on wall-clock deadlines or
frame timing that load can break; "correctness" means its result does
not depend on load beyond nextest's 60 s per-test kill
(`.config/nextest.toml`) and the job's `timeout-minutes`.

Every step uses the sccache server (shared by every worktree), the
worktree's own `target/` under `~/.cache`, and the gate's own TMPDIR
(`run_tmp`). None posts to GitHub; `gate --pr` posts after the gate
exits. Browser steps use a `mktemp -d` profile and a server on a port
the OS picks (port 0), so two gates never share a profile or a port.
Lavapipe is a CPU Vulkan driver; the gate hides `/dev/dri`
(`gate_sandbox.rs`), so the "GPU" is CPU time.

| Job: step | Time | Resources beyond the common ones | Sensitive to | Guarded today by |
|---|---|---|---|---|
| ci: install a CJK font (apt-install, setup lines) | 0.3 s | apt cache read | correctness | quiet + gate.lock excl |
| ci: the CJK font is there | 0.1 s | fontconfig | correctness | same |
| ci: dependencies (cargo-deny) | 9.4 s | network (advisory db) | correctness | same |
| ci: lint (fmt, clippy) | 50.6 s | CPU, all threads | correctness | same |
| ci: wasm lint | 37.2 s | CPU | correctness | same |
| ci: unit | 263.3 s | CPU; tests with 60 s kill | correctness (deadline only) | same |
| ci: host | 2.7 s | bwrap user namespace, user systemd | correctness | same |
| ci: build timings | 44.8 s | CPU | correctness | same |
| web: trunk build | 70.3 s | CPU, `target/web` | correctness | same |
| web: page contract in headless Chrome | 78.7 s | Chrome (SwiftShader), port 0 | timing: first-frame and per-check deadlines, frames at set intervals | same |
| web: the waker in headless Chrome | 6.5 s | Chrome, port 0 | timing: wakes 300 ms apart, a frame by 900 ms | same |
| web: the readout row | 4.1 s | Chrome, port 0 | timing: 45 s reading deadline | same |
| gpu: install lavapipe (setup lines) | 0.1 s | apt | correctness | same |
| gpu: lavapipe is the Vulkan device | 0.2 s | Vulkan loader | correctness | same |
| gpu: warm headless Chromium | 24.6 s | Chrome profile | timing: exists so the tests' 20 s screenshot deadline holds | same |
| gpu: gpu tier | 40.2 s | lavapipe (CPU), Chrome | timing: screenshot deadlines, frame counts per wake | same |
| gpu: text comparison page | 7.9 s | Chrome | timing: screenshot deadline | same |
| gpu: layout / a11y / events oracles | 92.9 s | Chrome, DevTools port 0 | timing: DevTools deadlines | same |
| gpu: the page's text input | 12.5 s | Chrome, ports 0 | timing: run deadline | same |
| window: screenshots (7 steps) | 32.8 s | Xvfb display (`xvfb_display.rs` picks a free one), lavapipe | timing: frame and screenshot deadlines | same |
| window: the waker in a window | 8.4 s | Xvfb (`xvfb-run -a`) | timing: one frame per wake | same |
| window: gpu-surface idle/invalidated/animating | 18.5 s | Xvfb | timing: frames per invalidate | same |
| window: the clipboard's image | 3.7 s | Xvfb, xclip | correctness | same |

By job: ci 408 s (all correctness), web 160 s (70 s correctness, then
89 s timing), gpu 178 s (timing), window 62 s (timing). No gate step
is a perf measurement against a budget; the timing steps are
correctness checks with deadlines that load breaks, which is why the
gate went exclusive ("GPU and page checks fail on timing under their
load", `gate_lock_shared.rs`). The perf measurement is `cargo xtask
scaling`, which is not in the gate.

### Entry points

| Entry point | Uses | Sensitive to | Guarded today by |
|---|---|---|---|
| `cargo xtask gate` | all of the above | as its steps | quiet (form gate) + gate.turn + gate.lock excl, whole run; load wait |
| `gate --pr` | the gate as a child, then `gh` posts | as the gate | the child's locks; posting unguarded (per PR) |
| `cargo xtask ci` | CPU (4 jobs, niced), sccache, the gpu tier on lavapipe | runs the gpu tier, but beside up to two other ci runs already | quiet wait, gate.turn, a ci slot (3), gate.lock shared, load wait; the quiet re-check |
| `cargo xtask test` | CPU, nextest | per tier | quiet wait only (declared limit in `quiet_wait.rs`: no share) |
| `cargo xtask quiet N -- cmd` | whatever cmd times | timing, by intent | quiet.holder (form run), then a drain: gate.lock excl taken and dropped |
| `quiet hold` / `release` | none itself | holds for others' timed runs | quiet.holder (form hold), drain; release removes it |
| pre-push hook | `cargo xtask lint`, `test --tier unit` | correctness | lint: nothing; test: quiet wait |
| queue pass (`queue_regate`, `queue_gate_child`) | git, gh, `gate --pr` child | as the gate | `queue_lock` per repository; the child's gate locks |
| nested xtask in a gate step (`lint`, `wasm`, `test`, `deny`, `setup`, `screenshot`, `web`) | as the step | as the step | no lock: `XTASK_GATE=1` skips the quiet wait and the ci share (`priority::in_gate`) |
| `cargo xtask scaling` | CPU, GPU timing judge | perf measurement | nothing (the nightly runs it under `nice`) |

## Decisions

- Holds per step, the wait added back to the job's clock; an unlisted
  step refuses the gate (Tom, 2026-11-22).
- A gate's shared hold takes one of the three ci slots (Tom, 2026-11-22).
- `cargo xtask test` takes a share without a slot after its quiet wait
  (Tom, 2026-11-22).
- One gate per worktree by a flock that waits, with a measured deadline
  (Tom, 2026-11-22).
- In its shared steps a gate builds with 4 jobs, as a ci builder does
  (`[build] jobs`; `gate_lock_slots.rs` counts three builders of 4 on 12
  threads); its exclusive steps keep every thread (Tom, 2026-11-23).
  The gate sets `CARGO_BUILD_JOBS` and `XTASK_GATE_JOBS` on each step,
  and an xtask the step runs keeps the latter (`priority.rs`).
- Losing a round to the quiet lock is not a failure for a gate: its
  share keeps trying until the quiet lock's and the gate lock's longest
  waits together, 101 min, then fails naming the holder. `ci` and
  `test` keep their three rounds (Tom, 2026-11-23).

## Design

### Holds

- **Shared**: a share of `gate.lock`, taken behind `gate.turn` as
  `cargo xtask ci` takes it today (`gate_lock_shared::acquire_shared`).
  Any number of shared holders run together.
- **Exclusive**: the quiet lock (form `gate`) and `gate.lock`
  exclusive behind `gate.turn`, as the gate takes them today
  (`gate_lock::acquire`), then the load wait.

`gate.turn` gives writer preference: an exclusive taker holds the turn
while it waits for the shares to go, and every shared taker must take
the turn briefly before its share, so no new share starts once an
exclusive taker is waiting. That is the mechanism ci runs and gates use
today; this change reuses it for gates' own phases.

### Which steps take which (Tom, 2026-11-22)

The gate takes a hold per step, before the step starts. Time spent
waiting for a hold is added back to the job's `timeout-minutes` clock
(`gate_run.rs` starts that clock at the job's first step), so a wait
never uses up the job's budget. Two tables in `.config/factory.toml`
name every step by its label, with the reason:

```
[gate.shared]
"ci: lint" = "fmt and clippy: correctness, no deadline but the job's"
...
[gate.exclusive]
"web: page contract in headless Chrome" = "first-frame and per-check deadlines in Chrome"
...
```

A planned step in neither table, or in both, refuses the gate before
any step runs, naming the step and the tables; so does an entry that
names no planned step, or has an empty reason, as `[gate.setup]` and
`[gate.actions]` refuse theirs. When consecutive steps take the same
hold it is kept; when the hold changes, the old one is released first
and the new one taken (a process cannot take `gate.lock` exclusive
while it holds a share of it: flock treats its two open files as two
owners). A run of consecutive shared steps is one share: an exclusive
taker waiting meanwhile keeps new shares out, and waits for that run to
end.

Short correctness steps that sit between exclusive steps (the Mesa and
Vulkan checks at the start of the gpu and window jobs, the clipboard
check) are marked exclusive, with that as their reason: a shared hold
there would give the machine up for under 4 s and queue the gate again
behind every exclusive phase waiting. Warming Chromium is exclusive:
it is there so the gpu tier's 20 s screenshot deadline holds. So
today's workflow switches once, inside the web job: the trunk build is
the last shared step (about 479 s shared and 330 s exclusive in
gate-603).

### Builders (Tom, 2026-11-22)

A gate's shared hold takes one of the three ci slots
(`gate_lock_slots.rs`), so at most three builders of either kind build
at once. Its exclusive hold takes none: it keeps every share out.

### Nested commands

Unchanged: a gate step's xtask sees `XTASK_GATE=1` and takes no lock,
since its gate holds what the step needs. A nested acquire by the same
gate therefore never waits on itself. The test for this runs, in a
second process with `XTASK_GATE=1`, `share::unless_in_gate_at`, the
function `ci_shared::run` and `test_shared::run` both call, while the
first process holds the exclusive hold.

### `cargo xtask test` (Tom, 2026-11-22)

Takes a share (no slot) after its quiet wait, with the quiet re-check
ci makes, so a gate's exclusive
phase waits for test runs already going and keeps new ones out. This
retires the declared limit in `quiet_wait.rs`.

### Same worktree (Tom, 2026-11-22)

The machine-wide lock today also keeps two gates out of one worktree
(they would share `target/` and `target/gate/github_output`). A gate
first takes an exclusive flock on `target/gate/run.lock`, writing its
owner line into it, and waits for it. The wait's deadline is the
longest gate measured, gate 270's 2,885 s, rounded up to the hour
(`quiet_lock::LONGEST`); past it the gate refuses, naming the holder
and the command to rerun.

### Names

Each share names itself in a marker beside the lock, `<who>.<pid>`
(`ci`, `gate` or `test`), so a waiter says what it waits for. An xtask
built before this change reads `ci.` markers only: it waits on a gate's
or a test run's share all the same (the flock), without naming it.

### Crash recovery

All flocks are freed by the kernel when their process exits. The quiet
lock's holder file is released by `Holder::ended` (pid gone or reused,
via `lock_owner`) or at its deadline, as today. `gate.holder` and
`ci.<pid>` markers are only names, cleared on read when their process
is gone. No new state outlives a process.

### Deadlines

Every wait keeps its own deadline: the quiet lock's (`quiet_lock::WAIT`,
61 min), the gate lock's for the turn, a slot and the lock together
(`gate_lock::WAIT`, 40 min, from touche's gate times), and the load
wait's, which goes on at its default 10 min. So one hold waits at most
111 min (`gate_deadline::HOLD_WAIT`). A gate takes two holds over
today's workflow, shared then exclusive (`HOLDS`, held to the tables by
a test), and its jobs' `timeout-minutes` add up to 42 min (`JOBS`,
held to the workflow by a test): the longest gate is 42 + 2 x 111 = 264
min (`gate_deadline::LONGEST`). A second gate in the same worktree
waits that long for the first (`gate_worktree_lock::WAIT`), and the
queue kills a gate after it and `gate --pr`'s three gh calls of 30 s
(`queue_gate_child::GATE_DEADLINE`). A gate's share tries rounds until 101 min
(quiet and gate lock together). Each quiet and lock wait is cut to what
is left of that when it starts, so the waits of a round together keep
within it; only a load wait begun just before the deadline can pass it,
by at most its own 10 min. So the share too is within 111 min. Declared limit: a `[load]
deadline_minutes` above the default is not counted; it can make a gate
outlive the deadline, which then kills or refuses it.

The exclusive hold's quiet holder file is renewed once the gate lock
and the load wait are had, so its 60 min deadline runs from the start
of the exclusive steps, not from before the waits.

### Transition

The lock files, their names and their formats do not change. An
old-protocol gate holds the quiet lock and `gate.lock` exclusive for
its whole run; a new-protocol process waits on both before any hold
(the quiet wait sees the old gate's form `gate` holder; the flock sees
its exclusive lock), so it treats the old gate as exclusive until it
ends. An old-protocol ci run takes the turn and a share; a new gate's
exclusive phase waits for it and keeps it out, as an old gate did. An
old-protocol gate arriving during a new gate's shared phase waits on
the turn and the lock, as it waits for ci runs today. Old `test`
binaries wait only on the quiet lock; the new gate's exclusive phase
still writes the quiet holder file, so they wait for it too.

Declared relaxed guard (`gate_hold.rs`, the exclusive hold): an old
`test` binary waits on the quiet lock only at its start and holds
nothing after. One started during a new gate's shared steps runs on
into its exclusive steps, and no file says it is there, so no contained
change can make the gate wait for it. Retired once every worktree's
xtask is built from this change or later; a rebase rebuilds it.

The order when switching from shared to exclusive is: release the
share, take the quiet lock, then the turn and `gate.lock` exclusive.
Taking the quiet lock while holding the share would deadlock two gates
both switching (each waiting on the other's share or quiet hold).

### A ci run beside a gate in its own worktree

Before, a ci run waited for any gate on the machine, its own
worktree's included. Now a ci or test run in a gate's worktree waits
only for the gate's exclusive steps; during its shared steps the two
build into the same `target/`, which cargo's own lock on the build
directory serialises.

### Not changed (open questions in the PR)

- The disk floor (30 GB, `gate_floor.rs`) was set for one gate at a
  time, about twice one gate's growth of 14.4 GB. Two gates overlapping
  from the floor would leave about 1 GB.
- `cargo xtask scaling`, a perf measurement, takes no lock today.
- `~/.cache/factory` holds stale `ci.<pid>` markers and
  `gate.holder.<pid>` temporary files from killed runs; harmless, since
  every reader checks the owner's liveness.

## Slices

1. A gate takes a hold per step from `[gate.shared]` and
   `[gate.exclusive]`, refusing an unlisted step; its wait is added back
   to the job's clock; the whole-run quiet and exclusive holds go; the
   shared hold takes a ci slot. Two-process tests: two shared holders
   overlap; an exclusive taker waits for active shares; new shares wait
   while an exclusive taker waits or holds; a dead holder's lock is
   recovered; two gates switching to exclusive at once both finish; an
   old-protocol whole-gate hold is respected; a nested xtask in a gate
   does not wait on its gate.
2. `cargo xtask test` takes a share.
3. The same-worktree guard.
