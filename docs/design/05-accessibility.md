# 5. Accessibility

Status: design, 2026-10-25. Nothing is built; the brief makes a full
accessibility tree a requirement from F2 and touché's MVP acceptance
step 6 needs it (every control named, the conversation read as it
streams, the layer list a list). Decided here: AccessKit for the
platform adapters, with the semantics computed by vapors from HTML and
ARIA, and a DOM mirror on the web.

## The model

**Semantics come from the element tree, not from app code.** Each
element has an implicit role from HTML (`button` is `button`, `h2` is
`heading` level 2, `input type=range` is `slider`, `ul` is `list`,
`dialog` is `dialog`), and the ARIA attributes in a template override
or add to it (`role`, `aria-label`, `aria-labelledby`,
`aria-describedby`, `aria-live`, `aria-expanded`, `aria-selected`,
`aria-valuenow`, `aria-valuemin`, `aria-valuemax`, `aria-valuetext`,
`aria-hidden`, `aria-busy`, `aria-disabled`, `aria-controls`,
`aria-haspopup`, `aria-current`, `aria-checked`, `aria-pressed`,
`aria-modal`). Names are computed by the accname algorithm (aria-label,
then labelledby, then the element's own content for roles that allow
it, then `title`). An attribute the table does not list is refused by
the compiler, naming the attribute; an attribute the role does not
allow (per the ARIA in HTML conformance table) is refused too.

**One tree, one update per frame.** The accessibility tree is a
projection of the element tree: one node per element that has a role
or a name, plus text nodes as `InlineTextBox` children of their inline
run with character positions from Parley. Phase 5 of the frame (area
2) diffs the projection against the last pushed one and sends the
changed nodes as one `TreeUpdate`, with the focused node. There is no
second source of truth and no app-side a11y API.

**Actions come back through dispatch.** A screen reader's "press" on a
button is an AccessKit `Action::Click`; the adapter delivers it to the
UI thread, which dispatches `click` on the element through the one
dispatch function (area 3), so a handler cannot tell a screen reader
from a mouse. `Action::Focus` moves focus, `SetValue` on a slider sets
the value and fires `input` and `change`, `ScrollIntoView` scrolls the
container, `SetTextSelection` moves the caret.

**Loud in debug.** An interactive element with no accessible name is a
refusal of the update in debug builds and in tests, naming the element
and its position; in release it is logged once. A `dialog` without
`aria-labelledby` or `aria-label` likewise. Rollout: A1 ships the check
as a report (a count in the lint's output and in recordings) so every
existing demo and touché page can be fixed, and flips to a refusal in
the slice after, once the demos and touché's pages pass; the report
stage is the relaxed guard, declared here, retired by that flip. The
lint runs in the scenario tier against every touché page.

## Platforms

| Target | Adapter | Notes |
|---|---|---|
| Windows | AccessKit UI Automation | Through `accesskit_winit`, which owns the window handle integration |
| macOS | AccessKit NSAccessibility | The same |
| Linux | AccessKit AT-SPI | The same; the sign-off reader is Orca |
| Web | A DOM mirror: a hidden, focusable subtree beside the canvas, one element per a11y node with `role`, `aria-*`, `tabindex` and text, kept in step by the same `TreeUpdate` diff | AccessKit lists a web adapter as planned, not available (checked 2026-10-25). The mirror is what browsers expose to the platform's reader (TalkBack, VoiceOver, NVDA through the browser). The mirror's elements receive real DOM focus, which follows vapors' one focus state: when vapors focuses a node, its mirror element is focused; when a reader or Tab focuses a mirror element, vapors focuses its node. The mirror element of an editable node hosts the EditContext or hidden textarea (area 3), and `keydown` is listened for on the document, so there is one owner of DOM focus and it is a projection of vapors' focus, never a second state. Bounds are set by `style` so a reader's highlight lands on the canvas pixels. If AccessKit's web adapter ships first, it replaces the mirror behind the same diff |
| Android and iOS native | `accesskit_android` and `accesskit_ios`, both available | Not scheduled; touché DESIGN.md's note that AccessKit had no iOS adapter is out of date |

Why AccessKit: it is the one Rust abstraction with maintained adapters
for the three desktop platforms and both phone platforms, used by egui,
Bevy, Slint, Xilem and Zed's GPUI (macOS). Its own page says the
adapters do not yet cover rich text or hypertext, or every role and
property; the mapping table below stays inside what they do. Its tree model (nodes with a role, properties and
children; incremental updates; actions back) is close to a browser's
accessibility tree, so the mapping from HTML semantics is a table, not
a translation. Its text model (`TextSelection`, inline text boxes with
character lengths and positions) is the one browsers expose, which is
what VoiceOver's text navigation needs.

Why a DOM mirror on the web rather than AccessKit-shaped JavaScript: the
browser already turns DOM and ARIA into the platform tree, and screen
readers test against it. This is the approach of Google Docs' canvas
renderer and of Flutter web (its "semantics tree" of hidden DOM
elements), and the brief chose it in 4.7.

## Mapping

| HTML | Role | Properties pushed |
|---|---|---|
| `button` | `Button` | name, disabled, pressed (from `aria-pressed`), expanded, haspopup |
| `input type=range` | `Slider` | value, min, max, step, value text, orientation |
| `input type=number` | `SpinButton` | value, min, max, step |
| `input type=checkbox` | `CheckBox` | checked (true, false, mixed) |
| `input type=radio` | `RadioButton` | checked, group from `name` |
| `input type=text`, `textarea` | `TextInput`, `MultilineTextInput` | value, selection, placeholder, read-only, inline text boxes |
| `h1`–`h6` | `Heading` | level |
| `ul`/`ol`, `li` | `List`, `ListItem` | size, position |
| `nav`, `main`, `aside`, `header`, `footer`, `section` with a name | landmarks | name |
| `dialog` | `Dialog` | modal |
| `img` | `Image` | name from `alt`; `alt=""` is `Presentation` |
| `a` | `Link` | url |
| `p`, inline runs | `Paragraph`, `StaticText` + `InlineTextBox` | text, character positions, word boundaries |
| `div`, `span` without a role or name | not in the tree (generic, pruned) | |
| `aria-live` region | `Status` or `Alert` | live, atomic, relevant |
| `canvas` with children | `Group`; its children are what they declare | A curve point is a `Slider` child with `aria-valuenow` |

## Live regions

The chat panel streams a reply; touché's acceptance says it is read as
it arrives. `aria-live="polite"` on the message element makes it a live
region; AccessKit has `Live::Polite` and the adapters fire the
platform's live-region event on text change. The elements layer rate
limits the announcements: changes within one second are coalesced into
one live announcement (the tree update itself still goes every frame,
area 2 phase 5; only the announcement is held) (touché's Feedback pattern asks for at most one per second
for stage changes), and a streaming message announces its new sentence,
not its new token, by sending the update when a sentence boundary
arrives or the stream ends.

## Reduced motion and contrast

`prefers-reduced-motion` (area 7 feeds `Environment`; area 6 honours
it) and `prefers-contrast` are read from the platform; the UA sheet
and touché's tokens respond through `@media`. Focus is always visible
(area 3).

## Taken from where

- **AccessKit**: the tree model, update protocol, action requests and
  platform adapters; taken whole.
- **HTML and ARIA**: implicit roles (HTML-AAM), allowed attributes
  (ARIA in HTML), name computation (accname), the WAI-ARIA Authoring
  Practices for keyboard behaviour; taken as the specification to match.
- **Flutter web, Uno Platform's WebAssembly target and Google Docs**:
  the hidden DOM mirror on the web.
- **egui and Bevy**: their AccessKit integrations were read for the
  winit adapter's lifecycle (activation on first reader query, so an
  app with no reader pays nothing).
