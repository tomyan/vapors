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
- **calc().** As built in S1: a computed `calc()` that still holds a %
  term (every other unit is resolved by the cascade) is refused, naming
  S4; the side table below arrives with S4's flex and grid mapping.
  Taffy stores a calc as an opaque pointer and resolves it
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

`TextPipeline::draw` writes its uniform and vertex buffers and
submits on each call, and so does `QuadPipeline::draw`, once per run
of quads. As built in S1 the painter keeps that: each batch is its
own submit, which is correct because writes and submits are ordered
on the queue. Batching into one pass is deferred to S7, where frame
time is measured; it changes nothing a screenshot can see.

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
- **Geometry against Chromium.** As built in S1: fixtures are `.wisp`
  files in `crates/elements-corpus/fixtures/`, each with no script: a
  `<style>` holding a `/* viewport WxH */` comment, and static HTML,
  so the same file is a page. `cargo run -p vapors-elements-corpus
  --example layout_oracle` wraps each fixture in a page with
  `@font-face` for the bundled fonts, loads it in headless Chromium,
  waits for `document.fonts.ready`, and writes the viewport the page
  measured and every element's border box (`getBoundingClientRect`,
  document order) to `<name>.boxes.json` beside it. It is an example
  rather than an xtask command because xtask is shared byte for byte
  with touche. The JSON is committed. The test compiles each fixture
  through `build.rs`, mounts it in a `Document`, lays it out at the
  recorded viewport, and compares every box within 1/64 px
  (Chromium's layout unit). A fixture may state a looser
  `/* tolerance N */` with its reason; none needs one so far, text
  included.
- The oracle is checked in CI: the `gpu` job, which has Chrome, runs
  it with `--check` and fails on any difference from the committed
  JSON. That catches a fixture edited without regenerating, and a
  Chrome that lays a fixture out differently. It uses `vapors-text`'s
  Chromium runner (deadline, stderr drain, the
  `DBUS_SESSION_BUS_ADDRESS` strip), which gained `dump_dom`.

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

**Web and native proof, every slice.** See "Demos" below.

## Slices

Each slice is visible or usable on its own and ends with a demo that
runs in a native window and on the WebGPU canvas. The web demo is a
first-class proof surface: a slice is not done until its page works on
Tom's phone and laptop over the tailnet. The order serves touche's shell first. After S5,
touche can write its top bar, panel grid and buttons as components.

| Slice | Delivers | Visible as |
|---|---|---|
| **S1 hello box** | `vapors-elements` with the arena, `Dom`, `css::Element`, UA sheet, global sheets, whole-tree restyle; Taffy block and flex with text leaves (one style per text run); `background-color`, `padding`, `margin`, `width`/`height`, `color`, `font-size`; the `Scene` display list; `QuadPipeline` (solid rects only); `ScenePainter`; `ElementsApp`; the demo set-up (`?demo=` routing, the index page, `--public-url /vapors/`, the shared `.pixels` checks, `[screenshot]` per D2) | `hello.wisp`: a styled div with padding and a sentence of text, in a window and on the phone. First geometry fixture and first Chromium pixel comparison. |
| **S2 click** | Hit testing, pointer events, `click`, bubbling, `:hover`/`:active`, `pointer-events`, refusal of unfired event names, `Next::Wait` redraw on demand | `counter.wisp` from `compiler-examples` counts clicks and taps natively and on the phone. Idle CPU near zero on desktop. |
| **S3 box model** | Borders (solid; other styles refused), `border-radius`, `box-shadow`, the UA button style | Buttons look like buttons; the pixel comparison covers radii and shadows |
| **S4 flex and grid** | Every flex and grid property in the CSS table mapped to Taffy 0.15, the `position` and alignment grammar moved to 0.15's shape, the `layout-oracle` corpus (with Taffy's generated fixtures as a source of cases) | The touche shell layout (44 px bar, canvas region, side panel) as a fixture matching Chromium at three viewport sizes |
| **S5 scoped styles** | D1: per-component `<style>`, nested components; the `style` attribute (refused until then) | touche's first panel as nested components, each with its own CSS |
| **S6 inline text** | Inline formatting contexts with spans, anonymous blocks, whitespace collapsing, the font properties (`font-family`, `font-weight`, `font-style`, `line-height`, `letter-spacing`, `font-feature-settings`, `text-align`), heading and paragraph margins with collapsing, background on inline elements, hit testing into spans | A paragraph with `strong`, `em` and `code`, matching Chromium's line breaks and boxes |
| **S7 incremental** | Restyle, relayout and text relayout scoped as in the table above, with counted tests; the 1,000-element update-time measurement | A counter in a large tree updates only its own text node's chain; the number is recorded |
| **S8 positioned** | `position: relative`/`absolute`/`fixed`, `z-index`, stacking contexts in paint and hit test, `overflow: hidden` (rect clip) | A toast component above the shell |
| **S9 scroll** | `overflow: scroll`/`auto`: scroll offset per element, wheel and touch pan to the innermost scrollable ancestor, offsets in whole device pixels, scrollbar painting with a UA style, clipped hit testing | A long list scrolls inside its region, on desktop and phone |
| **S10 img** | `img` from `Document::register_image(name, RgbaImage)`; `src` absent gives an empty box, as in HTML; an unregistered `src` is an error naming it; natural size and aspect ratio in layout | A thumbnail in a list row |

