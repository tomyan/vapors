# Icons

An inline `<svg>` holding stroked `<path>`s is drawn as an icon: a
replaced box of its `width` and `height`, its paths stroked in the
element's `color`, rasterised at device scale into the glyph atlas.
The markup is a subset of HTML's inline SVG, so the same fixture
renders in Chromium and the paint oracle compares the two. It is the
primitive touche's top bar needs (touche H1); the subset is what
touche's icons use, and nothing else is drawn approximately.

```html
<svg viewBox="0 0 16 16" width="16" height="16" aria-hidden="true">
  <path d="M5 3L2 6l3 3M2 6h7a4 4 0 0 1 0 8H6" fill="none"
        stroke="currentColor" stroke-width="1.5"/>
</svg>
```

## The subset

Anything outside it is refused before layout, naming the element, the
attribute and the fix.

`<svg>`:

- `viewBox`: required, four numbers `min-x min-y width height`, width
  and height above 0.
- `width`, `height`: required, positive numbers with no unit (CSS px).
  They are the box's natural size. Their ratio must equal the
  viewBox's; a different ratio would letterbox under
  `preserveAspectRatio`, which is not built.
- `aria-hidden="true"` (a decorative icon: not in the accessibility
  tree, as in Chromium, so a button holding it keeps its own name), or
  `role="img"` with a non-empty `aria-label` (a named image). One of
  the two is required.
- `id`, `class`, and `xmlns` with the SVG namespace, which is ignored.
- Children: `<path>` elements and whitespace only.
- CSS: sized by its attributes only, so `width`, `height`, `min-*` and
  `max-*` in CSS are refused (in Chromium the attributes are
  presentation hints that CSS overrides one axis at a time, which the
  natural-size route does not reproduce). Inline is refused as for
  `img`: give it `display: block` or make it a flex or grid item.
  touche's ask-bar svgs are inline today (`#tools-open`,
  `#conversation-expand`, `#ask-mic`, `#ask-voice`, `#ask-voice-mute`,
  `#ask-send`), so touche's CSS gives them `display: block`, as it does
  for Dictate's rect.
  `color`, `margin`, `display`, `visibility` and `opacity` apply as to
  any box.

`<path>`:

- `d`: required. Commands `M L H V A Z`, absolute and relative, with
  implicit repeats (`l4 4-4 4`), SVG number syntax (signs, decimals,
  exponents, `1.5.5` as two numbers), and arc flags written `0` or `1`,
  packed or spaced. Malformed data is refused with the offset of the
  first character not understood, naming the fix. Only whitespace may
  follow a command letter, so `M,1 1` is refused as in Chromium. A
  number that does not fit a 32-bit float is refused. An arc with a
  zero radius is drawn as a line to its end point, as SVG says.
  Curves (`C S Q T`) are not in the subset.
- `fill="none"`: required (SVG's default fill is black).
- `stroke="currentColor"`: required.
- `stroke-width`: a positive number in viewBox units; 1 when absent, as
  in SVG.
- Nothing else: no `class`, `style`, `transform`, caps or joins. Joins
  are miter with limit 4 and caps butt, SVG's defaults.

touche's Dictate icon also uses `<rect rx>`; it is written as a path
when that button moves, rather than widening the subset.

## Layout

The `svg` is a replaced box like `img` (`layout::replaced`), its natural
size the `width` and `height` attributes. Its `path` children are not
boxes.

## Paint

- `vapors-text` parses path data strictly (`IconPath`) into zeno
  commands; the same parser answers the elements' refusal check, so a
  path that passed the check rasterises.
- The element's content box is snapped to whole device pixels, as an
  image's is. The icon is rasterised at that device-pixel size: the
  viewBox is scaled into it by the smaller of device width / viewBox
  width and device height / viewBox height, and centred, as SVG's
  default `preserveAspectRatio` (`xMidYMid meet`) does. The ratios
  are equal by the `width`/`height` rule until snapping to device
  pixels changes the box's, and the uniform scale keeps strokes from
  stretching then. Each path of the
  `<svg>` is stroked by zeno into its own 8-bit coverage mask, placed
  in the atlas's coverage page, beside glyphs and evicted with them.
- zeno 0.3.3 bevels every join sharper than 90 degrees whatever the
  miter limit ([dfrg/zeno#29](https://github.com/dfrg/zeno/issues/29));
  touche's speaker icon has a 53-degree join. `icon_miter.rs` adds the
  missing wedge for joins between 90 degrees and SVG's limit of 4, so
  they miter as in Chromium. Retire it when zeno fixes the issue.
- One atlas entry per path. Its key is the path's `d` and
  `stroke-width`, the viewBox, and the device size in pixels: one path
  at one size and scale is rasterised once.
- `Item::Icon { icon, rect, colour }` in the display list, one per
  `<svg>`; the painter draws it through the text pipeline as one quad
  per path of the atlas coverage tinted by the element's `color`, in
  document order. Chromium paints each path separately, so where two
  paths of a translucent colour overlap the alpha compounds
  (`a + a(1 - a)`, 0.75 for two paths at 0.5); one quad per path gives
  the same. Text's gamma and contrast boost are glyph-specific and not
  applied to icons; the blend is confirmed by the paint oracle.
- Built in 1b: `vapors_text::Icon` (view box and `IconStroke`s, checked:
  at least one stroke, a view box with area, positive stroke widths) and
  `Icon::rasterize_stroke` (`Icon::rasterize` combines the paths
  source-over, what an opaque colour paints).
  `GlyphAtlas::icon(icon, stroke, size)` refuses a path
  larger than a page (`AtlasError::IconTooLarge`) before rasterising.
  `TextPipeline::draw_icons(target, atlas, &[IconDraw])` takes the
  snapped content box in device pixels and draws with `ICON_BLEND`
  (gamma 1, contrast 0), which the text shader makes a plain blend.

## Slices

| PR | Content |
|---|---|
| 1 | This doc; `vapors-text`: `IconPath` (strict parse, refusals) |
| 1b | Path rasterisation into the atlas, `TextPipeline` drawing an icon quad; unit and GPU tests |
| 2 | Compiler vocabulary (`svg`, `path`); elements: the refusals, replaced layout, accessibility, `Item::Icon` in paint and the painter |
| 3 | A paint fixture of touche's icons compared with Chromium at 1x and 2x, and an accessibility fixture against the oracle |
