# F2c: elements, layout and paint

Status: plan, for review before any code. Brief: touche
`UI-FRAMEWORK-BRIEF.md` sections 4.1 to 4.6 and 5; this page decides
how F2c delivers them and in which slices.

The goal: a compiled `.wisp` component plus CSS becomes boxes on
screen, in a native window and on the web canvas, and pointer input
reaches the component's listeners. After F2c touche can write its
shell (top bar, canvas region, side panel, toasts, scrolling lists) as
components.

## What exists and what is missing

| Piece | State on main |
|---|---|
| `vapors-dom` | The `Dom` trait, `{if}`/`{for}` blocks, `RecordingDom` (tests only) |
| `vapors-compiler` | `.wisp` to Rust; `STYLE` is the raw `<style>` text, unscoped and unused |
| `vapors-css` | Parse, cascade, `compute_tree`, `restyle_scope`, transitions; an `Element` trait to implement |
| `vapors-text` | Parley layout, atlas, `TextPipeline::draw` straight into a target |
| `vapors-scene` | `Color`, `Transform`, `Size`; no display list yet |
| `vapors-renderer` | `SolidPipeline::fill` and `ImagePipeline::draw`, one submit per call |
| `vapors-platform` | `App` with `event` and `frame`; redraws continuously |

So F2c also has to add the display list to `scene` and an instanced
quad pipeline to `renderer`. Neither exists today.

## Crates

```
vapors-elements   the element tree: implements Dom and css::Element; style,
                  layout (Taffy), text layout, hit testing, event dispatch,
                  paint to a Scene. No wgpu device needed for its tests.
vapors-scene      gains the display list: Item, Scene.
vapors-renderer   gains QuadPipeline: instanced rects with radii, borders, shadows.
vapors-app        new: ScenePainter (quads from renderer, glyphs from text, in
                  scene order) and ElementsApp, which implements platform::App
                  for a Document. Native window and web canvas both run it.
```

`vapors-elements` depends on `vapors-text` for Parley layout, which
pulls `wgpu` in as a compile-time dependency. No test in the crate
opens a device. Splitting `vapors-text` into layout and GPU halves is
possible later if compile time shows it is worth doing.

## The element tree

`Document` owns an arena of nodes. A `NodeId` is an index plus a
generation, so a stale id is detected rather than aliased.

- Node kinds: element (tag, attributes, properties, listeners, state
  flags) and text.
- The document starts with `html` > `body`. Components mount into
  `body` or any element below it. `html` matches `:root`.
- `Document` is shared as `Rc<RefCell<Document>>`, which is what the
  generated code takes. Dispatch drops the borrow before calling a
  listener, as `RecordingDom::dispatch` does, so listeners can write
  signals whose effects call back into the document.
- `css::Element` is implemented on a cheap handle (`&Document`, id).
  Parent, sibling and first-child links are stored, so each is O(1).

Failure handling:

- A `Dom` call with a stale or unknown id, `append` onto a text node,
  or an insert that would make a cycle panics, naming the call and the
  ids. Only a bug in generated code or the runtime can cause one.
- A tag the compiler accepts that F2c does not implement (`table`,
  `input`, `dialog` and the rest) is recorded as an error at
  `create_element`. The next `Document::update` returns it, naming the
  tag and the slice that brings it. It is never drawn as a `div`.
- `listen` for an event name the document never fires is refused the
  same way. F2c fires `click`, `pointerdown`, `pointerup`,
  `pointermove`, `pointerenter` and `pointerleave`. Otherwise
  `onmouseover` would compile and silently never run.

`Document::update(env) -> Result<(), Vec<ElementError>>` runs restyle,
then layout, then paint. `ElementsApp` reports any error with
`report_error`, which writes to `#error` on the web. It then stops
drawing new frames, as on device loss, rather than showing a frame it
knows is wrong.

## Styles

- **Sheets.** The document has a user-agent sheet (below), global
  author sheets added with `Document::add_stylesheet(text)`, and
  component sheets. touche's tokens (`design/tokens.json` as custom
  properties on `:root`) and its shell rules go in a global sheet. A
  parse error is returned from `add_stylesheet` with line and column.
- **Scoping.** sumi scopes each component's `<style>` to that
  component's subtree. The generated code does not currently tell the
  `Dom` which component created an element, so scoping needs a
  compiler change. **Decision needed (D1)**, listed at the end.
