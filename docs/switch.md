# The switch

`<input type="checkbox" switch>`: the HTML `switch` attribute (Safari
and Chromium implement it; not checked here). `docs/` named no other
form (`docs/design/03-events-focus-input-ime.md` has only the role).

## What it is

A checkbox (`crates/elements/src/checkbox.rs`) drawn by
`crates/elements/src/switch.rs` as a rounded track and a round knob.
Everything but the look is the checkbox's: a click or tap toggles it
(`input` then `change` with `checked`), Space toggles it on keyup, Enter
does not, `disabled` takes neither, `bind:checked` works, `:checked`
selects it and `:focus-visible` rings it (a rectangle).

## Look

| Part | Value |
|---|---|
| size | `width: 2.25em; height: 1.25em` (`crates/css/src/ua.css`), so it scales with `font-size`; an author's `width` and `height` win |
| track | the content box, fully rounded; `accent-color` when checked, the user agent's blue `#0060df` for `auto`; `--vapors-track-color` (default `#d0d0d7`) when not |
| knob | white circle, as tall as the track less an inset of 1/11 of its height on each side; at the start unchecked, the end checked |

`accent-color` is a new property: `auto | <color>`, inherited (CSS UI 4).
Only a switch reads it.

The numbers 2.25em by 1.25em are the brief's: not checked against
Chromium or Safari (no network or Chromium in the sandbox), nor the
colours. The 1/11 inset is chosen so a 40 by 22 px switch has the
gui-bench design's 2 px inset and 18 px knob.

Accessibility: role `switch` with the `checked` state; an explicit
`role` wins.

## Refusals

`switch` on an input that is not a checkbox (`text`, `range`, `number`,
or none) is refused at update, naming the type. In a `.wisp`, `switch`
is a boolean attribute, written `switch=""`; `switch="false"` and
`switch={expr}` are refused by the compiler, as `disabled` is.

## Left

- A knob shadow, a rounded ring, motion, `::part()`, a Chromium oracle
  fixture for the sizes. A knob with a shadow can be drawn from CSS
  instead: `appearance: none` takes the track and knob away
  (`docs/appearance.md`), and a decorative sibling is the knob.
- An absolutely placed switch needs `display: block`: the user-agent
  `inline-block` input is laid out only as a flex or grid item.
