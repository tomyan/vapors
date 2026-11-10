# Tooltips

An element's `title` attribute names or describes it in the
accessibility tree, as Chromium's tree has it, and `DocumentInput`
works out when its tooltip shows and hides. `place_tooltip` works out
where it goes. Nothing draws it yet (see "Not yet built").

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

Matched against Chromium's source (main, read 2026-11-10): the desktop
tooltips of Linux, Windows and ChromeOS. macOS Chromium uses the
system's tooltips instead. Paths are under `chromium/src`; views'
files are in `ui/views/corewm/`.

| Behaviour | vapors | Chromium source |
|---|---|---|
| The title of the node or its nearest ancestor with a `title` attribute; `title=""` is present, so it stops the walk and nothing shows | `Document::tooltip_text` | `third_party/blink/renderer/core/layout/hit_test_result.cc`, `HitTestResult::Title` (`!title.IsNull()`) |
| Whitespace trimmed at both ends (Rust's `char::is_whitespace` set, the no-break space included); only whitespace shows nothing | `tooltip_text.rs`, `shown` | `tooltip_state_manager.cc`, `TooltipStateManager::Show` (`base::TrimWhitespace`, `TRIM_ALL`) |
| Each tab as eight spaces; line breaks kept | `tooltip_text.rs`, `shown` | `tooltip_view_aura.cc`, `TooltipViewAura::SetText` (`base::ReplaceChars`) |
| Shows 500 ms after the pointer reaches a title; a move within the same element and text does not restart the wait | `TOOLTIP_DELAY`, `TooltipState::pointer_over` | `tooltip_controller.cc`, `kDefaultShowTooltipDelay`; `IsTooltipTextUpdateNeeded` |
| A move within the title the pointer was over keeps a keyboard tooltip armed or shown | `TooltipState::pointer_over` | `tooltip_controller.cc`, `OnMouseEvent` (`kMouseMoved`) calls `UpdateIfRequired(kCursor)`, which shows again only when the window's text or id differs from the tooltip's (`IsTooltipTextUpdateNeeded`, `IsTooltipIdUpdateNeeded`); a move that does not change the title under the pointer sends no new text, so the keyboard tooltip stays |
| Hides on any key-down, Esc included | `DocumentInput::tooltip_event` | `tooltip_controller.cc`, `TooltipController::OnKeyEvent` |
| Hides on a press, and the same title stays hidden until the pointer reaches another | `TooltipState::pressed` | `tooltip_controller.cc`, `OnMouseEvent` (`kMousePressed`), `ShouldHideBecauseMouseWasOncePressed` |
| Nothing arms while the button is down, and a move then keeps the pressed title hidden after the release | `TooltipState::pointer_over` | `tooltip_controller.cc`, `OnMouseEvent` (`IsMouseButtonDown` leaves the target `nullptr`; `SetObservedWindow(nullptr)` hides, and only `UpdateIfRequired` clears `tooltip_window_at_mouse_press_tracker_`) |
| Hides on the wheel | `DocumentInput::tooltip_event` | `tooltip_controller.cc`, `OnMouseEvent` (`kMousewheel`) |
| A touch never arms one and hides one | `DocumentInput::tooltip_event` | `tooltip_controller.cc`, `TooltipController::OnTouchEvent` |
| Hides when the window loses focus | `DocumentInput::tooltip_event` | `tooltip_controller.cc`, `TooltipController::OnWindowActivated` |
| Hides on leaving the window or moving off the title | `TooltipState::left`, `pointer_over` | `tooltip_controller.cc`, `OnMouseEvent` (`kMouseExited`); Blink sends an empty tooltip off a title (`core/page/chrome_client.cc`, `UpdateTooltipUnderCursor`) |
| Keyboard focus a key moved, `:focus-visible`, shows the element's own title after the same 500 ms | `TooltipState::focus` | `core/page/chrome_client.cc`, `ElementFocusedFromKeypress` (`element->title()`); `tooltip_controller.cc`, `UpdateIfRequired` (`GetShowTooltipDelay` for both triggers). Behind a flag in Chromium; on here so touché's shortcuts can be found from the keyboard |

Focus a press gave never arms a tooltip, even on a text field (which
a press makes `:focus-visible`). A keyboard tooltip hides on blur,
including a script's, and when its element loses its title or its box
(`display: none`). Each
frame, after the update, the pointer's tooltip follows what is under
the pointer: a removed element hides it, a changed title hides it and
arms the new one.

## For apps not using ElementsApp

`ElementsApp` does this itself. An app that drives its own frames
(touché's page, `crates/web/src/page_next.rs`) must:

1. Give `DocumentInput` every event, the host's own included (`give`
   does), so a key or press the host takes still hides the tooltip.
   After each, when `DocumentInput::take_tooltip_changed()` is true
   (the tooltip was armed, armed again, shown or hidden), draw a frame:
   it draws the hide, and its `Next` takes the new deadline or drops
   the wait for an old one.
2. Each frame, after the document's update and before drawing, call
   `DocumentInput::tooltip(&document, now)` for the tooltip shown.
3. Each frame, fold `DocumentInput::tooltip_deadline()` into the
   frame's `Next`: `Next::WaitUntil(deadline)` where it would
   otherwise return `Next::Wait`. Ask on every frame until the tooltip
   shows, because a frame's `Next::Continue` or `Next::Wait` replaces
   the wait. When the tooltip hides before its deadline,
   `tooltip_deadline` is `None`, so the frame step 1 asked for waits
   for nothing and no frame is drawn at the old deadline.
4. After the frame, call `take_tooltip_changed()` again: the frame drew
   what its update showed or hid, and the next event must not ask for
   a frame for it.

An app that skips step 3 never shows a tooltip early: it shows at the
first frame drawn at or after its deadline, which may be much later.

## Where the tooltip goes

`vapors_app::place_tooltip(anchor, size, window) -> Result<Point, _>`
returns the tooltip's top-left corner in window CSS pixels, given the
tooltip's measured size and the window's. It draws nothing.

`Anchor` is `Pointer(Point)` (the hotspot) or `Keyboard(BoxRect)` (the
focused element's border box). It is not the `TooltipTrigger` of the
show/hide state machine, which says what brought the tooltip up. The
caller maps the shown `Tooltip`'s `trigger` to an anchor:
`TooltipTrigger::Pointer` to `Anchor::Pointer` with the pointer's
position, and `TooltipTrigger::Keyboard` to `Anchor::Keyboard` with the
border box of the `Tooltip`'s `node` (`Painted::border_box`).

Pointer anchor. Chromium's `TooltipAura::GetTooltipBounds`
(`ui/views/corewm/tooltip_aura.cc`, read 2026-11-10) places a cursor
tooltip at the hotspot plus (`kCursorOffsetX`, `kCursorOffsetY`) =
(10, 15), not below the element. Then it shifts the rectangle
horizontally to fit the display, and when the bottom is past the display
it sets the top to the hotspot's y minus the tooltip's height (above
the cursor). vapors does the same steps, in CSS pixels. Differences:

1. Chromium mirrors the x offset in right-to-left locales (not built).
2. Chromium flips above whenever the bottom overflows the display.
   vapors flips above only when the above position fits, and otherwise
   tries beside the hotspot, right first and then left. With no side
   that fits, the tooltip is placed below and clamped, and may cover
   the hotspot.

The beside positions are half-open at the hotspot column: the right one
starts at the hotspot's x plus 10, and the left one ends on the
hotspot's x (its right edge is outside the tooltip).

Keyboard anchor (no cursor). Chromium centres the tooltip on an anchor
point with its top on it. vapors takes the focused element's border box
instead: centred horizontally under it with a 4 px gap (`KEYBOARD_GAP`,
a choice, not Chromium's), above it with the same gap when it does not
fit below. When it fits neither, or the box is taller than the window,
it is placed below and clamped at the window's bottom, so a keyboard
tooltip may then cover its element.

Both: the result is slid inside the window. The hotspot is not covered:
the below position starts 15 px under it and the above position ends on
the hotspot's row. A tooltip longer than the window on a side starts at
0 on that side and overflows the other end. Chromium's `AdjustToFit`
shrinks it instead; vapors' painter decides whether to wrap or clip it.

A hotspot outside the window, or a box entirely off-screen, is
accepted and the result clamped into the window; a captured drag can
put the pointer outside. Only a NaN, an infinite value, or a negative
size, box width or height, or window length is refused. The error
(`PlacementError`) names the field and its value, for example
"`size.height` is -1", and says what to pass.

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

Drawing the tooltip: above every item including gpu-surfaces, styled
by `--tooltip-*` custom properties.

Chromium's other rules: its tooltip hides itself after 10 s
(`tooltip_controller.cc`, `kDefaultHideTooltipDelay`), and text past
2048 characters (1024 on Windows) is cut at a word break
(`tooltip_state_manager.cc`, `kMaxTooltipLength`). Neither is built.