- **User-agent sheet.** `crates/elements/src/ua.css` holds the subset
  of the HTML specification's rendering section for the F2c elements:
  `display` for the block elements, margins for `p`, `h1` to `h6` and
  `body` (8 px, as in browsers), heading sizes and weights, `b` and
  `strong` bold, `em` and `i` italic, `code` monospace, and a button
  style. `font-family` on `:root` is Geist, the face vapors bundles.
  `sans-serif` and `system-ui` map to Geist, and `monospace` to Geist
  Mono. The CSS crate parses the sheet in a unit test.
- **Computed to used.** Each property in the CSS table is classified
  in one table in `elements/src/style/honoured.rs`: layout, paint,
  text, inherited-only, or deferred with the slice that implements it.
  A test asserts that every property in `vapors-css`'s table appears
  there exactly once, so adding a CSS property without deciding what
  the elements layer does with it fails. A deferred property whose
  computed value is not its initial value is an `ElementError`. The
  error names the property, the element and the declaration's
  position (`ComputedStyle::declared_at`). Values a mapping does not
  implement are refused the same way: `border-style: dashed`, or an
  inline element with a background before S6. Examples of deferred
  properties: `transform`, `filter`, `opacity` below 1 before its
  layer slice, and transitions.

## Layout: Taffy 0.15.0

Decision: Taffy, pinned `=0.15.0` (released 2026-10-17), used through
its low-level traits on the `Document` arena, not through `TaffyTree`.

Why Taffy:

- It implements CSS block (with margin collapsing), flexbox and grid,
  which covers brief section 5 items 1 and 2 and DESIGN.md's shell
  grid. Servo, Blitz, Bevy, Slint, Zed's GPUI and Floem use it, and
  its tests are generated from Chrome's layout of HTML fixtures. That
  is the same oracle this plan uses.
- `vapors-css` was written to match Taffy's `Style`: its flex and grid
  properties follow Taffy 0.9.

Alternatives considered:

| Option | Why not |
|---|---|
| Own layout engine | Months of work for flex and grid alone. sumi's Go layout is cell-based and does not transfer. |
| Yoga | Flexbox only, no grid. It is C++, so the FFI needs `unsafe`, which the workspace forbids. |
| Morphorm (Vizia) | Its own layout model, not CSS. That breaks "real HTML and CSS". |
| Stretch, Stretch2 | Unmaintained; Taffy is their successor. |
| Blitz (Stylo + Taffy) | Brings Stylo, a second CSS engine beside `vapors-css`. |

Why 0.15 rather than 0.14 (2026-08-24) or 0.9:

- `position: static` is the default and `absolute`/`fixed` boxes are
  placed against their containing block, as in CSS. Toasts and
  overlays need this, and 0.14 places them against the parent.
- `normal` is a real alignment value, not `None`. The sizing keywords
  (`min-content`, `max-content`, `fit-content`, `stretch`) arrived in
  0.14. `display: flow-root` arrived in 0.13.
- The risk is that 0.15 was released six days ago. Mitigations: pin it
  exactly, and run the Chromium geometry corpus (below), which is the
  check that would catch a regression. Moving to 0.14 means
  backporting position hoisting, which is the reason for choosing
  0.15.

The `vapors-css` grammar for `position` and the alignment properties
moves from 0.9's shape to 0.15's in the slice that maps them. The test
that checks the CSS table matches Taffy is updated in the same change.

Why the low-level traits: `TaffyTree` would be a second tree to keep
in step with the `Document` on every insert, move and remove. That is
two places answering one question. The traits (`TraversePartialTree`,
`LayoutPartialTree`, `CacheTree`, `LayoutFlexboxContainer`,
`LayoutGridContainer`, `LayoutBlockContainer`, `LayoutContainingBlock`,
`RoundTree`) are implemented on the arena. Taffy's
`examples/custom_tree_vec.rs` (336 lines) is the template.

- **Units.** Layout is in CSS px, as `f32`, unrounded. Paint snaps each
  edge to device pixels: `round(x * scale)`. Chromium snaps rects the
  same way, so 1px borders stay crisp at 1.5x.
- **calc().** Taffy stores a calc as an opaque pointer and resolves it
  through `resolve_calc_value(ptr, basis)`. Our calc values live in
  `Rc`s owned by the node's used style. The pointer is used only as a
  key into a side table and is never dereferenced, so no `unsafe` is
  needed.
- **Inline content.** Taffy does not lay out inline content.
  Consecutive inline children of a block container (text nodes and
  inline elements) form one inline formatting context. That context is
  a Taffy leaf whose measure function lays out one Parley layout with a
  span per inline element. A block container with both block and
  inline children wraps each inline run in an anonymous block, as CSS
  does. In a flex or grid container each text run becomes an anonymous
  item. Whitespace-only runs are dropped there, and inline elements are
  blockified. `white-space` collapsing (`normal`, `nowrap`, `pre`,
  `pre-wrap`; others refused until needed) happens before Parley,
  which does not collapse.
