# The gate runs its steps with /dev/dri hidden

Date: 2026-11-02. Decided by: the lead (asked for after touche's walk
passed in the local gate and failed on GitHub).

## Decision

`cargo xtask gate` runs every step through `bwrap --dev-bind / /
--tmpfs /dev/dri --die-with-parent` (`xtask/src/gate_sandbox.rs`), so
the gate sees no DRM device, as GitHub's runners have none. Without
one, headless Chromium could not present a WebGPU canvas, so a check
passed here and failed there (touche runs 41316945392 to 41337806803).
The gate refuses to start when bwrap is missing or does not leave
`/dev/dri` empty, naming the fix. To run a step by hand outside the
sandbox, drop the six arguments after `bwrap` from its reproduce line.

## What it gives up

The local gate no longer exercises a real GPU device at all. Its gpu,
screenshot and oracle steps were already pinned to llvmpipe by
`VAPORS_GPU_ADAPTER: llvmpipe` in `ci.yml`; the web job's headless
Chromium was the only step that used the dev box's Intel GPU, and that
was what hid the runner's failure. What still runs on real hardware:
an implementer's `cargo xtask ci`, whose gpu tier takes the first
hardware adapter (the dev box's Intel UHD 630) when
`VAPORS_GPU_ADAPTER` is unset. Nothing in the gate or in CI does.

## The off switch

`[gate] hide_gpu_device = "false"` in `.config/factory.toml` runs the
steps as they are, for a machine with no bwrap or no `/dev/dri` (the
planned Mac Studio). It is a relaxed guard, declared where it is read:
every such gate prints that `/dev/dri` is not hidden and that a step
can pass there that fails on GitHub. Retired when that machine has its
own way to hide its GPU from the checks.

## Supersedes

Nothing.
