# appearance

`appearance: none | auto` (CSS UI 4), not inherited. The
initial value is `none`; the user-agent sheet (`crates/css/src/ua.css`)
sets `auto` on the controls vapors draws parts of: a range, a checkbox
and a switch. Read by `crates/elements/src/style/appearance.rs`.

## What `none` does

| Control | `auto` | `none` |
|---|---|---|
| checkbox | its mark when checked (`checkbox_paint.rs`); 11 x 11 px content when unsized (`checkbox.rs`) | no mark; no content when unsized |
| switch | its track and knob (`switch.rs`) | neither: it draws only the box's own background, border, radius and outline |
| range | its track and thumb (`range_paint.rs`) | refused (below) |
| any other element | nothing of its own | nothing of its own |

Behaviour does not change: a click and Space toggle a checkbox or
switch, `:checked` selects it, it takes focus and `:focus-visible`, and
its role, name and checked state are the same.

A switch with `none` is styled from ordinary CSS: a background and
radius on the input for the track, `:checked` for the on colour, and a
decorative sibling for the knob (`input + span`), with `box-shadow` for
its shadow and `pointer-events: none` so a press on it reaches the
input.

## Checked against Chromium

Chromium 157.0.8103.0 (the pinned binary, `.config/tools.toml`),
measured 2026-11-26 with `getBoundingClientRect` and
`getComputedStyle`: a checkbox with `appearance: none` is 0 x 0 with its
3px 3px 3px 4px margins, no border and a transparent background,
`inline-block`, checked or not; without it, 13 x 13 and `appearance:
auto`; `type=Checkbox` matches `[type=checkbox]`, and it and
`type=RANGE` compute `auto` (measured 2026-11-27), as HTML's
case-insensitive `type` has it. `fixtures/appearance.wisp` holds the
boxes, roles, names and states to Chromium (`layout_oracle`,
`ax_oracle`).

Chromium draws a checkbox's look in its theme, so `none` takes all of
it away. vapors' look is the user-agent sheet's border, background and
radius (`ua.css`), which cascade as any declaration does, so `none`
leaves them: an author who wants none of it sets `border: 0` and a
background, as the fixture does. A switch has no border or background
in the user-agent sheet, so there it shows only as its 2.25em by 1.25em
size, where Chromium, which has no switch, would have 0 x 0.

## Refused

- `none` on a range: Chromium takes the track away and still draws the
  thumb, whose appearance is its own; vapors draws both as one part.
  Remove it, or style the range through its `--vapors-*` parts.
- Every keyword but `none` and `auto`, by the grammar: CSS UI 4's
  `<compat-special>` (`textfield`, `menulist-button`) and
  `<compat-auto>` (`checkbox`, `radio` and the rest), and the draft's
  `base` and `base-select`.
- `-webkit-appearance` and `-moz-appearance`, refused naming
  `appearance` to write instead.

`type` is matched without regard to case (`input[type=checkbox i]` in
the user-agent sheet), as HTML and Chromium 157 do: `type=Checkbox` is
a checkbox with `appearance: auto`. The older user-agent rules for a
checkbox's border and background, a switch's size and a range's margin
still match `type` with regard to case, so `type=Checkbox` draws its
mark without that border; not changed here.
