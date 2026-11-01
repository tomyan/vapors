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
| `input type=number` | Click focuses; drag on the field scrubs when `data-scrub` says so (touché's readout behaviour) | Digits, arrows by `step`, Enter commits | `input`, `change` | A value outside `min`..`max` is `:invalid` while typed and clamped on commit |
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
its `checked` attribute; `Document::checked`. A click flips it before
the click's listeners run, as HTML's legacy pre-activation behaviour
has it (a listener reads the new state, as on the web), flips it back
when a listener prevents the click, and otherwise fires `input` and
`change` carrying `checked` (`Event::checked()`, `Some` on those
events only), in Chromium's order (pointerdown, focusin, pointerup,
click, input, change); Space clicks it on keyup,
Enter does nothing; a disabled one gets its pointerdown and pointerup
and no click, focus or change. `:checked` follows the state, by click
or by property (`set_property` marks the restyle). The box is the
element's own (the UA sheet: a 1 px grey border, a 2 px radius, white,
Chromium's `3px 3px 3px 4px` margins), styled as any element; the mark
when checked is a rounded square in `--vapors-mark-color` inset a
quarter of the content box (no path primitive yet: F3 brings paths,
and a check glyph can replace it then). Without a size it is Chromium's
13 x 13 px (its content 11 px plus the UA sheet's 1 px border, so an
app that sets `border: 0` gets 11 px where Chromium stays 13), and as
a flex item it stretches across the line as Chromium's does
(`fixtures/checkbox.boxes.json`), so it is a measured leaf, not a
replaced box. Checked step by step against Chromium by
`fixtures/checkbox.events` with `expect checked`. Not built: `radio`,
`indeterminate`, `label` association (`label` is not an element yet).

As built (3-E3c, 2026-10-27): `input type=number`, the least editing
that takes a typed exact value before E6's editor. Its text is the
`value` property once set (a number written, or a text as typed), else
the `value` attribute; the DOM's `value` is the text when it is a
valid floating-point number, else empty, and `valueAsNumber` NaN then.
The caret is always at the end: typed characters a number takes
(digits, signs, the point, `e`; Chromium drops the rest) go there, or
replace the whole text when it is selected; focus by keyboard selects
it, as Chromium does, and a click does not; Backspace deletes the last
character; `input` fires with each edit. Enter and blur commit: a
number outside `min`..`max` is clamped into it and text that is no
number reverts to the last committed number (vapors' own rule:
Chromium keeps the text), so `change` always carries a number, and
fires only when the commit differs from the last; the blur's `change`
comes before its `focusout`, as Chromium orders it. ArrowUp and
ArrowDown step by `step` (1 for `any`), in decimal (a step of 0.1 from
0.2 is 0.3, as Chromium's `AlignValueForStep` keeps it), each with
`input` and `change`; Shift by ten, vapors' own (Chromium steps by
one), pinned by `tests/number_input.rs`. `:invalid` while the text is
no number or out of range (a step mismatch is not flagged), following
a later `min`, `max` or `step` too.

Two differences from the web, decided with the lead (2026-10-27):
(1) `bind:value` on a number field updates on `change`, not `input` as
Svelte's and the web's bindings do, so the app's `Signal<f64>` never
takes NaN mid-typing; pinned by `codegen_bind.rs:
bind_value_on_a_number_input_binds_a_number_on_change`. (2) On commit
a number outside `min`..`max` is clamped into it and text that is no
number, an emptied field included, reverts to the last committed
number, where Chromium keeps the text (and `""`) and matches `:invalid`
after the commit too; pinned by `tests/number_input.rs:
enter_commits_an_out_of_range_number_clamped_with_one_change`,
`text_that_is_no_number_has_an_empty_value_and_no_number_until_committed`
and `invalid_matches_while_the_text_is_no_number_or_out_of_range`. With `data-scrub`, a drag along the field steps the number
by one `step` per 2 CSS px (`data-scrub="4"`: per 4 px), `input` per
step and `change` at the release, counting from the text's number or
the last committed one; the press still focuses; a finger decides at
its first move whether it scrubs or scrolls, as a range's does; a
`data-scrub` of another shape, or on another input, is refused. The field's text is laid out as its content (its height is its
line, a blank field keeps it with a no-break space); the caret (1 px,
the text's colour) and selection (the ring's blue at a third) are
painted by `number_paint.rs`; a field needs a width of its own, since
Chromium's default (185 x 24 px at 14 px, from its font and spin
button) is not modelled: `width: auto` is refused naming the fix. The
`Character` platform event reaches `Document::character_typed`.
Checked step by step against Chromium by `fixtures/number.events`
(typing, Backspace, a dropped letter, Enter, arrows, Tab's selection,
blur by a click). Not built: caret movement and selection by pointer
or Home/End (E6), the spin buttons, `step` mismatch as `:invalid`,
a locale's decimal separator.

