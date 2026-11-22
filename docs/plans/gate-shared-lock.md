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

Status: proposed. The decisions marked **to confirm** are open.

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

### Which jobs take which **to confirm**

The gate takes a hold per job, before the job's first step, so a lock
wait never counts against the job's `timeout-minutes` (`gate_run.rs`
starts the job's clock at its first step). A new table in
`.config/factory.toml` names each job's hold with its reason:

```
[gate.shared]
ci = "compiles and correctness checks; the unit tier's 60 s kill is its only deadline"

[gate.exclusive]
web = "Chrome checks on frame timing and first-frame deadlines"
gpu = "lavapipe on the CPU and Chrome under screenshot deadlines"
window = "Xvfb windows counting frames per wake"
```

A job in neither table, or in both, refuses the gate before any step
runs, naming the job and the table to add it to. When consecutive jobs
take the same hold it is kept; when the hold changes, the old one is
released first and the new one taken (a process cannot take
`gate.lock` exclusive while it holds a share of it: flock treats its
two open files as two owners). For today's workflow that is one
change, shared for ci then exclusive for web, gpu and window: about
408 s shared and 400 s exclusive in gate-603.

The alternative is per step, with the wait time added back to the job's
clock: trunk's 70 s would also overlap, at the cost of a table of 31
step labels and a hold change inside the web job.

### Builders **to confirm**

A gate's shared phase builds on every thread at normal priority
(`priority.rs`). Today at most three ci runs build at once
(`gate_lock_slots.rs`: past three builders, throughput does not rise).
Proposed: the gate's shared phase takes a ci slot too, so at most three
builders of either kind run together. Without it, any number of gates'
shared phases could overlap beside three ci runs.

### Nested commands

Unchanged: a gate step's xtask sees `XTASK_GATE=1` and takes no lock,
since its gate holds what the step needs. A nested acquire by the same
gate therefore never waits on itself. The test for this drives the real
`ci_shared::run` and `test` paths under `XTASK_GATE=1` while another
process holds `gate.lock` exclusive.

### `cargo xtask test`

Takes a share (no slot) after its quiet wait, so a gate's exclusive
phase waits for test runs already going and keeps new ones out. This
retires the declared limit in `quiet_wait.rs`.

### Same worktree

The machine-wide lock today also keeps two gates out of one worktree
(they would share `target/` and `target/gate/github_output`). Proposed:
an exclusive flock on `target/gate/run.lock`, refused at once with the
holder's pid when held.

### Crash recovery

All flocks are freed by the kernel when their process exits. The quiet
lock's holder file is released by `Holder::ended` (pid gone or reused,
via `lock_owner`) or at its deadline, as today. `gate.holder` and
`ci.<pid>` markers are only names, cleared on read when their process
is gone. No new state outlives a process.

### Deadlines

Every wait keeps `gate_lock::WAIT` (40 min, from touche's gate times)
and the quiet lock's `WAIT` (61 min). A shared wait lasts at most one
other gate's exclusive phase (400 s in gate-603) plus the holds queued
before it; an exclusive wait lasts at most the longest shared holder
still running, a ci run of 7 to 23 minutes (`gate_lock_slots.rs`), so
40 min stands.

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

The order when switching from shared to exclusive is: release the
share, take the quiet lock, then the turn and `gate.lock` exclusive.
Taking the quiet lock while holding the share would deadlock two gates
both switching (each waiting on the other's share or quiet hold).

### Not changed

`cargo xtask scaling` takes no lock today and is out of this change;
it could take the exclusive hold in a follow-up. The disk floor (30 GB,
`gate_floor.rs`) was set for one gate at a time, about twice one
gate's growth; two overlapping gates at the floor would leave about
1 GB.

## Slices

1. A gate takes its hold per job from `[gate.shared]` and
   `[gate.exclusive]`, refusing an unlisted job; the whole-run quiet
   and exclusive holds go. Two-process tests: two shared holders
   overlap; an exclusive taker waits for active shares; new shares wait
   while an exclusive taker waits or holds; a dead holder's lock is
   recovered; an old-protocol whole-gate hold is respected; a nested
   xtask in a gate does not wait on its gate.
2. `cargo xtask test` takes a share.
3. The same-worktree guard.