- **Inline-block inside text**, such as a `button` in a paragraph,
  needs Parley inline boxes sized by Taffy. It is deferred: S6 refuses
  it with that message. A button as a flex or grid item, which is how
  touche uses buttons, is blockified and works from S2.

## Elements in F2c

From brief section 5, items 1, 2 and 6, less what F4 owns (inputs and
text editing):

- Block: `div`, `p`, `h1` to `h6`, `section`, `header`, `footer`,
  `main`, `nav`, `aside`, `article`, `blockquote`, `hr`.
- Inline: `span`, `b`, `strong`, `em`, `i`, `code`, `small`, `a`
  (styling only; navigation is F4's markdown view), `br`.
- `button`: UA style, `:hover`, `:active`, `disabled` (fires no
  `click`; sets `:disabled`).
- `img`: from an image registered with the document (S10).
- `input type=checkbox` is in brief item 1. It is left to F4 with the
  other inputs unless you want it in F2c (D3).

## Paint

`vapors-scene` gains the display list:

```rust
pub enum Item {
    Quad(Quad),               // rect, background colour, per-side border widths and colours, corner radii
    Shadow(Shadow),           // rect, radii, blur, spread, colour (S3)
    Text { text: TextId, origin: [f32; 2] },  // a laid-out run held by the document
    Image { image: ImageId, rect: Rect, radii: Radii },  // S10
    PushClip(Rect), PopClip,
}
pub struct Scene { pub items: Vec<Item>, pub scale: f32 }
```

Coordinates are device pixels. `scene` keeps no dependencies: text
and images are ids resolved by the painter, so `scene` does not depend
on `vapors-text`. Paint walks the tree in CSS painting order: for each
stacking context, negative z-index children, then block backgrounds
and borders in tree order, then inline content, then positioned
descendants by z-index. Stacking contexts arrive in S8. Before S8 the
order is tree order, which is the same thing when nothing is
positioned.

`renderer::QuadPipeline` draws quads instanced, one draw per run of
consecutive quads. Its fragment shader computes coverage from a
rounded-rect signed distance and draws borders as the difference of
the outer and inner shapes. Shadows use a closed-form blurred rounded
rect (the GPUI and Evan Wallace approach). `vapors-app::ScenePainter`
walks the scene and switches between the quad pipeline and
`TextPipeline` at each change of item kind. Each run is one draw in a
single render pass. Clips are scissor rects. A rounded clip
(`overflow: hidden` with `border-radius`) is refused until a slice
needs it.

`TextPipeline::draw` currently writes its uniform and vertex buffers
and submits on each call. The painter needs several text runs per
pass, so S1 changes it to append quads to a per-frame buffer and draw
ranges of it.

Redraw: the whole window each frame (brief 4.6). Frames are requested
on demand from S2: `platform::Next` gains `Wait`, which draws again
only on input or a request. `ElementsApp` returns `Wait` when the
document has nothing dirty and no transition is running.

## Hit testing and pointer events

- The hit test walks paint order in reverse over border boxes,
  respecting clips, `visibility: hidden`, and `pointer-events: none`.
  `pointer-events` is not in the CSS table yet; S2 adds it, `auto` and
  `none` only. Inside an inline formatting context, Parley's hit test
  gives a byte offset, and the span covering it gives the inline
  element. That way `<span onclick>` inside a `<p>` is the target.
- Platform events are in physical pixels. `ElementsApp` divides by the
  scale factor of the last frame.
- `PrimaryButton` and `PointerMoved` become `pointerdown`, `pointerup`
  and `pointermove`. `click` fires on the nearest common ancestor of
  the down and up targets, as in the DOM. `Tap` becomes the same
  sequence. The S2 tests pin that a touch fires one `click`, not two:
  winit and the browser can both report a touch as mouse input too.
- Dispatch goes target first, then up through the ancestors, with
  `stop_propagation`. A `disabled` button and its descendants receive
  no `click`.
- State flags: `:hover` follows the pointer, set on the target and its
  ancestors. `:active` is set from `pointerdown` until `pointerup`. A
  flag change restyles `Cascade::restyle_scope(flag).elements(e)` only.
- `vapors_dom::Event` has no coordinates today. F2c's handlers do not
  need them. F3's drag capture adds pointer data to `Event`, which is a
  change to the `dom` crate in that slice.
- Focus and keyboard activation (Tab, Enter and Space on a button) go
  with the accessibility tree in F2d, which needs focus anyway.

## Incremental restyle and relayout

Effects call `set_text`, `set_attribute`, `insert_before` and
`remove`. Each marks dirty bits on the node:

| Change | Restyle | Relayout | Text relayout |
|---|---|---|---|
| `set_text` | none | the context's leaf and its ancestors | its inline formatting context |
| `class`, `id` or other attribute | element and subtree; plus later siblings if the sheets use `+`/`~`; whole tree if they use `:has()` | if a layout property changed | if a text property changed |
| state flag (`:hover`) | `restyle_scope(flag).elements(e)` | as above | as above |
| insert or remove | the inserted subtree; siblings when the sheets use structural pseudo-classes or sibling combinators | the parent and its ancestors | the parent's context, when inline |

- Restyle compares each new computed style with the old one. Children
  are restyled only when an inherited property changed. The changed
  properties' classes, from the `honoured` table, decide between
  relayout, text relayout and repaint only.
- Relayout clears the Taffy cache on the dirty node and its ancestors,
  which is Taffy's own `mark_dirty` rule. Each inline formatting
  context keeps its Parley layout until its text or text style changes.
- **S1 to S6 restyle and lay out the whole tree on any change.** S7
  makes it incremental. S7's tests count the elements restyled and the
  nodes laid out per change and pin the expected counts. Each count is
  proven by breaking the scope rule and watching the test fail. S7
  also measures update time for a 1,000-element tree on this machine
  against the brief's 4 ms starting budget, and records the number.

## Tests

**Unit tier, no GPU.**

- `Document` as a `Dom`: the scenarios in `crates/dom/tests` run as a
  generic suite over both `RecordingDom` and `Document`. The two
  implementations then cannot disagree about tree shape after
  `{if}`/`{for}` operations.
- Every refusal above has a test that triggers it and checks the
  message.
- **Geometry against Chromium.** Fixtures are `.wisp` files under
  `crates/elements/tests/layout/`, each with no script: a `<style>` and
  static HTML. The same file is valid HTML. `cargo xtask layout-oracle`
  wraps each fixture in a page with `@font-face` for the bundled
  fonts, loads it in headless Chromium at the fixture's viewport, waits
  for `document.fonts.ready`, and writes every element's border box
  (`getBoundingClientRect`, document order) to `<name>.boxes.json`
  beside it. The JSON is committed. The unit test compiles the fixture
  (through `build.rs`), mounts it in a `Document`, lays it out, and
  compares every box. The tolerance is 1/64 px (Chromium's layout
  unit) for fixtures without text, and is stated per fixture when text
  sizes a box. The tolerance and the reason go in the fixture's first
  line.
- The oracle is checked in CI. The `gpu` job, which already has
  Chromium, reruns `layout-oracle` into a fresh directory and fails on
  any difference from the committed JSON. That catches a fixture
  edited without regenerating, and a Chromium change. The Chromium
  process code from `vapors-text`'s comparison (deadline, stderr
  drain, stale-output removal, the `DBUS_SESSION_BUS_ADDRESS` strip)
  moves to a shared test-support module rather than being copied.

