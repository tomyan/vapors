# Tooltips

An element's `title` attribute names or describes it in the
accessibility tree, as Chromium's tree has it, and `DocumentInput`
works out when its tooltip shows and hides. Nothing draws or places
the tooltip yet (see "Not yet built").

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

## When the tooltip shows

Matched against Chromium's source (2026-11-10): Blink's
`HitTestResult::Title` and `ChromeClient`, and views'
`TooltipController`, `TooltipStateManager` and `TooltipViewAura`, the
desktop tooltips of Linux, Windows and ChromeOS. macOS Chromium uses
the system's tooltips instead.

- **Text.** `Document::tooltip_text(node)`: the title of the node or
  its nearest ancestor with a `title` attribute. `title=""` is present,
  so it stops the walk and nothing shows. The text has whitespace
  trimmed at both ends (the set Rust's `char::is_whitespace` has, which
  includes the no-break space, unlike the accessible name), each tab
  as eight spaces, and its line breaks kept. A title of only whitespace
  shows nothing.
- **Delay.** `TOOLTIP_DELAY`, 500 ms: Windows' default initial tooltip
  delay (`TTM_SETDELAYTIME`) and views' `kDefaultShowTooltipDelay`. The
  wait starts when the pointer reaches a title; moving within the same
  element and text does not restart it, another element or text does.
- **Hiding.** Leaving the window, moving off the title, a press, the
  wheel, any key-down (Esc included), a touch and the window's blur
  hide it, armed or shown. After a press the same title stays hidden
  until the pointer reaches another; after the others the next move
  arms it again. No title arms while the primary button is down.
- **Keyboard.** Focus a key moved (Tab) to an element whose own title
  is non-empty, while it is `:focus-visible`, arms its tooltip with the
  same 500 ms delay, as Chromium's keyboard tooltips do (behind a flag
  there; on here so touché's shortcuts can be found from the keyboard,
  following the APG tooltip pattern). Focus a press gave never arms
  it, even on a text field. It hides on blur, on any key-down and when
  the element loses its title.
- **Touch.** A touch, tap or pan never arms a tooltip and hides one.
- **Changes.** Each frame, after the update, the pointer's tooltip
  follows what is under the pointer: a removed element hides it, a
  changed title hides it and arms the new one.

A host gives `DocumentInput` every event (`give`, the host's own
included) and, once per frame after the document's update, asks
`DocumentInput::tooltip(&document, now)` for the tooltip shown and
`DocumentInput::tooltip_deadline()` for when to wake: it returns
`Next::WaitUntil(deadline)` when it would otherwise wait, and asks again
on every frame until the tooltip shows, because a frame that returns
`Next::Continue` or `Next::Wait` replaces the wait. When the tooltip
hides before its deadline, `tooltip_deadline` is `None` and no frame
is drawn for it. `ElementsApp` does both.

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

Drawing and placing the tooltip: below the element, flipped above when
there is no room, clamped inside the window, clear of the pointer's
hotspot, drawn above every item including gpu-surfaces, styled by
`--tooltip-*` custom properties. A tooltip that hides does not yet ask
for a frame, since nothing is drawn for it.

Chromium's other rules: its tooltip hides itself after 10 s
(`kDefaultHideTooltipDelay`), and text past 2048 characters is cut at a
word break.