As built (E6a, 2026-11-01): `input type=text` (an `input` without a
type is one), the editing model's first half, on Parley 0.12's
`PlainEditor`: one editor per field, made when the field is first
pressed or focused, given the field's text style whenever it changes
so its geometry is the painted text's. The text is the `value` property,
written on every edit and read back when a signal sets it (the
selection clamped). A press puts the caret where it lands; on a
disabled field it leaves the selection where it was and fires
`selectionchange`, as Chromium does; a drag extends the selection under
capture; focus by keyboard selects the text whole. By touch, a tap
focuses the field and places the caret after its `click`, and a finger
down or dragging moves nothing (a drag pans), as Chromium's touch does.
A `value` attribute written before the field is edited reaches the
editor; once edited, the property holds, as the DOM's dirty value
does. Typed text replaces
the selection; Backspace and Delete remove the selection or a cluster,
with Control as far as Control and the arrow would move (Linux
Chromium: Control+Delete leaves the space after a word, Control+
Backspace takes "-" alone; Parley's own word deletes differ); the
arrows move by cluster, a word with Control,
Home and End to the ends, each extending with Shift; Control+A selects
all; Enter and blur commit (`change` when the text differs from the
last commit, the blur's before `focusout`). `input` fires per edit with
the value. `selectionchange` fires on the field once per
`Document::settle_selection` when the selection moved (the app settles
before each update, the vapors player after each event), as Chromium
queues one per task: two keys in one frame are one, a key that moves
nothing none, focus arriving in a field one even where the caret was.
`Document::selection` answers in UTF-16 units, as `selectionStart`/`selectionEnd` count. The caret (1 px, the
text's colour) and selection (the ring's blue at a third) come from
the editor's geometry, painted in the field's step under its text. A
field needs a width of its own, as a number field does. Checked step
by step against Chromium 155 by `fixtures/text.events`, every text
field's selection after every step; the Chromium player waits a task after each input call,
and the vapors player settles after each event, because Chromium's
queued `selectionchange` otherwise lands before or after the next key
from run to run. Clicks in a row (E6a, the second pull request): the
platform sends `Event::ClickCount` after each press of the primary
button, and the document places the caret, selects a word or the whole
text on it (`Document::pointer_clicks`), as Chromium does on
`mousedown`, after `pointerdown`. The platform counts on every
target, the web included: winit cancels `pointerdown` (so the canvas
selects no text), which in Chromium suppresses `mousedown` and with it
the browser's own count (probed, Chromium 155: `pointerdown`'s `detail`
is 0, and only `click` and `dblclick` carry the count, after release).
The interval and distance are stated fallbacks: GTK's defaults on Linux
(400 ms, 5 px, as Linux Chromium reads), Windows' (500 ms, 4 px),
macOS's (500 ms; no distance setting, so 4 px); on the web, the one the
browser's platform names (`navigator.userAgentData`, else the user
agent). No build compiles a reader for the systems' own settings yet;
a CI target that does retires the fallbacks. RELAXED: winit 0.30 gives
events no timestamp, so a press is stamped when winit hands it to the
platform, and presses queued behind a slow frame are stamped together;
retired when winit exposes event times. A double click
selects the word after the caret position nearest the point, as
Chromium does (just past a word, the space after it; past the text,
the last word), a triple click the whole text, neither starting a
drag. A press inside a selection, ends included, places the caret on
release, as Chromium waits to see whether the press drags the text.
Checked by `fixtures/words.events`, whose field sets its font and
border, which Chromium's field does not take from the page. A known
difference: words are Parley's (UAX 29), which keep a "." between
letters inside a word, so a double click in "e.g." selects "e.g" where
Chromium selects the "." alone; "3.5" and "don't" agree. A press inside
the selection moves the caret after its `click`, so a `click` listener
reads the selection held, as in Chromium. Not built: `detail` and
`dblclick`, dragging by word or line after a double or
triple click, dragging selected text, IME, `beforeinput`,
`placeholder`, `maxlength`, scrolling a long text to the caret, the
blinking caret (area 6), vertical selection in a `textarea`.

