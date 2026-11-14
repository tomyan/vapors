# Reviewing a pull request

What the review agent in step 4 of the slice loop (CLAUDE.md) judges,
beyond the "Failure must be loud" rules. The checks measure the tail
(`cargo xtask lint` and `ci`: clippy's `too_many_lines` and
`cognitive_complexity` at clippy.toml's thresholds, `[files] max_lines`);
the band below each hard limit is for the reviewer, and `cargo xtask
quality` (run before a review, and nightly) lists the worst of it so
the reviewer does not count; what is over a hard limit is not listed,
since it is allowed at its site or failing. The numbers and where they came from:
`docs/decisions/2026-11-13-quality-thresholds.md`.

## Single responsibility

- A file over 220 lines holds one concept, and its module doc comment
  names it. A doc comment that names two things with "and", or lists
  the file's contents, is a finding: name the split.
- A function does one thing, usually in 10 to 15 lines. A function in
  the band (over 15 lines, within 50) is read for what it does: one
  thing done in sequence, or a table of literals, passes; two things
  (parse and act, build and check) is a finding.
- A function over 50 lines or over complexity 12 is allowed at its site
  with a comment naming the one concept it holds. The reviewer checks
  the comment is true, not that it exists.

## Reaching through other objects' internals

- `a.b.c.d` reads through `a` and `b` to get at `d`; the caller now
  depends on the shape of three types. Two hops (`self.a.b`) is
  ordinary; three or more is a finding unless the chain is plain data
  (a record of records with no behaviour). The fix is a method on the
  nearest type that answers the question.
- Method chains (iterators, builders) are not this: the signal counts
  field steps only.

## Splits made to pass a number

- A file split only to get under a limit is a finding, whatever the
  line counts say: two files whose doc comments cannot name different
  concepts, a `foo.rs` and `foo_more.rs` or `foo_read.rs`, a function
  cut at line 15 into `do_a` and `do_a_rest`. Ask for the one split that
  names two concepts, or for the file back together under the hard
  limit.
- `#[rustfmt::skip]`, a widened line, or code moved into a macro to
  shorten a count is the same finding.

## How to report

Each finding names the file and line, what it reaches through or
holds twice, and the fix the reviewer would accept. Findings are
verified before action, as the slice loop says.
