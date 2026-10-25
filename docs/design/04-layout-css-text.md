# 4. Layout, CSS and text

Status: design, 2026-10-25. Mostly built (F2a, F1, F2c S1 to S8); this
page records the model as it stands, the gaps, and the slices that
close the ones touché needs. Detail of what exists: `docs/css.md`,
`docs/text.md`, `docs/F2c.md`.

## The model

**Semantics are the web's.** The authoring language is HTML and CSS
with the browser's meaning: the cascade (origins, importance,
specificity, order), inheritance, computed and used values, the box
model, block, flex and grid layout, inline formatting contexts, CSS
painting order, and the state pseudo-classes. What vapors does not
implement it refuses by name, with the slice that brings it; it never
approximates. That is the rule every gap below follows.

**Why the web's semantics.** Two reasons, both from the brief. The
first is the oracle: headless Chromium lays out and paints the same
file, so a claim about layout or paint is checked against a browser,
not asserted by a golden. The second is authoring: touché's design
system (`design/tokens.json`, `design/patterns.md`) is written in CSS
custom properties and patterns a web designer already knows, and the
page shells touché wrote before vapors could draw them transfer.

**Pipeline.** `Document::update(viewport)` runs restyle, then layout,
then paint, and returns the refusals or a `Scene`. Each phase keeps
what it can between updates (F2c S7): computed styles per element,
Parley layouts per inline run and width, Taffy caches per box. The
incremental rules are checked by counted tests and a randomised
differential test against an update from scratch.

| Phase | Engine | Taken from | Differs in |
|---|---|---|---|
| Parse and cascade | `vapors-css`, written against sumi's corpus | CSS Syntax 3, Selectors 4, Cascade 4, Color 4 | Refuses unknown properties, values, pseudo-elements and at-rules instead of dropping them; `var()` failures are errors, not invalid-at-computed-value |
| Block, flex, grid | Taffy `=0.15.0` through its low-level traits on the arena | Taffy's algorithms, tested against Chrome's layout of generated fixtures | No `TaffyTree`; one tree answers every question. Inline content is a measured leaf |
| Inline text | Parley 0.12 with fontique and skrifa | Chromium's line box placement (half-leading, rounded ascents) | One strut per line: inline boxes with other metrics are refused until the mixed-metrics slice |
| Paint | `vapors-scene` display list; instanced quads, closed-form shadows, glyph atlas | GPUI's scene shape; Skia's gamma and contrast values; Blink's device-pixel snapping | Grayscale AA only; rounded clips refused until needed |

## What is built

Selectors through `:has()`; `var()`, `calc()`, `light-dark()`,
`@media`, `@supports`, `@keyframes` parsing; the Taffy block, flex and
grid properties including sizing keywords and `calc()` with `%`;
borders, radii, outer shadows; scoped component styles with Svelte 5's
rule and `:global()`; the `style` attribute; inline runs with spans,
white-space collapsing, synthetic italic, `bolder`; `position`,
`z-index` and stacking contexts (S8); `pointer-events`, `visibility`,
`cursor`. Geometry fixtures match Chromium within 1/64 px at three
viewports and three device scales; paint fixtures hold to three pixel
rules at 1x and 2x.

## Gaps, in the order touché meets them

Each row is refused today, by name, where the computed value is not
the initial one (`style/honoured.rs`). The column "Pulled by" names
the touché need; see `ORDER.md` for sequencing against other areas.

