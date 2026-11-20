# `<input placeholder>`

A text field or number field with no value paints its `placeholder`.

```html
<input type="text" placeholder="Search customers" />
```

## What is painted

- The text of the attribute with line feeds and carriage returns removed
  (as Chromium's `StrippedPlaceholder` does; an input has one line). A
  placeholder of only line breaks is empty and paints nothing.
- In the input's font (family, size, weight, style, letter spacing) and in
  Chromium's placeholder colour, `#757575` (`getComputedStyle(input,
  '::placeholder').color` reads `rgb(117, 117, 117)` in the Chromium in
  this repository's sandbox; its html.css has `-internal-light-dark(#757575,
  #A9A9A9)`, and vapors has the light scheme only). The colour is the
  constant `placeholder::COLOR`.
- Left-aligned at the left of the content box, on one line, centred in the
  content box's height as Chromium's `LayoutTextControlSingleLine` centres
  the inner editor (also when the line is taller than the box), and cut to
  the content box. This holds when the input's own `overflow` is `visible`.
- Not while the field is `visibility: hidden`.
- Only while the field's text is empty. A value typed, set by a signal
  (`bind:value`) or given by the `value` attribute hides it at once; an
  emptied value shows it again. `placeholder` can be set at run time and
  follows.

It is not a hit, a caret position or a selection: a press on it hits the
input, the value stays empty, and the caret is at 0.

## Which inputs

A `placeholder` on an input of a type that takes none (`range`, `checkbox`)
is refused, naming the type, where Chromium ignores it. Text and number
fields take it (HTML 4.10.5.1: text, search, url, tel, email, password and
number; vapors builds text and number).

## Accessibility

The placeholder is the field's accessible description, not its name:
`aria-label`, the label and `title` name it. `aria-describedby` comes before
the placeholder; the placeholder stays the description once the field has a
value. A number field (role `spinbutton`) is described the same way. A field
with only a placeholder has an empty name and the placeholder as description.
(Chromium also falls back to the placeholder as the name of an otherwise
unnamed field, and then reports no description; vapors keeps name and
description apart, so an unnamed field should carry an `aria-label`.)

## Left

- `::placeholder { color }` in a component's sheet: vapors-css refuses every
  pseudo-element. It needs the parser, a matcher, `Cascade::compute_pseudo`,
  an `input::placeholder` user-agent rule in place of the constant, and the
  colour kept at each restyle so `input:focus::placeholder` repaints it.
- `:placeholder-shown`; a placeholder on search, url, tel, email and
  password inputs (the types are not built); a right-to-left or wrapping
  placeholder.
- Vertical centring of the typed text: the field lays it out at the top of
  its content box (the placeholder is centred as Chromium's is).
