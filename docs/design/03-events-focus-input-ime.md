# 3. Events, focus, input and IME

Status: design, 2026-10-25. Built: pointer hit testing in paint order,
`pointerdown`/`pointerup`/`pointermove`/`click` with bubbling,
`:hover` and `:active`, a disabled button, `cursor`, platform gestures
(pinch, pan, tap). This page designs the rest: pointer capture and
drag, wheel and scroll routing, keyboard, focus and `:focus-visible`,
the controls touché's cards need (range, number, checkbox, button),
text editing with IME, and the default actions each element carries.

## The model

**DOM semantics, one dispatch.** Every input becomes one event
dispatched on the element tree in the order the UI Events and Pointer
Events specifications give: capture phase down from the root, target,
bubble phase up, with `stop_propagation` and `prevent_default`. The
compiler's `onclick={h}` is a bubble-phase listener; `oncapture:click`
is deferred until something needs it. Default actions (a button's
activation, a checkbox toggle, a range's arrow keys, a scroll
container's wheel) run after dispatch unless prevented, as in a
browser. There is one dispatch function, used by pointer, keyboard,
focus, wheel, input and synthetic events alike (ARCHITECTURE.md).

**Units.** Platform events arrive in physical pixels; dispatch is in
CSS px of the last update's scale, with `f32` precision kept (no
rounding for a slider thumb). Each event carries `client` (viewport)
and `offset` (target's padding box) coordinates; `page` is the same as
`client` because in vapors the document root never scrolls: only
scroll containers do (a framework rule, which touché's shell also
follows).

## Pointer

| Piece | Semantics | From |
|---|---|---|
| Pointer identity | `pointer_id`, `pointer_type` (mouse, touch, pen), `is_primary` | Pointer Events 3 |
| Pressure, tilt, twist | On pen and touch where the platform reports them; `0.5` for a mouse button down, as the spec says | Pointer Events 3; winit's tablet support is checked at the start of the slice and the gaps are platform code |
| Capture | `set_pointer_capture(node)` from a handler during `pointerdown`; implicit capture for touch (every touch pointer is captured by its `pointerdown` target); released on `pointerup`/`pointercancel` or `release_pointer_capture`; `gotpointercapture`/`lostpointercapture` fired | Pointer Events 3 §Pointer capture |
| `pointercancel` | When the platform takes the pointer (a system gesture, a touch that becomes a pan the scroll container owns, window loss) | Pointer Events 3 |
| `click` | Fired on the nearest common ancestor of the `pointerdown` and `pointerup` targets, after `pointerup`, for the primary button only; `detail` counts clicks within the platform's double-click interval | UI Events |
| `dblclick`, `contextmenu` | `contextmenu` on secondary button or long press (touch), with `prevent_default` to stop the platform's menu natively | UI Events; touché's Context menu pattern |
| `pointerenter`/`pointerleave`, `pointerover`/`pointerout` | Computed from the hover path change each frame, enter/leave non-bubbling | UI Events |
| Coalesced moves | Several `pointermove`s between frames are delivered as one event with `coalesced()` listing the rest, so a brush can use every sample while the tree sees one dispatch per frame | Pointer Events 3 `getCoalescedEvents` |
| Wheel | `wheel` with `delta_x`, `delta_y` in px (Chromium's Linux line factor measured, F2c S9 decision), `ctrl` for pinch on a trackpad where the platform reports it that way; the default action scrolls the innermost scrollable ancestor, or zooms the canvas element when it says so | UI Events; S9 |
| Gestures | The platform's `Pinch`, `Pan` and `Tap` remain for the canvas element, which claims touches inside it (`touch-action: none`); elsewhere a touch pan scrolls the one container it started in for the whole gesture (lead's answer to impl-f2c, 2026-10-25) | The CSS `touch-action` model |

As built (3-E1a, 2026-10-26): a listener has no element handle, so
`event.set_pointer_capture()` captures to the element whose listener
called it, which is the web's `this.setPointerCapture(e.pointerId)`
with the listener's element as `this`; `release_pointer_capture()`
likewise. A change takes effect before the pointer's next event, with
`lostpointercapture` then `gotpointercapture`; `pointerup` releases.
`Event::client()` and `offset()` (from the target's padding box) are
CSS px, unrounded (`[0, 0]` on an event that is not a pointer's).
Leaving the window while captured keeps the press. While captured,
`:hover` stays on the capturing element and a second `pointerdown` goes
to it. A capturing element removed from the document gives the pointer
back, and its `lostpointercapture` goes nowhere (the web fires it on
the removed element, where nothing can observe it). Asking for capture
with no pressed pointer (from `click`, a release, `got`/`lostpointercapture`
or a key) panics, naming the event, as the web throws
`InvalidStateError` (review-v77).
Moves are dispatched as they arrive and layout waits for the frame, so
many moves between frames are one update (`elements_app_drag.rs`).

As built (3-E1b): finger `n` is pointer `2 + n`, captured by its
`pointerdown` target without asking; its moves and `pointerup` go
there, and `pointercancel` ends it. The click is still the platform's
tap, after the touch's own down and up, so one tap is one click; its
event carries pointer id 2 whichever finger tapped. A finger pans only
once it has moved past the tap slop (20 px), so a tap and a pan are
exclusive, as Chromium's slop makes them (review-v78). Until 3-E4's
`touch-action`, ownership is RELAXED (the lead's decision): while a
touch whose `pointerdown` listener called `set_pointer_capture` is
down, no pan scrolls (Chromium cancels such a touch all the same: only
`touch-action: none` stops that); otherwise the first pan cancels the
touches no listener owns, whether or not anything scrolls, as Chromium
does (checked by `fixtures/capture.events`). `:active` follows the
first finger down only and clears when it lifts (or when the mouse
button is released). `Pointer` has no `pointer_type` or `is_primary`
yet: a listener tells a touch by `id >= 2`. A finger is forgotten when
it lifts or is cancelled even if its event found no element; nothing
sends `pointercancel` for window loss yet.

Drag of a slider thumb or a curve point: `pointerdown` captures, each
`pointermove` carries full-precision coordinates to the captured
target whether or not the pointer is over it, `pointerup` releases.
Reorder of cards: the same, plus the FLIP animation from area 6. HTML
drag-and-drop (`dragstart`, `drop`) is not implemented; pointer capture
covers reordering, and file drops arrive from the platform as
`Event::FileDropped`, dispatched as `drop` on the element under the
pointer.

winit as of 2026-10-25: 0.30.13 is stable and has touches, no tablet
tools and no IME on the web; 0.31 (beta 4) folds touch into pointer
events, adds tablet-tool events and a clipboard API, and changes the
order of `Ime::Preedit` and `Commit`. The pointer model above is
0.31's shape, so the move to 0.31 is a mapping change in the platform
crate, and the web IME path is platform code either way.

## Keyboard

- `keydown`, `keyup` with `key` (the UI Events `key` value: `"a"`,
  `"Enter"`, `"ArrowLeft"`), `code` (physical key), modifiers, `repeat`.
  `keypress` is not implemented (deprecated in browsers).
- `beforeinput` and `input` on editable elements, with `input_type`
  (`insertText`, `deleteContentBackward`, `insertFromPaste`, ...) per
  the Input Events specification, so an editor can intercept.
- Shortcuts: a `shortcut` attribute is not a framework feature. touché's
  shortcut table (`shortcuts.js`) becomes a component that listens for
  `keydown` on the root and dispatches; the framework provides
  `event.matches("Mod+Z")`, where `Mod` is Cmd on macOS and Ctrl
  elsewhere.
- Key events go to the focused element, else `body`.

## Focus

- One focused element per document, `document.active_element()`.
  `tabindex`: `0` joins the sequence, `-1` is focusable by script or
  pointer only, a positive value is refused (the WAI-ARIA Authoring
  Practices advise against it; refusing is cheaper than ordering).
  Interactive elements (`button`, `input`, `textarea`, `select`, `a`
  with `href`, `summary`) are focusable by default; `disabled` removes
  them.
- Tab and Shift+Tab move through the sequential focus order: tree
  order among focusable elements, skipping `display: none`,
  `visibility: hidden` and `inert` subtrees. A focus trap (a modal
  `dialog`) is a subtree that wraps instead of leaving.
- `focus`, `blur` (not bubbling), `focusin`, `focusout` (bubbling), in
  the UI Events order.
- `:focus` is set on the element; `:focus-within` on its ancestors.
- As built (3-E2a): `Document::active_element`, `focus`, `key_pressed`
  and `key_released`; the platform sends `Event::Key` (the DOM's `key`
  with the modifiers held; no `code` yet) before a press's `Character`.
  Tab past the last focusable element leaves the page (nothing has
  focus), as in Chromium, rather than wrapping. A press focuses the
  nearest focusable element, or blurs, unless `pointerdown` is
  prevented. `blur`, `focusout`, `focus`, `focusin` fire in Chromium's
  order. A positive or non-integer `tabindex` is refused by name.
  Checked against Chromium by `fixtures/focus.events` (events and focus
  at each step; `focus` and `blur` are not listened for there, as
  Chromium also sends them to the window, which vapors does not have).
  Known gaps (review-v81): listeners see a key's `key` but not its
  modifiers or repeat yet; winit's key names are mapped where they
  differ from the DOM's only for Space and Super (Meta); "disabled" has
  two answers, a disabled control itself for focus and a disabled
  button ancestor for clicks, and `fieldset disabled` neither;
  `focus` and `blur` listeners that move focus again are not
  re-checked between the four events; focusability reads the last
  painted frame, so an element shown since the last update is not yet
  focusable. A prevented `pointerdown` does not focus, as in Chromium
  155 (`gpu_events_a_prevented_pointerdown_does_not_focus_in_chromium`);
  a touch tap does not focus yet.
- As built (3-E2b): `:focus-visible` is set when focus arrives after a
  key other than a modifier alone (by Tab, or by script after a key) or
  on a text field, and not
  after a press, matching Chromium at every step of
  `fixtures/focus.events`. `outline` (width, style, colour, offset and
  the shorthand) draws a solid ring outside the border box, painted
  with its own layer (after its normal-flow content, before the
  positioned layers above it, as Blink paints it), following rounded
  corners, clamped at zero for a negative offset; `auto` and dashed,
  dotted and double outlines are refused. Checked against Chromium's
  pixels by `fixtures/outline_paint`. The UA sheet's ring is vapors'
  own: `:focus-visible { outline: 2px solid #0060df; outline-offset:
  1px }` (not Chromium's native `auto` ring); buttons are
  `box-sizing: border-box`, as Chromium's are.
  `:focus-visible` follows the HTML heuristic: set when focus arrived by
  keyboard or by script after a keyboard interaction, not when it
  arrived by pointer, except on text inputs where it is always set.
- Focus is always visible: the UA sheet gives `:focus-visible` an
  outline in `accent`. `outline` is a new property for the elements
  layer (painted outside the border box, not laid out).
- Focus moves with the keyboard inside composite controls per the
  Authoring Practices: arrow keys in a range, a radio group, a listbox,
  a menu; Home and End; Escape closes a dialog, popover or menu and
  returns focus to the opener.

## Controls, with their default actions

The brief's list, in touché's order. Each is an element with
user-agent behaviour and an implicit ARIA role (area 5).

| Element | Pointer | Keyboard | Events | Notes |
|---|---|---|---|---|
| `button` | Press shows `:active`; `click` on release over the common ancestor | Enter and Space activate (Space on keyup, as browsers do) | `click` | Built, plus keyboard |
| `input type=range` | Press anywhere on the track sets the value and captures; drag changes it; `step` snaps | Arrow keys by `step`, Page keys by a tenth of the range, Home and End; Shift for ten steps | `input` on every change, `change` on release | Scrub to change, `value_as_number`. The thumb and track are UA parts styled through prefixed custom properties the UA sheet reads (`--vapors-thumb-size`, `--vapors-thumb-color`, `--vapors-track-height`, `--vapors-track-color`), so tokens apply; `::part()` comes later (Q1, decided 2026-10-25) |
| `input type=number` | Click focuses; drag on the field scrubs when `data-scrub` says so (touché's readout behaviour) | Digits, arrows by `step`, Enter commits | `input`, `change` | A value outside `min`..`max` is clamped on commit and the field says so through `:invalid` |
| `input type=checkbox`, `type=radio`, `switch` role | Click toggles | Space toggles; arrows move within a radio group | `input`, `change` with `checked` | `bind:checked` exists |
| `select` | Opens a listbox popover (area 3 owns the popover primitive) | Arrows, type-ahead, Enter, Escape | `change` | Deferred until touché has one; the model choice uses it |
| `input type=text`, `textarea` | Click places the caret; drag selects; double click selects a word | The editing model below | `beforeinput`, `input`, `change`, `selectionchange` | |
| `a` | `click` opens the href through the platform's URL service; a `#fragment` scrolls | Enter | `click` | F2d |
| `details`/`summary` | Toggles | Enter, Space | `toggle` | For collapsible cards |
| `dialog` | `show_modal()` traps focus, `inert` elsewhere, Escape closes | | `close`, `cancel` | F5 |
| `popover` attribute | Light dismiss on outside click or Escape | | `toggle` | F5 |

As built (3-E3a, 2026-10-27): `input type=range` is an element; the
other input types are refused naming their slice (checkbox E3b,
number E3c, text E6). Its numbers follow HTML's sanitization (`min` 0,
`max` 100, `step` 1, `any` for none, the step base `min` else the
`value` attribute; the value clamped and snapped, the middle without
one); the live value is the `value` property once set (`bind:value`
on a range or number input binds a `Signal<f64>` through
`PropertyValue::Number` and `Event::value_as_number()`), else the
attribute. A press takes the value where the thumb's centre would be
(half a thumb in at either end, as Chromium maps it) before focus
moves, captures the pointer to the input and drags; `input` fires on
each change, `change` at the release when the value differs from the
press's, after `pointerup` and before `lostpointercapture`. A finger
takes the value where it lands and decides at its first move, as
Chromium's `SliderContainerElement` does: moved further along the
range than across it, the range owns the touch (the RELAXED rule's
ownership, so no pan takes it; equal counts as along, as Chromium's
`GetDirection` has it); further across, the touch is the pan's and the
range takes no more of its moves. A finger's drag ends at its lift,
with `change` after `lostpointercapture` (the mouse's comes before it)
and even after the pan's `pointercancel`, as Chromium's slider fires it
on touchend; a finger the platform takes (`touch_cancelled`: a palm, a
system gesture) ends its drag with nothing, as Chromium fires no
`change` on `touchcancel`. Each pointer drags on its own, so two
fingers on two ranges drag both. Arrow keys step by one step (`step="any"`: a
hundredth of the range), Page keys by a tenth of the range or one
step when larger, Home and End to the ends, each with `input` then
`change`; these match Chromium step by step
(`fixtures/range.events`, with `expect value` checking each range's
`value` after every step). Shift with an arrow steps by ten steps,
vapors' own default action (`2026-10-27-direct-tools-first`, 3):
Chromium steps by one, so it is pinned by `tests/range_keys.rs`, not
the oracle. A key pressed while an element is focused sets
`:focus-visible` on it, as Chromium does after a press focused it. A
disabled range gets its `pointerdown` and `pointerup` and no click, no
focus and no value, as Chromium. The parts are read from the four
custom properties wherever they are set (an ancestor's, so tokens,
inherit; the user-agent values are in `style/parts.rs`, not the UA
sheet, so an inherited declaration wins); a value of another shape is
refused where it is declared. The look differs from Chromium's native
slider on purpose (Q1), so the fixture is not paint-compared; the
`slider` demo's `.pixels` pins vapors' own. Without a size a range is
Chromium's 129 x 16 px with its 2 px margin (`fixtures/range.boxes.json`).
Not built: `data-scrub` (E3c), `:invalid`, a vertical range, `list`
tick marks, and `pointercancel` for window loss ends a drag without
`change`.

As built (3-E3b, 2026-10-27): `input type=checkbox`. Checked by its
`checked` property once set (`bind:checked`, a `Signal<bool>`), else
its `checked` attribute; `Document::checked`. A click toggles it after
the click's listeners ran, unless one prevented it, then `input` and
`change` fire carrying `checked`, in Chromium's order (pointerdown,
focusin, pointerup, click, input, change); Space clicks it on keyup,
Enter does nothing; a disabled one gets its pointerdown and pointerup
and no click, focus or change. `:checked` follows the state, by click
or by property (`set_property` marks the restyle). The box is the
element's own (the UA sheet: a 1 px grey border, a 2 px radius, white,
Chromium's `3px 3px 3px 4px` margins), styled as any element; the mark
when checked is a rounded square in `--vapors-mark-color` inset a
quarter of the content box (no path primitive yet: F3 brings paths,
and a check glyph can replace it then). Without a size it is Chromium's
13 x 13 px, and as a flex item it stretches across the line as
Chromium's does (`fixtures/checkbox.boxes.json`), so it is a measured
leaf, not a replaced box. Checked step by step against Chromium by
`fixtures/checkbox.events` with `expect checked`. Not built: `radio`,
`indeterminate`, `label` association (`label` is not an element yet).

## Text editing and IME

One editing model for single- and multi-line fields, built on Parley's
editor (`PlainEditor` and `PlainEditorDriver`, parley 0.12: movement
and selection by cluster, word, soft line and hard line, insert and
delete, `set_compose`/`finish_compose` for IME preedit; one style for
the whole text, which suits a plain-text Composer). vapors adds undo,
the clipboard, caret painting and the DOM events. Rules:

- Caret and selection are painted by the elements layer in `accent`
  and the selection colour, snapped to device pixels; the caret blinks
  on the animation tick (area 6) and stops while typing.
- IME: natively, winit's `Ime` events (`Enabled`, `Preedit`, `Commit`,
  `Disabled`), with `set_ime_allowed` on focus of an editable element
  and `set_ime_cursor_area` set to the caret's rectangle on every
  layout, so the candidate window follows the caret. On the web, the
  EditContext API where present (Chromium), with its
  `textupdate`/`compositionstart`/`compositionend` events and
  `updateControlBounds`/`updateSelectionBounds` for the candidate
  window; elsewhere a hidden `<textarea>` positioned at the caret. DOM
  focus on the web is owned by the a11y mirror (area 5): the mirror
  element of the vapors-focused node holds it, and for an editable node
  that element is the EditContext's host (or holds the hidden textarea),
  so IME and the screen reader agree on one element. `keydown` is
  listened for on the document, so it arrives whichever element has DOM
  focus. The platform reports which IME path is in use in the status
  line once, and
  area 10 records it, because the hidden textarea has known gaps
  (composition in some IMEs, the caret rectangle) that the brief
  accepts for the secondary target.
- Preedit text is shown inline with an underline and is not in the
  value until commit; `beforeinput` is not fired for preedit changes
  (as in browsers), `compositionupdate` is.
- Clipboard through area 7: `paste` carries text and images; the
  Composer's image paste is an event with `files`.
- Undo inside a field is its own stack, separate from the app's
  document history; Mod+Z in a field undoes the field.
- `user-select` for read-only text (agent replies): selection across
  elements is a range over the inline runs, copy serialises the text
  with newlines at block boundaries.

## Synthetic input

Tests and scenario replays (area 10 C4) inject events at the platform
layer (`ElementsApp::event`) so they travel the same path as real
input, including gesture derivation and capture. There is no second
entry point. The brief's automation API is deferred (Q13); when it
arrives it injects here too.

## Taken from where

- UI Events, Pointer Events 3, Input Events 2, HTML's focus rules and
  the WAI-ARIA Authoring Practices: the semantics, so the oracle is a
  browser.
- The web's `touch-action` and implicit touch capture rules rather
  than winit's raw touches: they decide who owns a gesture before it
  starts, which is what a scroll container beside a canvas needs.
- Parley's editor rather than a new one; cosmic-text's editor was
  considered and not taken because vapors already lays out with
  Parley and one shaper is one answer.
- GPUI's and Xilem's dispatch were read for shape; neither models
  default actions or focus heuristics, which vapors needs to match a
  browser.

## Deliberately left out

- HTML drag-and-drop events; pointer capture covers reorder, and file
  drops are a platform event.
- `keypress`, `mouse*` events (`pointer*` cover them; `onmouseover`
  compiles today only to be refused as unfired, which stays).
- Positive `tabindex`.
- Touch-specific `touch*` events: touches arrive as pointers.
- A gesture recogniser beyond pinch, pan, tap and long press.
- A `contenteditable` rich-text editor: the markdown view is read-only;
  the composer is a `textarea`.

## Oracles

| Part | Oracle |
|---|---|
| Dispatch order and targets | Chromium: the page-contract script drives the same HTML with synthetic pointer and key events and records the event log (`type`, target id, phase) through a listener script; the vapors log must equal it. Cases from WPT `pointerevents/` and `uievents/` testharness tests that drive synthetic input, ported where headless Chromium can run them |
| Capture | WPT `pointerevents/pointerevent_setpointercapture_*` and the implicit-capture tests, run in Chromium for the reference log |
| `click` on the common ancestor, `detail` | The existing S2 tests plus a Chromium log |
| Focus order and `:focus-visible` | Chromium's `document.activeElement` after a scripted Tab sequence (`Input.dispatchKeyEvent` over the DevTools protocol, which the oracle runner gains); `:focus-visible` matched via `matches(':focus-visible')` |
| Controls | WPT `html/semantics/forms/the-input-element/` cases for range and number value sanitisation and stepping; keyboard behaviour against a Chromium log |
| Editing | A scripted transcript (type, move, select, delete, paste) against Chromium's `<textarea>` value and selection after each step; IME against a recorded composition transcript, since no headless IME exists; by hand on macOS and Windows with a real IME at the sign-off |
| Pen | No automated oracle (no pen on this box); a recorded transcript from Tom's hardware when it exists |

## Slices

| Slice | Delivers | touché use |
|---|---|---|
| E1 capture and drag | `set_pointer_capture`, implicit touch capture, `pointercancel`, full-precision coordinates on `Event` | Draggable slider thumbs; curve points later |
| E2 keyboard and focus | `keydown`/`keyup`, `tabindex`, Tab order, `:focus`, `:focus-visible`, `outline`, button activation by key | Every card control reachable by keyboard; the screen-reader acceptance step |
| E3 range, checkbox, number | The three inputs with UA parts, default actions, `input`/`change`, `bind:value` as a number | The layer cards' sliders and toggles |
| E4 wheel routing and `touch-action` | Wheel to the innermost scroll container or the canvas; a touch pan owned by one container | The layer stack scrolls beside the canvas on the phone |
| E5 `contextmenu`, long press, `details` | | Card actions; collapsible cards |
| E6 text editing | Parley editor, caret, selection, clipboard, undo, native IME | The Composer in vapors |
| E7 web IME | EditContext, hidden textarea fallback | The Composer on the phone |
| E8 `select`, `dialog`, popover | | The model choice; About; sheets |
| E9 pen | Pressure and tilt | Brush masks (milestone 3) |