| Gap | Pulled by | Slice | Notes |
|---|---|---|---|
| `overflow: scroll`/`auto`, scrollbars, scroll offsets in whole device pixels, momentum on touch | The layer stack and history lists | F2c S9 (in flight) | Decisions taken 2026-10-24: `Event::Scroll { dx, dy }` in px; an overlay thumb, shown on touch only while scrolling. Momentum is not in S9: it has no oracle until a transcript is recorded from the Pixel, which is F2d's first step |
| `img` from a registered image | History thumbnails | F2c S10 | `Document::register_image`; natural size in layout |
| `transition-*` on elements | Agent edits animating controls; hover and pressed states | Area 6, slice A1 | `vapors-css` already samples transitions; the elements crate has to start, advance and retire them per frame |
| `opacity` as a layer; `filter: blur()` | Disabled controls, the before/after fade, sheets over the canvas | Area 6, A3 | Needs an offscreen target per layer in the painter |
| `transform` 2D (`translate`, `scale`, `rotate`) | Slider thumbs, FLIP reorder of cards, sheet slide-in | Area 6, A1 | Paint and hit test through the inverse; the layout box is unchanged |
| `@keyframes` driven animation | Busy indicators, progress line | Area 6, A4 | Sampler exists in `vapors-css` |
| `br`, `a` | The chat panel and About | F2d | Decided D6 |
| Mixed inline metrics (a real line baseline) | Inline code in chat at a different size | F2d | Today an inline with other metrics is refused |
| `font-feature-settings` (`tnum`), `font-variation-settings` | Readouts that must not jitter; variable weight | F2d | `vapors-text` honours tabular figures already; the CSS property is not mapped |
| `text-decoration`, `small`, `sub`/`sup`, `del`/`s`/`u` | Markdown view | F4 | |
| Right-to-left and bidi | Arabic and Hebrew in chat | F4 or later | Parley supports it; vapors pins Ltr and refuses RTL text today |
| `@container` | Cards that rearrange with the panel's width | After S9 | Needs a restyle pass after layout, as `docs/css.md` describes |
| Rounded clips (`overflow: hidden` with `border-radius`) | Thumbnails in rounded cards | S10 or area 6 A3 | A clip mask in the quad shader or a stencil |
| Elliptical radii, `inset` shadows, `dashed`/`dotted` borders | Nothing in touché | None scheduled | Refused with the reason |
| `::before`/`::after`, `@layer`, `@import`, `@font-face` | Nothing in touché | None scheduled | Generate the content as an element; fonts are registered through `FontSystem` |
| Inline-block inside a paragraph (a button in text) | Nothing in touché | None scheduled | Parley inline boxes sized by Taffy |
| `user-select`, selection painting | Copying from agent replies | F4 | With the editing model in area 3 |
| Colour emoji | Chat | F4 or later | The RGBA8 atlas page exists; nothing rasterises into it |
| Web font fallback fetching | Non-Latin scripts on the web | F6 | `FontSystem::register_fallback` is the hook |
| Large text above about 64 px | Headings on a 2x display at the biggest scale | When seen | Capped atlas size or paths |

## Decisions recorded here

- **Refuse, never approximate.** A deferred property in use fails the
  update, naming the element, the property, the value and where it was
  declared. The alternative, drawing the nearest thing, would make the
  Chromium comparison lie: a fixture would pass while the author's
  intent was dropped.
- **Layout in CSS px as `f32`, paint snapped to device pixels.** Blink's
  snapping rules for borders and rects; the geometry corpus checks the
  CSS px values, the paint corpus the snapped pixels.
- **Text is a measured leaf for Taffy, never laid out by Taffy.** Parley
  owns line breaking; Taffy asks the inline run for its size under a
  constraint. This is how Blitz and Bevy use Taffy too.
- **One tree.** `Document` implements `Dom`, `css::Element` and Taffy's
  traits on one arena. A second tree would be two places answering one
  question.
- **The user-agent sheet is a subset of the HTML rendering section**,
  with Geist in place of the platform's default face and a vapors button
  style in place of the native look. The oracle page applies
  `appearance: none` and the same button rules on the Chromium side so
  the comparison is like for like.

## Deliberately left out

- A second CSS engine (Stylo via Blitz): `vapors-css` refuses what it
  does not know, which a general engine cannot.
- Floats, multi-column, table layout, `writing-mode`, `contain`,
  `content-visibility`: no touché need; each is refused by name.
- Pixel-perfect parity with a browser's native form controls: vapors
  draws its own controls (area 3) and compares them with
  `appearance: none` equivalents.
- Subpixel (LCD) text antialiasing: breaks under transforms and over
  translucent backgrounds; macOS dropped it.

## Oracles

| Part | Oracle | How |
|---|---|---|
| Cascade, selectors, values | Chromium `getComputedStyle` | A fixture's computed styles dumped by the layout oracle beside its boxes; a subset of web-platform-tests `css/css-cascade`, `css/selectors`, `css/css-variables` testharness cases ported to Rust where they test computed values |
| Block, flex, grid geometry | Chromium `getBoundingClientRect` within 1/64 px | `crates/elements-corpus` fixtures; Taffy's own generated fixtures as a source of cases; WPT `css/css-flexbox` and `css/css-grid` reftests run as geometry comparisons where the reference is a trivially laid-out page |
| Inline text geometry | Chromium within 0.1 px | The same corpus; Chromium sums advances in 1/64 px, Parley in floats |
| Paint | Chromium screenshots at 1x and 2x under three pixel rules | `gpu_paint.rs`; the rules are themselves tested by perturbing Chromium's own rendering |
| Text ink | Chromium's mean ink per block within the band | `text_comparison`; sign-off by Tom on the comparison image |
| Incremental restyle and relayout | An update from scratch | The seeded differential test; counted tests per change kind |
| Scrolling | Chromium `scrollTop` after a synthetic wheel; pixel rows after a scroll | S9's checks; momentum has no browser oracle (Chromium's curve is platform-specific), so it is held to a stated curve and a recorded transcript |

## Slices

Already ordered by F2c and F2d; this page adds none of its own. The
rows in the gap table name them. What this page asks of the other
areas: area 6 owns transitions, transforms, opacity and keyframes on
elements; area 3 owns selection and editing; area 9 extends `transform`
to 3D and adds `perspective`, `transform-style` and
`backface-visibility`, which this page's refusal table must list until
then.
