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
  viewBox is scaled into it (device width / viewBox width, and the
  same for height; the ratios are equal by the `width`/`height`
  rule) after translating by `-min-x`, `-min-y`. Every path of the
  `<svg>` is stroked by zeno into one coverage mask, and that
  8-bit mask is placed in the atlas's coverage page, beside glyphs
  and evicted with them.
- One atlas entry per `<svg>`, however many paths it has. Its key is
  every path's `d` and `stroke-width` in document order, the viewBox,
  and the device size in pixels: one icon at one size and scale is
  rasterised once.
- `Item::Icon { icon, rect, colour }` in the display list, one per
  `<svg>`; the painter draws it through the text pipeline as one quad
  of the atlas coverage tinted by the element's `color`. Text's gamma and contrast
  boost are glyph-specific and not applied to icons; the blend is
  confirmed by the paint oracle.

## Slices

| PR | Content |
|---|---|
| 1 | This doc; `vapors-text`: `IconPath` (strict parse, refusals) |
| 1b | Path rasterisation into the atlas, `TextPipeline` drawing an icon quad; unit and GPU tests |
| 2 | Compiler vocabulary (`svg`, `path`); elements: the refusals, replaced layout, accessibility, `Item::Icon` in paint and the painter |
| 3 | A paint fixture of touche's icons compared with Chromium at 1x and 2x, and an accessibility fixture against the oracle |