As built (E6b, 2026-11-01): a field's own undo and paste. Each field
keeps an undo stack, grouped as Chromium groups a field's edits: a run
of typing is one step, a run of Backspace (or of Delete) another,
typing after a deletion joins it and a deletion after typing does not,
a paste is a step alone, and anything that moves the selection other
than an edit ends the run. Control+Z undoes to the text and selection
before the step, Control+Shift+Z and Control+Y redo to those after it,
each with `input` and a `selectionchange` even where the caret was; a
new edit drops what could be redone; a signal setting the text drops
the stack. A paste (`Document::text_pasted`) fires `paste` on the
focused field, bubbling, and a listener that prevents its default
keeps the text out; else it replaces the selection, each line break a
space. On the desktop the platform sends `Event::Paste` whatever has
focus, and `ElementsApp` hands it to a focused text field first,
reading the clipboard's text (`Clipboard::text`, three answers, a
deadline); with none focused the clipboard is not read. Checked by
`fixtures/undo.events`, whose `clipboard` step writes Chromium's
clipboard. Chromium's undo stack is the page's, not the field's: an
undo in one field after edits in another differs, and is not compared.
Not built: copy and cut, the web page's paste (with the web input
path), Command for Control on macOS, `beforeinput`'s `historyUndo`.

As built (E6, the web's text input, 2026-11-01): a hidden `<textarea>`
mirrors the focused text field, chosen over EditContext because one
path serves Chrome, Safari, Firefox and Android's keyboards, doc 03
needs it as the fallback anyway, and headless Chromium drives it with
`Input.insertText` and `Input.imeSetComposition`; EditContext can
follow as Chromium's path. The app sets the field
(`Platform::set_text_field`: text, UTF-16 selection, caret area) after
each event and frame when it changed; the textarea takes it, moves to
the caret (transparent, 16 px so a phone does not zoom, no pointer
events) and takes DOM focus when the app handles the press or tap,
so that a phone may open its keyboard, which needs focus inside the
user's gesture (not checked headless). Its `input` and composition events come back as
`Event::TextEdited` (its whole state, and the text it was made
against); the field takes the difference, composing text included,
underlined, with `input`, and refuses an edit made against a text it
no longer holds, reporting it and setting the field again. Its keys
come back as `Event::Key`; it acts itself only on printable
characters, any key while an IME composes, and Control or Command with
C, V or X, and every other key's default is prevented, so the field's
own keys (arrows, Backspace, Enter, Tab, undo) act. Its `paste` comes
back as `Event::PastedText`, through `Document::text_pasted`. Checked
by `scripts/web-text-input.sh`: typed text, a composition and its
commit, Backspace, an arrow, a composition in the middle and Shift+Home
on Chromium's own `<input>` and on the built page's field demo, the
text and selection equal after each. Not checked headless: a real
phone keyboard (Gboard's autocorrect and its Backspace on composed
words), the keyboard opening on a tap, the candidate window's place.
Not built: EditContext, copy and cut, the a11y mirror's ownership of
the textarea (area 5).

As built (E6c, native IME, 2026-11-01): while a text field has focus
the window allows the system IME (`set_ime_allowed`) and places its
candidate window at the field's caret (`set_ime_cursor_area`, from the
field the app sets for the text input). winit's `Ime` events become the
same `Event::TextEdited` the web's textarea sends, kept against that
field: a preedit replaces the selection when it starts and its own
range after, in the value while it composes and underlined, its caret
where the IME puts it; a commit replaces the composing range and ends
it; `Disabled` clears a composition; a field set with another text, or
none, ends it. So the web and the desktop agree with Chromium's
`<input>`. Checked by unit tests on winit's own documented sequences
(pinyin, a dead key over a selection); not checked with a real IME,
which needs a desktop session (macOS, Windows, Linux with IBus or
Fcitx) at the sign-off, including whether a platform also sends a
key's text alongside its commit.

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
- Preedit text is shown inline with an underline and is in the value
  while it composes, with `input` per update, as Chromium's own
  `<input>` holds it (the lead's decision, 2026-11-01: the web follows
  the browser, and native winit IME matches it, so the two agree; the
  launcher's list filters as you compose). `beforeinput` is not fired
  for preedit changes (as in browsers), `compositionupdate` is.
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
