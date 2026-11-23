# Saved Chromium references

The paint tests (`crates/elements-corpus/tests/gpu_paint.rs` and
`gpu_paint_rules.rs`) hold vapors' rendering of each paint fixture to
Chromium's. They used to screenshot every fixture with headless Chromium
in each run. On the hosted runner, the same Chromium 157 draws
`text_line_box_paint`'s glyph rows one device pixel off from this
machine (runs 45067990897 and 45072756560), so the result depended on
where it ran. Tom decided on 2026-11-22 that ordinary checks compare
against committed Chromium screenshots, and on 2026-11-23 that:

- an ordinary run renders vapors and compares it with the committed PNG,
  and queries nothing about the browser or the reference's freshness
  (no version, checksum, flags or manifest);
- a missing or unreadable reference fails, naming the file and the
  capture command;
- references are captured with Chromium by hand, only to investigate a
  mismatch when there is reason to suspect the reference, and looked at
  before a replacement is committed. Otherwise a mismatch is fixed in
  vapors;
- every reference the paint suite uses is saved: each fixture, scale
  and changed state.

## What is saved

`crates/elements-corpus/fixtures/references/` holds 50 PNGs, each as
Chromium wrote it:

- `<fixture>_<scale>x.png`: the 11 paint fixtures (a `/* paint` comment
  in the style) at 1x and 2x, 22 files. `gpu_paint` compares vapors with
  them: pixel rules for every fixture at 1x and for all but
  `text_line_box_paint` at 2x, ink placement and the tripwire for
  `text_line_box_paint` at 2x;
- `<fixture>_<state>_<scale>x.png`: the 14 changed states in
  `paint_reference_states.rs` `STATES` (CSS appended to `paint_boxes` or
  `icons_paint`, as a vapors bug would change them) at 1x and 2x, 28
  files. `gpu_paint_rules` holds that each fails the rules against its
  fixture as it is.

The first set was captured on 2026-11-23 with the pinned Chromium
`157.0.8103.0-r1717344` (binary sha256 `c620e149…`, `.config/tools.toml`)
on Debian 13. `text_line_box_paint_2x.png` is byte for byte the file
captured for R1 on 2026-11-22.

## Checks

- `paint_reference::read` refuses a missing file, one that is not a
  readable PNG, and one that is not the window's size at its scale.
  Each refusal names the file and its capture command.
- `the_committed_references_are_every_one_the_paint_tests_read` holds
  the directory to exactly the references the tests read: none missing,
  none unread.
- `gpu_paint_rules_every_state_is_tested_at_every_scale` holds every
  state to a test at each scale.
- The two shift tests in `gpu_paint` move the saved
  `text_line_box_paint` 2x reference one device row down, or one column
  right. They hold that the placement check refuses each, after
  checking that it passes unshifted.

## Capture

`cargo xtask reference all` or `cargo xtask reference <fixture> <scale>
[<state>]` runs the corpus example `paint_reference`. It finds Chromium
as every comparison does (`Chromium::find`: `VAPORS_CHROMIUM`, else the
pinned install), loads the fixture's page with the state's CSS appended,
and copies the screenshot into the references directory. Nothing records
which browser drew a file; the commit that adds or replaces it says so.

## Not covered

- `gpu_events` still drives a live Chromium. It compares events, not
  paint, and the layout, a11y and events oracles do the same.
- The live comparison is gone from the paint tests. The withdrawn hosted
  diagnostic (branch `diag/chromium-text-placement`, no PR) is separate.
