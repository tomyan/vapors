# Tooltips

An element's `title` attribute names or describes it in the
accessibility tree, as Chromium's tree has it. Drawing the title as a
tooltip on hover is not built yet; it follows in later pull requests
(see "Not yet built").

## The accessibility tree

`crates/elements/src/a11y_title.rs` reads `title`. The corpus fixture
`crates/elements-corpus/fixtures/titles.wisp` and its `titles.events`
hold vapors to Chromium 155's tree for each case below.

- **Name.** When `aria-label`, `aria-labelledby` and, for a role named
  by its content (`button`, `checkbox`, `option`), the content give no
  name, the title is the name. Each run of whitespace becomes one space
  and the ends are not trimmed: `title="  Spaced\n out  "` names the
  node `" Spaced out "`.
- **Description.** When something else names the node, the title is
  its description, whitespace collapsed and trimmed. The content of the
  ids in `aria-describedby` replaces it when that content is not empty;
  when no id is found, the title is the description.
- A title of only whitespace, or `title=""`, is neither.

```html
<button title="Open (o)">Open</button>        <!-- name "Open", description "Open (o)" -->
<button title="Undo (Ctrl+Z)"><svg aria-hidden="true">…</svg></button>
                                               <!-- name "Undo (Ctrl+Z)" -->
```

`AxElement::description` and `AxNode::description` carry the
description; the events oracle's projection compares it.

## Not yet built

The tooltip itself: shown after the pointer rests on an element with a
title, hidden on leave, press, scroll or Esc, placed below the element
or above it when there is no room, drawn above every item including
gpu-surfaces, and never shown for touch input.
