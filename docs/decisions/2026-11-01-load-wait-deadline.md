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

## Amended 2026-11-13: what the wait reads

The wait now reads the count of tasks runnable now (`/proc/loadavg`'s
fourth field, `runnable/total`; the count includes the reading
process), every 5 s, and waits while the mean of the last six readings
(30 s) is over the threshold, instead of the one-minute load average
every 30 s. It runs only once six readings are in, so every run waits
at least 25 s. It still says what it waits on every 30 s, and the
threshold and the deadline above are unchanged. The mean of six, not
one reading or three in a row, is the lead's choice on 2026-11-13 from
a sampling recorded at `WINDOW` in xtask/src/load_wait.rs: with ten
builders running, three low readings in a row came in 10.6 % of
windows, the mean of six never. The one-minute average decays for 90 to
120 s after the work has gone (gate 271), so the gate waited on work
that had finished; the runnable count over the thread count says the
CPUs are taken now (docs/plans/factory-throughput.md C3). The gate's
per-step load lines still print the one-minute average.