- **Not taken**: GPUI's and iced's approach of leaving a11y to the app
  or to a later layer; the brief's rule is that semantics come from the
  elements.

## Deliberately left out

- A `role` on arbitrary `div`s is allowed (ARIA), but an interactive
  role without the matching keyboard handling is refused in debug:
  `role="button"` on a `div` with no `tabindex` and no key handler.
- Custom accessibility nodes from app code, outside the element tree. A
  canvas overlay exposes its points as elements with roles, which is
  how the curves editor gets keyboard and reader access.
- Automatic alt text, automatic labels from nearby text.
- Braille display specifics, beyond what the adapters do.

## Oracles

| Part | Oracle |
|---|---|
| Roles and names | Chromium's accessibility tree for the same HTML: the oracle runner gains `Accessibility.getFullAXTree` over the DevTools protocol, and the test compares role and computed name per element. WPT `accname/` and `html-aam/` testharness cases ported where they test name computation |
| Allowed attributes | The ARIA in HTML table, encoded once; a test that refuses each disallowed pair from a generated list |
| Tree updates | A recorded sequence of `TreeUpdate`s for a scenario (a card expands, a slider moves, a message streams) pinned as a snapshot; the diff proven minimal by counting nodes per update |
| Actions | A fake adapter sends `ActionRequest`s; the test asserts the element events and signal values, proving a reader's press equals a click |
| Live regions | The coalescing rule under a stepped clock |
| Web mirror | Headless Chromium's a11y tree of the mirror equals the native projection for the same document, both dumped as the same JSON |
| Screen readers | By hand at sign-off: Orca here, TalkBack on the Pixel, VoiceOver and NVDA when the machines exist; a checklist per touché acceptance step 6 |

## Slices

| Slice | Delivers | touché use |
|---|---|---|
| A1 roles, names, the tree | Implicit roles, ARIA attributes with refusals, accname, the projection and diff, `accesskit_winit` on native, the lint for unnamed controls | Every card control has a role and a name; the layer list is a list |
| A2 actions and focus | Click, Focus, SetValue, ScrollIntoView through dispatch | A reader moves a slider |
| A3 web mirror | The hidden DOM subtree with focus kept in step | TalkBack on the Pixel |
| A4 text | Inline text boxes, selection, text inputs | The Composer and chat text are read by character and word |
| A5 live regions | Polite and assertive with coalescing | The streamed reply is read as it arrives |
| A6 dialogs and landmarks | Modal, landmarks, `aria-current` | About and sheets |
