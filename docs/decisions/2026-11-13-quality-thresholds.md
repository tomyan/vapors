# Quality checks at the tail, from a measurement

2026-11-13. Decided by the lead (Tom delegated the design on
2026-11-13); this note records the measurement the numbers come from.

## What was wrong

The checks were proxies that could be gamed and that forced work
without judging it: a hard file cap of 220 lines (`[files] max_lines`),
clippy's `too_many_lines` at its default 100, no complexity lint, no
signal for reaching through other objects. Seen: a merge pushing a
file to 223 lines forced a split (`update_counts.rs` ->
`update_counts_read.rs`); an agent met the cap with `#[rustfmt::skip]`;
many small files meant more merge conflicts.

## The shape now

Each metric has a guideline (the rule as written), a hard limit at the
tail of main's distribution, and a band between them that review
judges (`docs/review.md`). The hard limits are checks in `lint` and
`ci`. `cargo xtask quality` prints the band's worst (what is over a
hard limit is not listed: it is allowed at its site or failing); the
nightly run (`.github/workflows/nightly.yml`) prints it and a reviewer
runs it by hand; it is not a step of `lint` or `ci` and never fails on
a finding. `cargo xtask quality --distributions` runs the measurement
below again (clippy's counts, the file-cap fit, the agreement) and
`--list` prints every sample.

| metric | guideline (`[quality]`) | hard limit | where |
|---|---|---|---|
| lines in a file | 220 | 320 | `[files] max_lines` |
| lines of code in a function body | 15 | 50 | clippy.toml `too-many-lines-threshold` |
| cognitive complexity | 6 | 12 | clippy.toml `cognitive-complexity-threshold`, lint on in Cargo.toml |
| field hops in a chain | 3 | none | signals only |

## The measurement, main at adb3b677

Percentiles are by nearest rank. Function lines and complexity are
clippy's own counts, collected by running clippy with both thresholds
at 1 (`CLIPPY_CONF_DIR` pointing at a `clippy.toml` with
`too-many-lines-threshold = 1` and `cognitive-complexity-threshold = 1`,
`cargo clippy --workspace --all-targets --no-deps --message-format=json
-- -W clippy::cognitive_complexity -W clippy::too_many_lines`), which
is what `cargo xtask quality --distributions` now runs
(`xtask/src/quality_clippy.rs`), joined to every function xtask finds
(`quality_join.rs`): a function clippy did not report counts as 1, so
the command's n is every function the host run compiled, where the
first measurement's n for complexity below counted only the functions
clippy reported (those over 1); its medians and p90s are lower by
that. Test code is files named `*_tests.rs` or `tests.rs` or under
`tests/`.

Lines per `.rs` file (code 949 files, tests 863):

| | median | p90 | p95 | p99 | max |
|---|---|---|---|---|---|
| code | 104 | 193 | 209 | 219 | 220 |
| tests | 98 | 186 | 201 | 216 | 220 |

Six code files sit at exactly 220 and 5% of them within 10 lines of
it: the distribution is cut off by the old cap, so its p99 is the cap.
The hard limit is read from the part the cap did not squash: a
lognormal through the median and p90 (`fitted_p99` in
`xtask/src/quality_stats.rs`, printed by `--distributions`) gives sigma
0.48 for code and 0.50 for tests, and p99 of 319 (code) and 313
(tests). 320 for both; the two distributions do not differ enough for
two caps.

Lines of code in a function body (clippy's count: the body between
its braces, blank and comment-only lines left out):

| | n | median | p90 | p95 | p99 | max |
|---|---|---|---|---|---|---|
| code | 4667 | 8 | 19 | 24 | 37 | 80 |
| tests | 5068 | 7 | 17 | 22 | 33 | 99 |

At 40 (about p99), 44 functions on main are over (26 code, 18 tests);
at 45, 20; at 50, 14. The limit is 50 so each existing function over
it carries an allow at its site with a reason naming the one concept
it holds, and that list stays short enough to read: the two demo lists,
the corpus's action and node-kind dispatches, three fixture page
builders, the perf and css_dump command sequences, five one-table or
one-scenario tests. The ten worst:

```
 80  crates/demos/src/later.rs:8                        slices
 68  crates/demos/src/lib.rs:82                         all
 61  crates/elements-corpus/src/events_vapors_act.rs:55 act
 59  crates/perf/src/main.rs:29                         run
 58  crates/elements-corpus/src/events_vapors.rs:55     play
 56  crates/compiler/src/codegen/blocks.rs:148          collect
 53  crates/elements/src/layout/boxes.rs:59             add_element
 51  crates/css/examples/css_dump.rs:28                 run
 50  xtask/src/main.rs:143                              main
 50  crates/racer/src/gpu/pipeline.rs:2
 99  crates/elements/tests/support/changes.rs:60        change (tests)
 78  crates/elements/tests/support/roles_page.rs:19     page (tests)
```

`AxError::fmt` (crates/elements/src/a11y_error_message.rs:10) is 124
lines and already allowed at its site, so clippy did not report it.

Cognitive complexity (clippy's count):

| | n | median | p90 | p95 | p99 | max |
|---|---|---|---|---|---|---|
| code | 2639 | 2 | 4 | 5 | 6 | 12 |
| tests | 4000 | 3 | 4 | 5 | 7 | 12 |

(n counts the functions clippy reported, those over 1.) The scale is
too coarse for a limit at p99 to leave a band: at 6 or 7 about 70
functions would need allows. The hard limit is main's max, 12, so
nothing on main is over it and anything new over it is novel; the
guideline is the p99, 6. The four over 10: `directive`
(elements-corpus events_script.rs:160, 12), a css parse test
(css/tests/parse_rules.rs:137, 12), `add_element` (layout/boxes.rs:59,
11), `collect_repeats` (css/syntax/matcher.rs:97, 11).

Field chains in code (xtask's count, 9720 chains): median 1, p90 1,
p99 2, max 4. Seven chains of 4 hops, all `self.run.race.progress.*`
in crates/racer; 3-hop chains are mostly `self.ax.counts.*` in the
a11y mirror and `painted.from.placement.*`.

## xtask's counts against clippy's

The signals are printed by xtask, which reads the source with syn and
reproduces clippy's two counts so the band and the hard limit measure
the same thing. Two things hold them together:

- `cargo xtask quality --distributions` joins xtask's count of every
  function to clippy's and fails under the floors in
  `xtask/src/quality_clippy.rs` (lines 99.9%, complexity 97.0%), so a
  toolchain bump that moves clippy's rules is found when it next runs.
  On main at adb3b677, compared by hand the same way: lines agree on
  9349 of 9353 functions (99.96%; the four differ by 2, all `async
  fn`s, whose body clippy reads from the desugared coroutine),
  complexity on 10667 of 10936 (97.5%); the rest are third-party
  macros whose expansions hold branches (`tracing::trace_span!`),
  `return`s inside closures, and two return-type aliases of `Result`.
  A count allowed at its site is not compared, since clippy says
  nothing about it; what the host run did not compile (a `#[cfg]` for
  another target or a feature, a test in an example) is left out and
  its number printed. The figures the command printed on this branch
  are in the pull request.
- A fixture of the constructs the count models,
  `xtask/fixtures/quality_probe.rs.txt`, with clippy's counts for each
  function in `quality_probe.clippy.tsv` from
  `scripts/quality-probe-clippy.sh`; the unit test
  `every_fixture_function_counts_as_clippy_counted_it` holds xtask to
  them exactly. That is where `#[test]` functions counting,
  `let ... else` not being a branch, closures being separate bodies and
  macro arguments being walked were confirmed. The declared gaps are in
  `xtask/src/quality_complexity.rs`; the hard check is clippy's own.

## Retiring this

When the band's worst offenders are gone from the signals for a month,
the hard limits can come down toward the guidelines, measured again
with `cargo xtask quality --distributions`; the same command says when
a limit has become the cap that shapes the distribution, as 220 did.
On a toolchain or clippy bump: run `--distributions` and
`scripts/quality-probe-clippy.sh`; a drift fails the first and shows
in the second's diff.