### Demos

One web package, `vapors-web-demo`, holds every slice's demo. The page
picks the demo from the query string, `?demo=<name>`. With no query it
shows an index page listing the demos, itself drawn by vapors from
S1 on. The image app stays as `?demo=image`, so its existing page
checks keep running. The team lead serves the build at `/vapors/` on
the tailnet. As built, trunk builds with `--public-url ./`, so the
page loads its script and wasm from beside itself at any path;
trunk's default `/` loads nothing under a subpath.
`check-page-contract.sh` gains a case that loads the page under
`/vapors/` and fails if the wasm does not start.

Each demo is a `.wisp` component in `crates/demos/demos/` (a library
crate, so the web page and the native example share it). The native
example `cargo run -p vapors-app --example demo -- <name>` runs the
same component in a window.

Every slice adds three checks for its demo:

- **Native screenshot test.** `cargo xtask screenshot demo <png> --
  <name>` runs the native example under Xvfb; the example saves the
  frame and checks the demo's pixels itself, exiting 1 on a miss. The
  `gpu` tier also paints every demo offscreen and checks the same
  pixels: a colour inside each styled box, and a colour just outside
  it. Where a slice has an interaction, the test
  replays it first: for S2, a synthetic click injected before the
  capture, and it checks the count text changed by its ink.
- **Web page-contract check.** The demo's URL is a case in
  `check-page-contract.sh`. The case passes when `#error` is hidden
  and empty and `#status` shows the adapter line.
- **Web pixel check.** The same script takes the page's screenshot in
  headless Chromium (WebGPU through SwiftShader, as the existing
  gradient case does). It checks the same named pixels as the native
  test. The pixel list is one file per demo,
  `crates/demos/demos/<name>.pixels`, read by both checks through the
  same code (the web check runs the `check_pixels` binary), so the
  native and web checks cannot drift apart.

| Slice | Demo (`?demo=`) | What Tom checks on the phone and laptop | Pixels checked |
|---|---|---|---|
| S1 | `hello` | A dark panel with padding and a sentence of Geist text, centred in a light page | Panel fill, page fill, ink in the text box |
| S2 | `counter` | A button whose count goes up on each click or tap, and highlights under the mouse | Button fill; after an injected click, the count text's ink differs |
| S3 | `boxes` | Cards with rounded corners, 1 px and 4 px borders, a shadow, and three button styles | Border colour on each side, fill inside a radius, shadow darkening below a card |
| S4 | `shell` | The touche shell: 44 px top bar, canvas region and side panel, which rearranges at the phone breakpoint | Bar, canvas and panel fills at the region centres, at 390 and 1280 px wide |
| S5 | `panel` | A touche-style panel made of nested components, each with its own CSS, and a rule that does not leak between them | The child's fill unchanged by the parent's same-named class |
| S6 | `text` | A paragraph with `strong`, `em` and `code`, a heading, and a span that counts its own taps | Ink in each line box; `code` background behind its text |
| S7 | `stress` | 1,000 elements, a counter that ticks every frame, and the update time printed in the demo | The counter's ink changes between two captures |
| S8 | `toasts` | Buttons that raise toasts above the shell; a toast stays until dismissed | Toast fill over the canvas region |
| S9 | `scroll` | A 200-row list that scrolls inside its region, by wheel and by finger | Row fill at the top before and after an injected scroll |
| S10 | `images` | A list of thumbnails decoded from bundled PNGs | Pixels inside a thumbnail match the source image |

Deferred out of F2c, each refused when used, naming where it comes:
opacity layers and `filter` (F3, with the canvas compositing work),
transitions on elements, `transform`, inline-block inside text,
rounded clips, focus and keyboard (F2d), inputs and editing (F4),
`dialog`/popover (F5), damage tracking.

F2d, in this order (the lead, 2026-10-25): momentum and fling for
touch scrolling, first, since phone scrolling without them feels
broken; `br` and `a` (D6); mixed inline metrics (refused since S6);
then focus and keyboard.

After each slice the plan is reread. The question is whether the next
slice is still the most useful one for touche, given what the last
slice showed.

## As built

### S1 (PR #32)

See the PR; the points above marked "As built in S1" record where it
differs from the plan.

### S2