**GPU tier.**

- Offscreen through `gpu-harness`: mount a fixture, update, paint,
  render, read back. Compare with Chromium's screenshot of the same
  page at 1x and 2x:
  - Solid rects and borders at device-pixel-snapped edges: each
    region's pixels equal, per channel within 1/255.
  - Anti-aliased radius edges: per-pixel difference within a stated
    bound.
  - Text: the F1 ink-ratio band per text block.
- CI uploads the composite (ours | Chromium | difference) as an
  artefact, as F1 does for text.
- Buttons are compared with an author style and `appearance: none` on
  the Chromium side only, because Chromium's native button look is not
  a target. `appearance` is not a vapors property, so the oracle page
  adds that rule itself.

**Interaction.** `ElementsApp::event` is driven with synthetic
platform events and no device. Tests assert dispatch order, `click`
targeting, `:hover` and `:active` flags, and the signal values after.

**Web.** `scripts/check-page-contract.sh` gains a case for the
elements page: the as-built page shows the component, read as a pixel
colour inside the styled div from the screenshot, and `#error` hidden.

## Slices

Each slice is visible or usable on its own. Native and web both run
every slice from S1. The order serves touche's shell first. After S5,
touche can write its top bar, panel grid and buttons as components.

| Slice | Delivers | Visible as |
|---|---|---|
| **S1 hello box** | `vapors-elements` with the arena, `Dom`, `css::Element`, UA sheet, global sheets, whole-tree restyle; Taffy block and flex with text leaves (one style per text run); `background-color`, `padding`, `margin`, `width`/`height`, `color`, `font-size`; the `Scene` display list; `QuadPipeline` (solid rects only); `ScenePainter`; `ElementsApp`; a native example and the web page | `hello.wisp`: a styled div with padding and a sentence of text, in a window and on the phone. First geometry fixture and first Chromium pixel comparison. |
| **S2 click** | Hit testing, pointer events, `click`, bubbling, `:hover`/`:active`, `pointer-events`, refusal of unfired event names, `Next::Wait` redraw on demand | `counter.wisp` from `compiler-examples` counts clicks and taps natively and on the phone. Idle CPU near zero on desktop. |
| **S3 box model** | Borders (solid; other styles refused), `border-radius`, `box-shadow`, the UA button style | Buttons look like buttons; the pixel comparison covers radii and shadows |
| **S4 flex and grid** | Every flex and grid property in the CSS table mapped to Taffy 0.15, the `position` and alignment grammar moved to 0.15's shape, the `layout-oracle` corpus (with Taffy's generated fixtures as a source of cases) | The touche shell layout (44 px bar, canvas region, side panel) as a fixture matching Chromium at three viewport sizes |
| **S5 scoped styles** | D1: per-component `<style>`, nested components | touche's first panel as nested components, each with its own CSS |
| **S6 inline text** | Inline formatting contexts with spans, anonymous blocks, whitespace collapsing, the font properties (`font-family`, `font-weight`, `font-style`, `line-height`, `letter-spacing`, `font-feature-settings`, `text-align`), heading and paragraph margins with collapsing, background on inline elements, hit testing into spans | A paragraph with `strong`, `em` and `code`, matching Chromium's line breaks and boxes |
| **S7 incremental** | Restyle, relayout and text relayout scoped as in the table above, with counted tests; the 1,000-element update-time measurement | A counter in a large tree updates only its own text node's chain; the number is recorded |
| **S8 positioned** | `position: relative`/`absolute`/`fixed`, `z-index`, stacking contexts in paint and hit test, `overflow: hidden` (rect clip) | A toast component above the shell |
| **S9 scroll** | `overflow: scroll`/`auto`: scroll offset per element, wheel and touch pan to the innermost scrollable ancestor, offsets in whole device pixels, scrollbar painting with a UA style, clipped hit testing | A long list scrolls inside its region, on desktop and phone |
| **S10 img** | `img` from `Document::register_image(name, RgbaImage)`; `src` absent gives an empty box, as in HTML; an unregistered `src` is an error naming it; natural size and aspect ratio in layout | A thumbnail in a list row |

