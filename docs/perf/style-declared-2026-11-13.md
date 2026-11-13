# Style slice B: declared longhands and the grid readers (2026-11-13, main at 7c4fc70d)

The two `Used` adds from the per-element profile of the rows Insert
frame (`docs/perf/style-s1-2026-11-13.md`, "Why"): of the `Used`
derivation's about 15k instructions per restyled element, 48% was
`check_honoured` (`crates/elements/src/style/honoured.rs`), which
walked every longhand of the table per element, and 13% the grid
readers (`grid_style.rs`), which built a one-track `Vec` for the
initial `grid-auto-rows: auto` and serialised `grid-auto-flow`'s
keywords to match them.

## The change

- vapors-css keeps per computed style which longhands a declaration
  reached (`Declared`, `crates/css/src/cascade/declared.rs`): a winner
  on the element, or, for an inherited property, a declaration on an
  ancestor, as one bit per longhand (`u128`; a table longer than 128 is
  refused at the first restyle, naming the fix). `ComputedStyle::
  declared(id)` and `declared_ids()` read it.
- `check_honoured` visits `declared_ids()`, about ten per element,
  and looks each treatment up by index, instead of walking the table
  and comparing every deferred longhand's value with its initial. A
  longhand no declaration reached computes to its initial value, which
  debug builds assert for every deferred or off longhand
  (`assert_undeclared_at_initial`), naming the property.
- `auto_tracks` returns the empty list for `auto`, which is what Taffy
  repeats when the list is empty (its default; `explicit_grid.rs`
  `auto_track_count == 0`), and `auto_flow` matches the keywords
  without serialising them.

## Measured

`vapors-perf 60 10`, release, Intel UHD 630, load 30 to 70
(instruction counts do not move with load). Before is main at
00dbc38b with #279 (`docs/perf/style-s1-2026-11-13.md`'s baseline);
after is this branch on main at 7c4fc70d, without #279 or #299, so
its Insert frames carry the body check's 45 instructions per row and
none of S1's saving.

| workload | phase | before | after |
|---|---|---|---|
| 3000 rows, Class (`--instructions`) | style | 600,030 | 582,235 (−18k, −3%) |
| `--scaling Class`, add frames | style | 599,679 / 601,084 / 601,827 | 579,179 / 582,218 / 582,232 (flat; the run passes since #288) |
| `--scaling Insert`, insert frames | style | 548,998 / 556,408 / 558,109 | 531,634 / 538,625 / 535,424 at 1000 rows −17k |

Per restyled element, from the Class edit's four: about 4.4k
instructions less. Sampled (perf, `instructions:u`, call chains under
`style::used`), `check_honoured`'s share of `used` went from 42% to
9% and `grid` from 13% to about 8% of what remains; `used` is now
about half `box_style`, a sixth `decoration`.

## Checks

- `crates/css/tests/cascade_declared.rs`: a declared longhand is
  declared and an undeclared one is not; an inherited property
  declared on the parent is declared on the child, and three ancestors
  up; a non-inherited one declared on the parent is not; `inherit`
  declares it; a `var()` winner declares it.
- `crates/elements/tests/honoured_declared.rs`: a deferred property
  declared on an element is refused naming it, once; an element under
  it is not refused for it; a page without deferred declarations
  updates.
- The rule `check_honoured` relies on, proven by breaking: with
  `Declared::of` ignoring winners, both refusal tests fail at
  `assert_undeclared_at_initial`: "animation-name: no declaration
  reached it, yet its computed value `spin` is not the initial `none`".
  Restored.
- `grid_style_tests.rs`: `auto_flow` reads each form and refuses an
  unknown one naming it; `auto_tracks` of `auto` is the empty list,
  of one size one track.
- Every css and elements test passes with the assertions on.