- **Redraw on demand.** `Next::Wait` draws again only when the app
  calls `Platform::request_redraw` while handling an event, or the
  window is resized or changes scale. `ElementsApp` asks when
  `Document::take_changed` says the tree, a text, an attribute, a
  stylesheet or a pointer state changed; the image app asks when its
  view transform, image or file name changed. Idle frames, measured
  over 3 s after start-up (native: `idle_fps` under Xvfb on llvmpipe,
  which presents without vsync; web: headless Chromium 155 with
  SwiftShader, real time):

  | App | Native before | Native after | Web before | Web after |
  |---|---|---|---|---|
  | hello (elements) | 82 (27/s) | 0 | 180 (60/s) | 0 |
  | image app | 530 (177/s) | 0 | 172 (57/s) | 0 |

  The web "before" numbers were taken with the same page script before
  `scripts/web-idle-frames.sh` existed; the "after" numbers come from
  the script, which refuses a page that drew no frame or showed an
  error, so a page that never drew cannot pass as idle.
- **Pointer input.** `Document::pointer_moved`, `pointer_pressed`,
  `pointer_released`, `pointer_left` and `tapped` (CSS pixels) hit-test
  the last update, set `:hover` and `:active`, and dispatch
  `pointermove`, `pointerdown`, `pointerup` and `click` (on the nearest
  common ancestor of press and release) with bubbling. A disabled
  button gets no `click`. `pointer-events` (`auto`, `none`) is a new
  CSS longhand; `visibility` is read. A touch reaches the document only
  as its tap, so one tap is one click. `pointerenter`/`pointerleave` are
  not fired (refused like any unfired event).
- **Button.** Supported where it is a flex item (grid arrives in S4), which
  blockifies it; elsewhere it stays inline-block and is refused until
  S6. `disabled="false"` on a button is refused (on other elements HTML
  ignores `disabled`, and so does vapors), since it disables in HTML and the
  `Dom` cannot remove an attribute until F4. `cursor` moved to S3: the
  platform has no way to set it yet.
- **Page contract.** A hold that reached the server's 30 s deadline used
  to end silently and let the dump or screenshot run anyway; it now
  fails naming the page. The screenshot copies size the canvas at
  640x400 themselves and hold on a frame at that size, because headless
  Chromium lays the page out 313 px tall while it loads and resizes it
  to `--window-size` only to take the screenshot, after the hold has
  ended: a hold waiting for a 400 px frame would never end. A resize
  itself is redrawn: the `resize` case changes the canvas's CSS size
  after the first frame and holds until a frame at the new size.
  Each demo's `.pixels` file may list `click X Y` lines, made before the
  screenshot natively (by the example) and on the web (synthetic
  pointer events on the canvas).

### S3

- **Borders, radii, shadows.** Solid borders with a width and colour per
  side (`dashed`, `dotted`, `double` refused); widths snapped to device
  pixels as Blink snaps them (under one device pixel is one, otherwise
  floored), and laid out by Taffy. Circular corner radii in px; a %
  radius is refused, since it makes elliptical corners on a box that is
  not square (`9999px` makes a pill). Outer `box-shadow` lists with
  offset, blur and spread; `inset` refused. The quad shader draws a
  box's background under its border with coverage from the signed
  distance to the rounded edges; a corner join between two border
  colours is split on the diagonal, where Skia blends it. Shadows are
  a closed-form blurred rounded rectangle (Evan Wallace's method),
  not drawn under the box.
- **CSS.** vapors-css gains the per-side shorthands `border-top`,
  `border-right`, `border-bottom`, `border-left`, which it refused.
- **Cursor.** `Platform::set_cursor`; the document's cursor is the
  hovered element's `cursor` (`auto` is the arrow until text selection
  in F4, where a browser shows the text cursor over text).
- **Button and hr.** A vapors button style (1px grey border, 3px radii,
  a light fill), not Chromium's native look. `hr` is supported, drawn
  with a solid grey border where Chromium's is `inset`; its box is the
  same.
