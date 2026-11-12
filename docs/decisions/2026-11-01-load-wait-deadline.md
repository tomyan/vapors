# The load wait goes on at its deadline, warning

2026-11-01. Decided by Tom.

`cargo xtask ci` and `cargo xtask gate` wait while the machine's
one-minute load is over `[load] threshold` (`xtask/src/load_wait.rs`).
The question was what happens when `[load] deadline_minutes` passes with
the load still over it: fail the run, or go on.

Tom's answer: "Carry on, warn".

So at the deadline the run goes on and prints
`load: STILL <load>, over <threshold> ... after waiting <s> s; going on
anyway at the deadline ...`, and the final summary repeats that line.
This departs from "fail at the deadline", so it is declared as a relaxed
guard where it runs. The reasons recorded there: the wait protects other
agents' build speed, not this run's result, and a failure would have
callers retry in a loop that adds load. It is retired when runs queue
for CPUs through a scheduler instead of this wait.

The 10-minute default is the wait agents were asked to do by hand
before it was mechanical, not a measurement.
