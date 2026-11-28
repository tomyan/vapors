# `<input placeholder>`

A text field or number field with no value paints its `placeholder`.

```html
<input type="text" placeholder="Search customers" />
```

## What is painted

- The text of the attribute with line feeds and carriage returns removed
  (as Chromium's `StrippedPlaceholder` does; an input has one line). A
  placeholder of only line breaks is empty and paints nothing.
- In the input's font and in its placeholder colour (below).
- Left-aligned at the left of the content box, on one line, centred in the
  content box's height as Chromium's `LayoutTextControlSingleLine` centres
  the inner editor (also when the line is taller than the box), and cut to
  the content box, also when the input's `overflow` is `visible`; not while
  `visibility: hidden`.
- Only while the field's text is empty. A value typed, set by a signal
  (`bind:value`) or given by the `value` attribute hides it at once; an
  emptied value shows it again. `placeholder` can be set and changed at run
  time and follows (the tree cannot remove an attribute yet; an empty one
  paints nothing).

It is not a hit or a selection: a press on it hits the input.

## `::placeholder` colour

```css
.search::placeholder { color: #6b7280; }
```

`::placeholder` is taken only at the end of a selector (`input::placeholder`,
`form ::placeholder`, `form > ::placeholder`), and only with `color`. The
parser turns the rule into one on the input that sets
`-vapors-placeholder-color` (initial `#757575`), so the cascade treats it as
any property's value. Refused, naming them: another property in the rule;
`inherit`, `initial` or `unset`, also from a `var()`; `::placeholder`
anywhere else in a selector; the longhand written by name, also in
`transition-property`.

Unstyled, a placeholder is `#757575`, opaque, whatever the input's own
`color`: the pinned Chromium's (157.0.8103.0) computed `::placeholder`
colour and opacity, saved in
`crates/elements/tests/placeholder_chromium/computed.txt` (regenerate with
`scripts/placeholder-chromium.sh` when the pin changes; the tests read the
saved file and run no browser, and fail when it names another Chromium than
the pin). vapors has the light scheme only. A class set at run time
recolours it. The text typed into the field keeps the input's `color`.

## Which inputs

A `placeholder` on a `range` or `checkbox` is refused, naming the type
(Chromium ignores it). Text and number fields take it.

## Accessibility

The placeholder is the field's accessible description, not its name
(`aria-label`, the label and `title` name it). `aria-describedby` comes
before it; it stays the description once the field has a value. A `title`
that is the name leaves it the description; a `title` that is itself the
description wins over it. A field with only a placeholder has an empty name
(Chromium falls back to the placeholder as name; vapors keeps the two apart,
so give the field an `aria-label`).

## Left

- Placeholder styles other than its colour (`opacity`, fonts and the rest),
  and `:placeholder-shown`; search, url, tel, email and password inputs (not
  built); a right-to-left or wrapping placeholder; vertical centring of the
  typed text.