Deferred out of F2c, each refused when used, naming where it comes:
opacity layers and `filter` (F3, with the canvas compositing work),
transitions on elements, `transform`, inline-block inside text,
rounded clips, focus and keyboard (F2d), inputs and editing (F4),
`dialog`/popover (F5), damage tracking.

After each slice the plan is reread. The question is whether the next
slice is still the most useful one for touche, given what the last
slice showed.

## Decisions for the team lead

- **D1 Scoping.** (a) Compiler change: `mount` passes the component's
  style to the `Dom` (`dom.style_scope(STYLE)` returning a scope id)
  and tags each element it creates with the scope. Each scope's rules
  then match only elements of that scope, Svelte's rule, with every
  compound in the selector in scope. It touches `vapors-dom`, the
  compiler and the snapshots. (b) No scoping: each `STYLE` is added as
  a global sheet. That is not sumi's behaviour, and rules leak between
  components. Recommendation: (a) in S5, with S1 to S4 using global
  sheets only, since touche has no components yet. Who owns the
  compiler change: me, or the compiler's owner?
- **D2 Web page.** The web package (`vapors-web-demo`) runs the image
  app, and the page contract checks it. (a) The page picks its app from
  the query string (`?app=image`), the contract script loads both, and
  the elements app is the default. (b) A second web package, with
  `cargo xtask web` and `[web] package` taking a list. Recommendation:
  (a), which needs no factory change. `cargo xtask screenshot` has the
  same issue (`[screenshot] package = "vapors-platform"`): the elements
  example lives in `vapors-app`, so that setting needs a list or moves.
- **D3 Checkbox.** It is in brief item 1. Recommendation: F4 with the
  other inputs, unless a touche slice before F4 needs it.
- **D4 Taffy 0.15.0**, six days old, over 0.14. Recommendation as
  argued above.
