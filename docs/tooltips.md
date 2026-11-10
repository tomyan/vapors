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
- A title of only ASCII whitespace, or `title=""`, is neither. Other
  whitespace is kept: a no-break space at either end stays in the name
  and in the description, and a title of only no-break spaces is the
  name.
- A whitespace-only `aria-label` names nothing, so the title is the
  name. `aria-describedby` with ids found gives the description even
  when the title is the name.

```html
<button title="Open (o)">Open</button>        <!-- name "Open", description "Open (o)" -->
<button title="Undo (Ctrl+Z)"><svg aria-hidden="true">…</svg></button>
                                               <!-- name "Undo (Ctrl+Z)" -->
```

`AxElement::description` and `AxNode::description` carry the
description; the events oracle's projection compares it.

## Where the tooltip goes

`vapors_app::place_tooltip(trigger, size, window) -> Result<Point, _>`
returns the tooltip's top-left corner in window CSS pixels, given the
tooltip's measured size and the window's. It draws nothing.

Pointer trigger. Chromium's `TooltipAura::GetTooltipBounds`
(`ui/views/corewm/tooltip_aura.cc`, read 2026-11-10) places a cursor
tooltip at the hotspot plus (`kCursorOffsetX`, `kCursorOffsetY`) =
(10, 15), not below the element. Then it shifts the rectangle
horizontally to fit the display, and when the bottom is past the display
it sets the top to the hotspot's y minus the tooltip's height (above
the cursor). vapors does the same steps, in CSS pixels. Differences:
Chromium mirrors the x offset in right-to-left locales (not built), and
vapors adds a fallback for a window too short for above or below: beside
the hotspot, right first and then left. With no side that fits, the
tooltip is placed below and clamped, and may cover the hotspot.

Keyboard trigger (no cursor). Chromium centres the tooltip on an anchor
point with its top on it. vapors takes the focused element's border box
instead: centred horizontally under it with a 4 px gap (`KEYBOARD_GAP`,
a choice, not Chromium's), above it with the same gap when it does not
fit below. When it fits neither, it is placed below and clamped.

Both: the result is slid inside the window. The hotspot is not covered:
the below position starts 15 px under it, the above position ends on
the hotspot's row (the bottom edge is outside the tooltip), and the
beside positions start at or end on its column. A tooltip longer than
the window on a side starts at 0 on that side and overflows the other
end. Chromium's `AdjustToFit` shrinks it instead; vapors' painter
decides whether to wrap or clip it. A NaN, an infinite value or a
negative size is refused with the input's name (`PlacementError`).

## Not yet built

Title on images. Chromium 155 (2026-11-10, read with the events oracle's
`project image`) has each of these as an `image`. No corpus fixture
holds them, because the corpus requires vapors to accept every fixture
and compare its boxes: the icon subset refuses `title` on an `svg`
(and an `svg` with `role="img"` and no `aria-label`), and `<img>` has no
role in vapors' tree. Adding them needs the subset to take `title` and
`img` to get a role, name and description:

| Markup | name | description |
|---|---|---|
| `<svg role="img" title="T">` | `T` | none |
| `<svg role="img" aria-label="L" title="T">` | `L` | `T` |
| `<img alt="" title="T">` | empty | `T` |
| `<img title="T">` | `T` | none |
| `<img alt="A" title="T">` | `A` | `T` |

The tooltip itself: shown after the pointer rests on an element with a
title, hidden on leave, press, scroll or Esc, drawn above every item including
gpu-surfaces, and never shown for touch input.
