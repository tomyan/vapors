# Overflow on one axis

`overflow-x` and `overflow-y` take every combination of `visible`,
`hidden`, `clip`, `auto` and `scroll`.

## Computed values

The cascade computes the pair together (`crates/css/src/cascade/overflow.rs`,
CSS Overflow 3 section 3.1): when one axis is neither `visible` nor `clip`,
a `visible` on the other computes to `auto` and a `clip` to `hidden`.
So `overflow-y: auto` alone computes to `auto auto`, and `overflow-x: hidden`
alone to `hidden auto`. The 25 pairs are in `crates/css/tests/cascade_overflow.rs`.

## What each axis does

- A box with an axis that is `auto` or `scroll` is a scroll container. Layout
  sees the computed pair, so a flex item's automatic minimum size is 0.
- Every axis that is not `visible` cuts the content off at the padding box.
- Only an `auto` or `scroll` axis has a scroll range, a thumb, and takes the
  wheel and touch pans. A `hidden` or `clip` axis has range 0, so a wheel or
  pan along it passes to the next container out (as Chromium does for an
  axis it cannot scroll by user input).
- `auto` and `scroll` behave alike: thumbs are overlays that take no space and
  show only when there is something to scroll.

## Refused

`clip` beside `visible` (either order) would clip one axis and leave the other
visible. vapors clips to a padding box, so this is refused, naming the pair.
Fix: give both axes `clip` or `hidden`.

The viewport (html, and body while html is `visible`), text fields and
number fields still refuse `auto` and `scroll` on either axis.

## Not checked against Chromium here

The wheel passing out of a `hidden` axis and the pair computation follow the
specification; the first is Chromium's behaviour as understood, not measured
in this sandbox (no Chromium).