- **Paint against Chromium.** `crates/elements-corpus/tests/gpu_paint.rs`
  draws each paint fixture with vapors and with headless Chromium at 1x
  and 2x and holds the pixels to three rules (`paint_check`): where
  Chromium's 3x3 neighbourhood is one colour, ours is within 2; every
  pixel is within 96; and in each fixture element's border box, at most
  2 pixels are off by more than half the local contrast (the larger
  channel range of either image's 5x5 neighbourhood). The third rule is
  what catches a wrong radius or a 1 px shift: a misplaced edge moves a
  pixel by about the whole local contrast, anti-aliasing unlike Skia's
  by a fraction of it. Measured on `paint_boxes` (2026-10-23): ours has
  no pixel off by more than half the contrast in any element at 1x or
  2x. `gpu_paint_rules.rs` checks the rules: Chromium's own rendering
  with a radius, a shadow, a border width or a position changed must
  fail them, at both scales. The composites are uploaded by CI's gpu
  job.
- **Page controls.** The demo page's own Open button and status line
  sat over an elements demo's canvas. An elements demo now calls
  `vapors_platform::hide_page_controls()`. That hides both, and the
  page then needs no Open button. Errors still show: `#error`, or
  `#status` when the page has no `#error`, is un-hidden whenever an
  error is written to it. The page contract checks that no visible
  element other than `#error` overlaps the canvas of each elements demo
  and the index. It also checks that only the image demo refuses a page
  without the Open button.
- **Radii.** Corner radii that do not fit their box are scaled by one
  factor, the smallest of each side's length over the sum of its two
  radii (Backgrounds 3 §5.5), computed in `paint`; the shader draws each
  corner exactly. An inner corner's radius is the outer one less the
  two border widths meeting there; where those widths differ the inner
  corner is an ellipse, which vapors does not draw, so a radius with
  unequal meeting widths is refused, naming the radius declaration. A
  spread shadow's radius follows Backgrounds 3 §7.1.1, including the
  reduced growth for a radius smaller than the spread. A negative blur
  is refused.

### S4

- **Grid.** `display: grid` with `grid-template-columns`/`-rows`
  (lengths, %, `fr`, `auto`, `min-content`, `max-content`, `minmax()`,
  `repeat(<count>, ..)`), `grid-auto-columns`/`-rows`, `grid-auto-flow`
  (with `dense`), line and `span` placement, and `justify-items` and
  `justify-self` (`left`/`right` refused). vapors-css refuses line names
  and `repeat(auto-fill | auto-fit, ..)`; neither is mapped yet.
  `inline-grid` is laid out where it is blockified (a flex or grid
  item).
- **Sizing keywords.** `min-content`, `max-content` and `fit-content`
  in `width`, `height` and `flex-basis`. In `min-*` and `max-*` they
  are refused: Taffy 0.15 takes lengths or `auto` there.
- **calc() with %.** Taffy holds such a calc as an opaque handle (the
  address of the computed `Calc`, never dereferenced) and asks for its
  value against a basis; `layout/calcs.rs` answers from clones taken
  while the computed styles were alive. A handle it does not know, or a
  calc that does not evaluate, fails the update.
- **Corpus.** The oracle now lays each fixture out in an iframe of
  exactly its viewport, so a fixture may be narrower than Chromium's
  500 px minimum window and is as tall as asked (it was 313 px for a
  400 px window), and may name several viewports
  (`/* viewport 390x800 768x1024 1280x800 */`). New fixtures:
  `grid_tracks`, `sizing` (sizing keywords, calc with %, flex-basis
  content) and `shell`, touche's shell at three widths with its 600 px
  breakpoint; all match Chromium within 1/64 px. An element with no box
  compares as zeros, as `getBoundingClientRect` gives it.
- **Demo.** `shell`: the 44 px bar, the canvas, the 280 px side panel,
  and under 600 px the canvas alone; its pixels are checked at 640x400,
  390x800 and 1280x800, natively and on the web.
- **Scaled layout.** A fixture viewport may name a device scale
  (`640x400@1.5`); the oracle passes it to Chromium and refuses a page
  whose `devicePixelRatio` differs. `borders` is laid out at 1, 1.5 and
  2.625, where border widths snap to whole device pixels.
- **Web screenshots per section.** The page contract takes one
  screenshot of each demo per `at WxH` section of its `.pixels` file,
  with the window and canvas at that size (`check_pixels --viewports`
  lists them, `--clicks <file> WxH` gives a section's clicks), and
  checks it through `check_pixels`, which picks the section by the
  screenshot's size. Removing the boxes demo's `flex-wrap` fails the
  412x800 web screenshot.
- **Not in S4.** `position` and its alignment grammar stay with S8;
  `order` is not in vapors-css's table.

### S5

- **Scoped styles (D1).** A component with a `<style>` generates
  `SCOPE`, `concat!(env!("CARGO_PKG_NAME"), "/", <path>)`, where the
  path is the file's under the compiled directory's parent
  (`demos/panel.wisp`). `mount_before` calls `Dom::add_style(SCOPE,
  STYLE)`, and each element it creates gets
  `set_attribute(el, SCOPE_ATTRIBUTE, SCOPE)` (`data-wisp`, defined once
  in vapors-dom). The Document adds a scope's sheet once. Each of its
  selectors requires the scope:
  - The subject compound requires it as an attribute selector, so a
    scoped rule gains one attribute of specificity.
  - Every other compound, and the compounds inside `:is`, `:not`,
    `:where` and `:has`, require it inside `:where()`, which adds none.
    This is Svelte 5's rule.

  A scoped sheet that does not parse is refused by every update, naming
  its scope. The generated output changed: the compiler snapshot and
  the counter's HTML now carry the scope.
- **`:global(...)`.** vapors-css parses it like `:is()`. In a scoped
  style, the compound holding it is left unscoped, so a component
  writes `:global(body) { ... }`. In a sheet added with
  `add_stylesheet` it is refused, naming the line. The demos style
  `html` and `body` this way, and `vapors_demos::document` no longer
  adds a global sheet.
- **Global builds.** `compile_dir_global` compiles components whose
  `STYLE` the app adds as a sheet, with nothing scoped. The layout
  corpus uses it, so its CSS means what Chromium reads. A scoped copy
  would be one attribute more specific than the same rule in Chromium.
- **The `style` attribute.** vapors-css reads an element's `style`
  attribute as author declarations above any selector's specificity
  (Cascade 4 section 6.2). An important sheet declaration still beats a
  normal inline one. The Document parses every `style` attribute before
  styling. An attribute that does not parse is refused, naming the
  element, line and column. A refused value from one is located as
  `style attribute:line:column`.
- **Demo.** `panel`: a panel and three `PanelRow` children, each with its
  own CSS. Both use a `.title` class, and the panel's `.panel .title`
  (red) is more specific than a row's `.title` (green dots). The pixel
  checks hold the dots green at 640x400 and 390x800. With the subject
  compound left unscoped, the dots turn red.
- **Not in S5.** `@keyframes` names stay global; `:global { ... }` blocks
  and `:global` inside `:is()` are not supported.

### S6

- **Inline runs.** A block's consecutive inline content (text, and
  `display: inline` elements, nested) is one inline run: one string,
  with white space collapsed across element boundaries, and each inline
  element's byte range and text style. Taffy lays a run out as one
  anonymous block. Parley lays it out with each element's style over
  its range.
- **Inline boxes.** Each inline element has one fragment per line it is
  on (`Layout::fragments`, without the space a line breaks after).
  - A fragment is the element's content area, placed as Chromium places
    it: the line's half-leading over the block's font, the block's
    ascent, less the element's ascent. Every ascent and descent is
    rounded to a device pixel.
  - The fragments' union is the element's border box.
  - Each fragment takes the pointer, and is filled with the element's
    background before the text.

  `inline_text.wisp` and `text_align.wisp` match Chromium within
  0.1 px. Chromium sums advances in 1/64 px, Parley in floats.
- **Text properties.**
  - Read now: `font-style` (italic and oblique), `letter-spacing` and
    `text-align` (left to right only).
  - The bundled faces have no italic. vapors draws Skia's synthetic
    slant (`SYNTHETIC_SLANT`, a quarter), as Chromium does.
  - `bolder` and `lighter` compute to numbers from the inherited
    weight (CSS Fonts 4's table, in vapors-css). `b` and `strong` are
    `bolder`, as in Chromium's UA sheet.
  - Deferred to a slice after F2c: `font-feature-settings`,
    `font-variation-settings`, `text-decoration` and families by name.
- **Tags.** `span`, `b`, `strong`, `em`, `i`, `var`, `code`, `kbd`,
  `samp`, `mark` and `abbr` are implemented.
  - `small` waits: Chromium resolves `smaller` through a keyword table.
  - `br` and `a` come first after F2c (D6). `del`, `s`, `sub`, `sup`
    and `u` are not scheduled.
  - Generic `monospace` is the bundled Geist Mono (D5).
  - Chromium's 13 px default for `code` under a `medium` font size is
    not copied.
- **Refused, naming the element:**
  - an inline element whose font size, content area or line height
    differs from its block's. Chromium's line baseline is the maximum
    over the strut and every inline box of its rounded ascent plus
    half-leading, and vapors places inline boxes on the block's strut.
    A line mixing metrics comes in a slice after F2c. Weight and face
    alone are laid out: the bundled faces share their vertical metrics.
  - text from a right-to-left script, or a right-to-left mark. Runs are
    laid out left to right (Parley's base direction set to Ltr).
  - a block inside an inline element;
  - a margin, padding or border on an inline element;
  - an atomic inline (`inline-block`) outside a flex or grid container.

  A grid container now blockifies its children, as a flex container
  does.
- **Demo.** `text`: a heading and one paragraph of bold, italic and
  code that wraps, with a span that counts its own taps and turns
  yellow once tapped. Its pixel checks click the span at 640x400 and
  390x800. With spans out of the hit list, the span stays white and the
  check fails.

### S7

- **What is kept between updates:**
  - each element's computed and used style, and the initial style
    (`restyle.rs`, `restyle_pass.rs`);
  - each Parley layout, keyed by its run and width
    (`layout/text_cache.rs`);
  - each box's Taffy cache and layout (`layout/keep.rs`).

  An update restyles what the changes since the last one marked; the
  table above gives the scopes.
  - A pointer flag on a subject compound restyles the element alone
    (`Cascade::restyle_scope_of_selectors`, new in vapors-css). Its
    children are restyled only when its computed style changed, which
    covers what they inherit.
  - Layout is kept by diffing the rebuilt box tree against the last
    one, not by marks. A box keeps its cache only when it and its whole
    subtree are unchanged, at the same scale.
- **Counted tests:**
  - `elements_restyled`, `text_layouts_built` and `boxes_laid_out` are
    pinned per kind of change (`incremental_*.rs`), each proven by
    breaking its rule.
  - After five kinds of change, every element's box equals a fresh
    document's given the same changes.
  - A seeded randomized differential test (`incremental_random.rs`)
    makes 40 random changes per seed: classes, attributes,
    `display: none` by the style attribute, text, insert, move, remove,
    hover, press, resize, and sheets and scoped styles added. Its CSS
    uses inherited properties, `var()`, `+` and `~`, `:nth-child`,
    `:last-child`, `:empty`, an ancestor attribute, `:hover` on a
    non-subject compound, `@media`, and calc with %, em and vw.
    - After every change, each settled frame's boxes, computed styles
      and display list must equal those of an update from scratch
      (`Document::forget_kept`).
    - CI runs 6 seeds; `VAPORS_INCREMENTAL_SEEDS` runs more and
      `VAPORS_INCREMENTAL_SEED` one. 200 pass.
    - Six broken scope rules each fail it.
- **The 1,000-element measurement** (`update_time`, release, CPU only).
  Setup: 1,001 elements, one counter text changed per update.
  Machine: this box's i5-10500T, with a load average of 9 to 20 from
  the other agents. Median update time:

  | Point | Median |
  |---|---|
  | Before S7 | about 60 ms |
  | Styles kept | 4.4 to 5.6 ms |
  | Layout kept as well | 2.6 to 3.1 ms, p95 2.9 to 3.9 ms |

  The remaining time is whole-tree walks: the tree check, the `style`
  attribute check, the box build and diff, and paint.
- **What those numbers say about the phone.** They show which phases
  scale with the tree, and that keeping work removes them on the CPU
  vapors runs on here. They do not give the phone's time:
  - the Pixel 9 Pro's cores differ;
  - the web build is wasm, usually slower than native;
  - this box's load inflates and scatters them.

  The `stress` demo prints each frame's measured update time, so the
  phone's number comes from opening `?demo=stress` on the phone. Under
  headless Chromium's virtual time, the page contract's screenshot
  shows 0.00 ms: its clock does not advance during an update.
- **Demo.** `stress`: 1,000 elements under a counter that ticks every
  frame, through `ElementsApp::with_per_frame`, a hook called before
  each update with the last one's duration.
  - `?demo=stress&ticks=N` stops it after N frames, so the page
    contract's headless page goes idle.
  - `gpu_stress.rs` checks that the counter's ink changes between two
    frames.

### S8

- **Position** (`style/position.rs`): `relative`, `absolute` and
  `fixed`, with `top`, `right`, `bottom` and `left` in px and %, as
  Taffy 0.15 takes them.
  - Taffy places an out-of-flow box against the box that hoists it.
    `LayoutBox.hoisted` records that box, and `place` takes its corner
    as the origin. A fixed box is hoisted to the root, which spans the
    viewport.
  - `sticky` is refused, naming a slice after F2c (S9 built scrolling
    without it). A non-static position on an inline
    element is refused: it has no box of its own.
  - An out-of-flow box never keeps its Taffy cache between updates
    (`keep.rs`). Seed 3 of the randomized test found the bug that
    keeping it caused.
- **Paint order** (`paint_order.rs`): CSS 2 Appendix E, reduced. Each
  stacking context paints, in this order:
  1. its root;
  2. its negative z-index layers;
  3. its in-flow descendants in tree order;
  4. its z-index auto and 0 layers in tree order;
  5. its positive layers.

  The layers are sorted with a stable sort. Hit testing reads the
  painted order backwards.
  - A positioned box with z-index auto paints whole in place, and its
    positioned descendants join the enclosing context. A box with a
    z-index roots a context, and so does every fixed box.
  - z-index applies to positioned boxes and to flex and grid items. On
    any other static box it is ignored, as in CSS.
  - One difference from CSS: within the in-flow descendants each box
    paints its background and then its children, where CSS paints every
    background before any text.
- **Clips** (`clip.rs`, `style/overflow.rs`): `overflow: hidden` and
  `clip` cut descendants off at the padding box.
  - A box takes the clip of its containing block's content: its
    parent's when in flow, its hoisting box's when absolute, none when
    fixed.
  - The root never clips. The body clips only when the root's overflow
    is not `visible`; otherwise the body's overflow belongs to the
    viewport (CSS Overflow 3 section 3.3). touche's shell sets
    `overflow: hidden` on `body`.
  - Hits are cut to the clip.
  - Refused: `scroll` and `auto` (S9); axes that differ; a clip with a
    `border-radius`; overflow on an inline element.
- **Clips in the renderer.** The scene keeps each item's clip, and a
  batch ends where the clip changes. `Rect::scissor` rounds the clip to
  whole pixels inside the target. `Target` and `RenderTarget` gain
  `scissor`, which the quad, shadow and text pipelines set on their
  pass. The painter intersects each batch's scissor with the target's
  own, and skips a batch clipped to nothing.
- **Checked against Chromium.** These paint fixtures are compared at 1x
  and 2x:
  - `positioned` (boxes only);
  - `stacking`: z-index against tree order, negative z under in-flow
    boxes, a z-auto box lending its child, a context holding its child
    below a later sibling;
  - `clipping`: a wide child, a shadow, nested clips, absolute children
    placed inside and outside the clip, a bordered clip.

  Text is not in `clipping`: glyph edges differ from Chromium's by more
  than the paint rules allow. `gpu_text` checks text under a scissor
  instead. The paint harness takes whole scales only (its scale is a
  `u32`, and Chromium is shot at that device scale factor), so no
  fixture is compared at a fractional scale such as 1.5; snapping
  there is unchecked against Chromium.
- **Demo.** `toasts`: Save and Export raise toasts into a fixed stack
  (z-index 10) at the bottom right, over the canvas region and above
  the bar (relative, z-index 1, with a shadow). Each toast stays until
  its Dismiss button is pressed.
  - `toasts.pixels` raises three toasts at 640x400 and one at 390x800,
    and checks the toast fill over the canvas.
  - Dismissal is checked in `crates/demos/tests/toasts.rs`, which
    updates the document after every click.
  - A `.pixels` file cannot check dismissal. Its clicks are all made
    before the next frame, and a click is hit against the last painted
    frame, so a click cannot reach a toast that an earlier click in the
    same file raised. This is the same frame-later behaviour as D7.

### S9

- **Scroll containers** (`scroll.rs`, `chain.rs`, `scrolling.rs`):
  `overflow: scroll` and `auto` keep an offset per element on the
  Document, clamped to the content each update, dropped when the
  element stops scrolling, applied in whole device pixels.
  - Which boxes move with a container follows the containing block,
    by the walk S8's clips use: an absolute box placed by a box outside
    it does not move, and a fixed box never does.
  - Taffy 0.15's scrollable overflow leaves out the in-flow children's
    end margins and the end padding, which Chromium counts; the range
    is the larger of the two. The `scrolling` layout fixture
    (`data-scroll="x y"`, applied by the oracle page and the corpus
    test) showed Taffy 4 px short.
  - The page itself does not scroll: `scroll` and `auto` on html, or on
    body while html is visible, are refused.
- **Input.** `platform::Event::Scroll { dx, dy }` in CSS pixels, signed
  as the DOM's deltas. A wheel notch is `LINE_PIXELS` = 120, measured:
  one XTest button-5 press under Xvfb over a headful Chromium 155 page
  gave `deltaY` 120, `deltaMode` 0 and `scrollY` +120. The image view
  zooms one step per notch, from `dy`.
  - A wheel scrolls the innermost container under the pointer that can
    move that way, else the next one out (`Document::scroll_by`).
  - A burst of wheel events keeps one container while each comes within
    500 ms of the last (`wheel_scroll.rs`), as Chromium latches it:
    measured with XTest notches over a list inside a list, the inner
    one at its end, a notch 0.5 s after the last went nowhere and one
    0.55 s after scrolled the outer one.
  - A diagonal delta goes whole to one container, not split by axis:
    measured, a (40, 30) wheel over a sideways-only list moved it 40
    and its vertical container not at all; with the list at its end,
    the container took the 30.
  - A touch gesture keeps the first container it moves, as Chromium
    latches it (`touch_scroll.rs`); a pan past its end goes nowhere.
    The gesture lasts until the last finger lifts or is cancelled.
  - touche's pin bump changes one test,
    `crates/viewer/src/viewer_app_tests.rs:41`:
    `Event::Scroll { steps: 1.0 }` becomes
    `Event::Scroll { dx: 0.0, dy: -vapors_platform::LINE_PIXELS }`.
- **Thumbs** (`thumb.rs`, D10): overlay only, styled by
  `scrollbar-color` and `scrollbar-width`; on touch, shown only while a
  gesture scrolls. They take no pointer. A thumb paints after its
  container's descendants, positioned ones in an enclosing context
  included (`thumb_order.rs`): measured, Chromium paints both its
  classic scrollbar and its overlay thumb over a relative child that
  runs under them.
- **A scroll only repaints**: no restyle, no layout, no text layout
  (`incremental_scroll.rs`).
- **`.pixels` gains `wheel X Y DX DY`**, made by the parser, the native
  demo example and `gpu_demos` (through `vapors_demos::input`), and by
  the page contract (a pointer move, then a WheelEvent in pixels on the
  canvas, through winit and `ElementsApp`).
- **Demo.** `scroll`: two 200-row lists under a fixed bar; a wheel over
  the left one moves its top row from row 0 to row 3 (row 6 on the
  phone), while the right one stays at row 0.
- **Deferred to F2d:** momentum and fling.

### S10

- **Images** (`images.rs`): `Document::register_image(name, RgbaImage)`
  registers an image for the `img` elements whose `src` is `name`.
  `RgbaImage` is in vapors-scene now, re-exported by vapors-canvas.
  - Refused, naming the fix: a `src` naming no registered image; an
    inline `img` (an image in a line of text is inline-block content,
    not built: give it `display: block`, or put it in a flex or grid
    container); a rounded `img` (no rounded clip yet).
- **Layout** (the `images` fixture, against Chromium at both sizes): an
  `img` is a replaced box.
  - With no width or height it takes its natural width rather than
    stretching. One length sets the other through the aspect ratio.
  - Without a `src` it is 0 x 0, and margins do not collapse through
    it. The first version got both wrong; the fixture showed them.
- **Paint** (the `image_paint` fixture, against Chromium's pixels at 1x
  and 2x): the image over the content box, after the `img`'s background
  and border, under its clip.
  - `ImagePipeline::draw_rect` stretches a texture over a rect with
    linear sampling, inside the target's scissor.
  - The painter uploads each registered image once and keeps it while
    frames draw it.
- **Fixture images.** They are in `elements-corpus/fixtures/images/`.
  The oracle and the paint harness copy them beside their pages, and
  the corpus tests register them by file name.
- **Demo.** `images`: a history list with a 48 x 32 thumbnail per turn,
  from bundled 96 x 64 PNGs; the pixel checks read each quadrant's
  colour inside the thumbnails.

## Decisions (2026-10-23)

- **D1 Scoping.** A compiler change in S5: `mount` registers its
  `STYLE` as a scope and tags each element it creates; a scope's rules
  match only its own elements, with every compound of a selector in
  scope (Svelte's rule). S1 to S4 use global sheets only. The S5 pull
  request updates the compiler snapshots and says the generated output
  changed.
- **D2 Web page.** One web package with `?demo=` pages, an index page,
  served at `/vapors/` with a page-contract case for the subpath, and
  a shared `.pixels` file per demo. `[screenshot] package` becomes a
  list; xtask stays byte-identical with touche's, so touche's copy
  needs the same change.
- **D3 Checkbox.** F4, with the other inputs.
- **D4 Taffy `=0.15.0`.** The pull request that adds it records what
  0.15 changed in the areas vapors uses. cargo-deny's advisories check
  stays on. The Chromium corpus is the regression gate; a layout bug
  it catches in Taffy is reported before it is worked around.
- **D5 Monospace is Geist Mono (2026-10-23).** Generic `monospace` (and
  `ui-monospace`) is the bundled Geist Mono, the brand's monospace,
  where Chromium uses the system's (DejaVu Sans Mono on this Debian).
  Layout fixtures that compare monospace text name Geist Mono, so they
  compare like for like. Chromium's 13 px default size for monospace
  under a `medium` font size is not copied.
- **D6 `br` and `a` in F2d (2026-10-23).** Forced line breaks and links
  are the first item after F2c: touche's chat panel and About page need
  them. Until then they are refused, naming F2d.
- **D7 Hover after layout settles a frame later (2026-10-24).** An
  update that moves content under a still pointer notices the new
  hover after painting. It sets the flags and asks for another frame,
  which restyles; the hover style shows one frame later. Chromium also
  re-hovers after layout. The randomized differential test compares
  settled frames, updating until no change is pending, and fails if
  that takes more than three updates.
- **D8 Clicks in a `.pixels` file stay back to back (2026-10-24).**
  The format gains no way to wait for a frame between clicks. A check
  that needs one, such as dismissing a toast that an earlier click
  raised, is a Rust test that updates the document after each click
  (`crates/demos/tests/toasts.rs`).
- **D9 Text clipping is checked in `gpu_text` (2026-10-24).** Glyph
  edges differ from Chromium's by more than the paint fixtures' rules
  allow, so the `clipping` fixture has no text; its comment says so.
- **D10 Overlay scroll thumbs everywhere (2026-10-25).** A scroll
  container draws only a thumb, over its content at the far edge of
  its padding box, taking no layout space, on desktop and phone alike.
  This matches the Chromium oracle, which runs with
  `--hide-scrollbars`, but not desktop Chromium's 15 px classic gutter.
  An app styles the thumb with standard CSS, from its own tokens:
  `scrollbar-color: <thumb> transparent` (a track colour is refused:
  there is no track) and `scrollbar-width: auto | thin | none` (8, 4
  and 0 CSS px). touche's Scroll region pattern is
  `scrollbar-color: var(--line-strong) transparent; scrollbar-width:
  thin`. Once a finger has touched, thumbs show only while a touch
  gesture scrolls; a mouse move or wheel shows them again. Thumbs take
  no pointer: dragging one is not built.
