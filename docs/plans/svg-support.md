# Full SVG support

Design only, for after the performance finish line
(`docs/plans/perf-finish-line.md`). Nothing here is built, and nothing
here changes current work. Read from `origin/main` at 9b405f9
(2026-11-12). Every number is cited or marked "to be measured"; line
counts per slice are estimates.

The plan builds on `docs/plans/one-scene.md`: its S rows (S1 to S8)
give vapors a shape table, `<path>` fill and stroke from CSS, `<rect>`
and `<circle>`, compile-time classification and the `<Decal>`. This
document is what "full SVG", as browsers implement SVG 2, needs beyond
that, item by item, and in what order.

Rules that bind every row (Tom, 2026-11-10 to 2026-11-12;
`docs/decisions/2026-11-11-analysable-sources.md`,
`docs/briefs/NATIVE_SVG_BRIEF.md`):

- no API whose effect the compiler's analysis cannot see; every
  property of every painted thing is constant or known variable;
- generated code calls a small shared runtime, never a per-use copy;
- runtime performance first: idle does nothing, and the cost of a page
  is flat in its size;
- every failure is loud: unsupported input is refused naming file and
  line, never approximated or skipped;
- correctness is judged against Chromium, through the existing oracles.

## Contents

1. What vapors supports today
2. The gap table against SVG 2 as Chromium implements it
3. Per item: technique, oracle, cost, verdict
4. Left out, and why
5. Slices
6. Decisions for Tom
7. Risks

## 1. What vapors supports today

The icon subset, `docs/icons.md`, and nothing else of SVG.

- **Elements:** `svg` and `path` are in the compiler's vocabulary
  (`crates/compiler/src/template/vocabulary.rs`); the compiler does not
  read their attributes. Every other SVG element is an unknown tag.
- **`<svg>`:** `viewBox` (four numbers, width and height above 0),
  `width` and `height` (positive CSS numbers, in the viewBox's ratio to
  a part in 10^5), one of `aria-hidden="true"` or `role="img"` with a
  non-empty `aria-label`, `id`, `class`, `xmlns` with the SVG
  namespace, and the compiler's scope attribute. Children: `path` and
  whitespace only, at least one path. Checked before layout by
  `crates/elements/src/svg_checks.rs`; the largest dimension times the
  viewport scale may not pass 256 device px (`svg_limit.rs`).
- **`<path>`:** `d` in `M L H V A Z`, absolute and relative, implicit
  repeats, SVG number syntax, arc flags packed or spaced
  (`crates/shapes/src/path.rs`, `path_lex.rs`); `fill="none"`
  and `stroke="currentColor"` required; `stroke-width` a positive
  number, 1 when absent; no other attribute
  (`crates/elements/src/svg_path_checks.rs`). Joins miter at limit 4,
  caps butt (SVG's defaults), nothing else.
- **CSS:** the svg takes `color`, `margin`, `display` and
  `visibility` as any box, and `opacity` is refused on it as on any
  box; CSS sizes on it are refused, as
  is inline display, percentage padding and a border radius; a path
  may not differ from its svg in `color`, `display` or `visibility`
  (`svg_style_checks.rs`). No SVG property exists in `vapors-css`:
  `fill`, `stroke` and `stroke-width` are attributes the element tree
  reads, not properties the cascade sees.
- **Layout:** the svg is a replaced box sized by its attributes, as
  Chromium maps them (`crates/elements/src/svg.rs`); paths are not
  boxes.
- **Paint:** each path is stroked on the CPU by zeno into an 8-bit
  mask at device size, cached in the glyph atlas, drawn as one tinted
  quad per path (`crates/elements/src/svg_paint.rs`,
  `crates/scene/src/display.rs` `Item::Icon`,
  `crates/text/src/draw_icon.rs`). Coverage is baked before any
  transform; there is no transform. zeno bevels joins under 90 degrees
  (dfrg/zeno#29); `crates/text/src/icon_miter.rs` adds the missing
  wedge.
- **Hit testing:** the svg's border box, as any box; the kept hit list
  holds `(NodeId, BoxRect)` pairs (`crates/elements/src/kept_hits.rs`);
  `pointer-events` takes `auto | none` only
  (`crates/css/src/property/table_rest.rs:63`).
- **Accessibility:** an `aria-hidden` svg is out of the tree; a
  `role="img"` svg is an `image` named by its `aria-label`, which a
  button takes into its name from content
  (`crates/elements/src/a11y_name.rs:95-100`); recorded against
  Chromium 155 in `crates/elements-corpus/fixtures/icon_names.ax.json`.
- **Oracles:** the paint comparison
  (`crates/elements-corpus/tests/gpu_paint.rs`, rules in
  `crates/elements-corpus/src/paint_check.rs`: flat regions within 2,
  every pixel within 96, at most 2 strongly-off pixels per element
  region) holds touche's ten icons against Chromium 155 at 1x and 2x
  (`fixtures/icons_paint.wisp`); the layout oracle leaves paths out of
  the boxes (`crates/elements-corpus/src/page.rs` `NOT_A_BOX`); the
  accessibility oracle (`examples/ax_oracle.rs`, DevTools) and the
  events oracle (`examples/events_oracle.rs`) take any fixture.
- **Deferred in CSS, refused when set:** `opacity` and `filter` (F3),
  `transform` and the animation and transition properties ("a slice
  after F2c"), `crates/elements/src/style/honoured.rs`. `vapors-css`
  already parses `@keyframes`, transitions and timing functions
  (`crates/css/src/animation/`); the element tree does not run them.
- **Planned, not built** (`docs/plans/one-scene.md`, S1 to S8): the
  `vapors-shapes` crate with the parser moved into it; lyon
  tessellation of fills (nonzero first) and strokes with per-vertex
  outline distances; closed-form rects and circles; `Item::Shape` and
  a `ShapeStore`; the antialiasing measurement (S3b); `<path>` fill and
  stroke from CSS with `fill`, `stroke` and `stroke-width` as
  properties; `<rect>` and `<circle>`; the compiler's per-crate
  `SHAPES` table and the process registry; `d={expr}` as known-variable
  geometry (S6); shapes in the 3D pass and `<Decal>` (S7, S8). C and Q
  curves enter the parser in S2.

## 2. The gap table

"Chromium" is Chromium 155, the oracle's version. "After S rows" is
what `one-scene.md` S1 to S8 leave built. The verdict is this plan's:
**support** (a slice in section 5), **refuse** (refused loudly at
build time or at the update, with the fix named, until a slice takes
it), **out** (section 4, refused with no slice planned). Nothing is
ever drawn approximately: every row not marked support is refused.

| Feature | Chromium | vapors today | After S rows | Verdict |
|---|---|---|---|---|
| `<rect>` (with `rx`/`ry`), `<circle>` | drawn | refused (not in the subset) | closed-form shapes, filled and stroked | support, done by S4b |
| `<ellipse>`, `<line>`, `<polyline>`, `<polygon>` | drawn | refused | not planned | support (V1): ellipse and line closed-form, poly* as paths |
| `<path>` commands `M L H V Z` | all | all | all | support, done |
| `<path>` arcs `A` | all flag forms | all flag forms, zero radius as a line | the same | support, done |
| `<path>` curves `C S Q T` | all | refused | `C` and `Q` in the parser (S2) | support: `S` and `T` in V1 |
| `fill-rule` nonzero / evenodd | both; `clip-rule` for clips | stroke only | nonzero first | support (V2): evenodd is a tessellator option |
| `fill`, `stroke` colours; `currentColor`; `none` | CSS properties, presentation attributes at specificity 0 | attributes, fixed values | properties in `vapors-css`, attributes mapped (S4a) | support, done by S4a |
| `fill-opacity`, `stroke-opacity`, per-shape `opacity` | multiplied into the paint | refused (`opacity` deferred to F3) | not planned | support (V2): a paint alpha, no layer |
| `stroke-width` | user units, scaled by the CTM | positive number | a CSS property (S4a) | support, done by S4a |
| `stroke-linejoin` miter / round / bevel; `stroke-miterlimit` | the three; `miter-clip` and `arcs` fall back to miter | miter at 4 only | miter at 4 only | support (V3) for the three; `miter-clip` and `arcs` refused |
| `stroke-linecap` butt / round / square | all | butt only | butt only | support (V3) |
| `stroke-dasharray`, `stroke-dashoffset` | drawn | refused | not planned | support (V4), literal values; a bound offset is known variable |
| `vector-effect: non-scaling-stroke` | drawn | refused | not planned | refuse until V22 |
| `paint-order` | drawn | refused | not planned | support (V2): item order |
| `<g>` | a group with inherited style and a transform | refused | not planned | support (V5) |
| `transform` attribute and CSS `transform` on SVG elements; `transform-origin` (initial `0 0` on an SVG element), `transform-box` | 2D affine; 3D functions on SVG elements flatten to 2D | refused | the item carries one 2x3 affine; the svg's viewBox transform | support (V5), 2D affine; 3D functions refused |
| `viewBox` with `preserveAspectRatio` (ten alignments, `meet`/`slice`, `none`), nested `<svg>`, `overflow` on an svg | drawn | ratio must equal the attributes'; `xMidYMid meet` implied | the same | support (V6) |
| CSS `width`/`height` on an svg; percentage sizes; `auto` sizes from the viewBox ratio | CSS overrides the attributes per axis | refused | the same | support (V6) |
| `<defs>`, `<use href="#id">` (`x`, `y`, `width`, `height`), `<symbol>` with its own viewBox | a shadow copy of the referenced subtree, styled under the `use` | refused | not planned | support (V7): compile-time expansion of a literal same-template reference; a bound or cross-template `href` refused |
| `<linearGradient>`: `x1 y1 x2 y2`, `gradientUnits`, `gradientTransform`, `spreadMethod` pad / reflect / repeat, `<stop>` with `offset`, `stop-color`, `stop-opacity`, `href` inheritance between gradients | drawn; stops interpolated in sRGB (Chromium ignores `color-interpolation`) | refused | not planned | support (V8) |
| `<radialGradient>`: `cx cy r`, `fx fy fr` | drawn, including the focal forms | refused | not planned | support (V9); `fx`/`fy`/`fr` in a second step of the same slice |
| `<pattern>` | a tile rendered and repeated | refused | not planned | refuse until V23 |
| `clip-path: url(#id)` with `<clipPath>` holding shapes, `clipPathUnits`; `clip-path: <basic-shape>` | anti-aliased clip | the clip is one scissor rect; rounded clips refused | the same | support: closed-form clips analytic (V10); path clips as device-space coverage (V12) |
| `<mask>`, `mask-type` luminance / alpha, `maskUnits`, `maskContentUnits` | an offscreen layer multiplied by the mask's luminance or alpha | refused | not planned | support (V12), on the layer mechanism |
| group `opacity` on `<g>`/`<svg>` | an offscreen layer composited at the opacity | refused (F3) | not planned | support (V11), shared with F3's HTML `opacity` |
| `mix-blend-mode`, `isolation` on SVG elements | blended through a layer | refused | not planned | out for now (section 4) |
| `filter` with `<filter>` primitives, `filterUnits`, `primitiveUnits`, filter region, `color-interpolation-filters` (initial `linearRGB`), and CSS filter functions | all 17 SVG 1.1 primitives | refused (F3) | not planned | support a set (V13, V14); the rest out (section 4) |
| `<marker>`, `marker-start`/`-mid`/`-end`, `orient` (angle, `auto`, `auto-start-reverse`), `markerUnits`, `refX`/`refY`, `markerWidth`/`markerHeight`, its own `viewBox` | drawn per vertex | refused | not planned | support (V15) |
| `<text>`, `<tspan>`: `x y dx dy rotate`, `text-anchor`, `dominant-baseline`, `font-*` from CSS, `fill`/`stroke` on glyphs | laid out by the text engine | refused (a text child is refused) | not planned; the brief's slice 7 | support (V16) upright; (V17) transformed and `textLength` refused |
| `<textPath>` | glyphs placed along a path | refused | not planned | support (V17), after V16 |
| `<image>` inside an svg | an image at `x y width height` under `preserveAspectRatio` | refused | not planned | support (V20) through the image pipeline |
| `<a>` inside an svg | a link | refused | not planned | out for now (section 4) |
| `<switch>`, `systemLanguage`, `requiredExtensions` | conditional processing | refused | not planned | out |
| `<foreignObject>` | HTML laid out inside the svg | refused | not planned | out |
| `<script>` in svg | runs | refused | not planned | out |
| `<style>` inside an svg | a sheet | refused | not planned | out: a component's `<style>` is the one sheet |
| `<title>`, `<desc>` | the svg's accessible name and description; a shape's tooltip and name | refused as a child | not planned | support (V19) |
| `<metadata>` | ignored | refused | not planned | support (V19): accepted and dropped, which is exact, since it paints nothing |
| SMIL: `<animate>`, `<animateTransform>`, `<animateMotion>`, `<set>`, `<mpath>` | runs | refused | not planned | out: CSS animations and transitions instead (V21) |
| CSS transitions and animations on `fill`, `stroke`, `opacity`, `stroke-dashoffset`, `transform` | run | refused (deferred) | not planned | support (V21), on the CSS animation slice |
| CSS `d: path(..)` and its interpolation | run | no `d` property | `d={expr}` as known-variable geometry (S6) | refuse: `d` stays an attribute, bound through `d={expr}` |
| `pointer-events` on shapes: `visiblePainted` (initial), `visibleFill`, `visibleStroke`, `visible`, `painted`, `fill`, `stroke`, `all`, `none`; `bounding-box` | hit by the painted shape per the keyword | `auto | none`, hit by the box | the same | support (V18) |
| Accessibility of shapes and groups: `role`, `aria-*`, `<title>` naming, `<text>` as static text, the svg root's role | as the `ax_oracle` records it | `aria-hidden` and `role="img"` + `aria-label` on the root only | the same | support (V19), each form recorded by the oracle |
| `shape-rendering`, `text-rendering`, `image-rendering` hints | hints | refused | not planned | out: coverage is the renderer's policy |
| `color-interpolation` | ignored by Chromium | refused | not planned | out |
| Namespaced attributes (`xlink:href`, `xml:space`) | `xlink:href` as a deprecated alias of `href` | refused by the compiler (`codegen/attribute_refusals.rs`, `NAMESPACES`) | the same | out: write `href` |

## 3. Per item

Each item: what the shape table (`one-scene.md` section 1) already
covers; the GPU technique, with the alternatives weighed; the oracle
that judges it; the cost in code, memory and per-frame work; the
verdict. "Build time" means the compiler, in `build.rs` through
`compile_dir` (`crates/compiler/src/build/mod.rs`), over literal
values; "run time" means the known-variable path, tracked and counted.

Three techniques recur, so they are named once:

- **Closed form.** The fragment shader has a signed distance to the
  shape's edge and takes coverage from it with `fwidth`, as
  `crates/renderer/src/quads.wgsl` does for rounded boxes. Exact under
  any affine transform when the distance is evaluated in the shape's
  own space and scaled by the pixel footprint, which `fwidth` of the
  distance gives. Works for rectangles, rounded rectangles, circles,
  ellipses (the implicit function over its gradient's length, exact to
  first order at the edge, which one-pixel coverage needs) and stroked
  line segments (a capsule or a rotated rectangle).
- **Tessellation.** lyon turns a path into triangles at build time for
  literal geometry; each vertex carries its distance to the outline,
  and the fragment shader takes coverage from that distance with
  `fwidth`, after the transform (one-scene 1.2). S3b measures this
  against MSAA; this plan assumes S3b chose the analytic method, and
  says where MSAA would change a row.
- **Device-space coverage.** A shape drawn by either method into an
  R8 texture covering a device-space region, in the frame it is
  needed, after every transform; content draws multiply their coverage
  by its texel. This is late antialiasing, so it is within the brief's
  rule; it is what clips by arbitrary paths and masks use.

Not used, and why: **stencil-and-cover** (two passes through a stencil
buffer) gives aliased edges unless the target is multisampled, and
the paint rules bound every pixel to 96 of Chromium's, which an
aliased edge against an anti-aliased one fails; **per-shape SDF
textures** bake coverage at one resolution, early antialiasing, which
the brief rules out ("Antialiasing after every transform"); **a
compute-shader vector renderer** (vello) is the brief's route B, a
second renderer, kept as a later option and not needed by any row
below.

### 3.1 Basic shapes, path commands, fill rules

- **Table:** S2 classifies `<rect>`, `<circle>` and paths to closed
  forms or meshes; S4b draws them; `C` and `Q` enter the parser in S2.
- **Technique:** `<ellipse>` and `<line>` are closed forms (a circle
  under a non-uniform transform is an ellipse, so the circle shader
  already has to be the ellipse shader, evaluated in local space).
  `<polyline>` and `<polygon>` are paths of lines, a polygon closed.
  `S` and `T` are parser work: the reflected control point. Curves are
  flattened at build time at the entry's tolerance for its largest
  scale (one-scene 1.2); a chord approximation of a circle of radius
  `r` user units at tolerance `t` needs about `pi * sqrt(r / (2t))`
  segments, 70 for `r` 100 at `t` 0.1 px, so a flattened mesh is
  small (decision D1 is whether to evaluate curves in the shader
  instead). `fill-rule` is a tessellator option (`FillRule::EvenOdd`),
  applied at build time and part of the entry's canonical form, which
  S2 already includes; it costs nothing at run time.
- **Oracle:** a paint fixture per element, filled and stroked, at 1x
  and 2x, with the rules every fixture is held to; where Skia's own
  anti-aliasing is off exact coverage, the pixel is compared with
  exact coverage and declared, as `paint_support/exact.rs` does for
  Mute.
- **Cost:** per entry, 52 bytes per vertex (position, normal, three
  edge distances, the triangle's three edge normals) and three vertices
  per triangle with no index buffer, 6.5 times the indexed layout (one-scene 1.2, as
  S2 built it), to be measured in S5a; no new dependency beyond S2's
  lyon.
- **Verdict:** support (V1, V2).

### 3.2 Strokes: joins, caps, miter limit, dashes, non-scaling

- **Table:** S2 tessellates strokes with lyon's stroker at SVG's
  defaults (miter 4, butt); S4a gives `stroke-width` from CSS, a
  change tessellating again at run time.
- **Technique:** lyon's `StrokeOptions` take `LineJoin::{Miter,
  MiterClip, Round, Bevel}`, `LineCap::{Butt, Square, Round}` and a
  miter limit, so joins and caps are build-time options and part of
  the canonical form. `miter-clip` and `arcs` are refused: Chromium
  draws them as `miter`, and vapors does not draw one thing as
  another; the refusal names `miter`. Outline distance for stroke
  meshes: as for fills, three per-edge distances per vertex of each
  triangle, the outline edges found by their use count in lyon's
  output (one-scene 1.2, as S2 built it); exact on straight segments
  and first-order on round joins and caps. S3b's measurement decides
  whether that is within the rules; if not, round joins and caps are tessellated with
  an explicit centre vertex. Dashes: the flattened path is split by
  the dash pattern into subpaths at build time, each then stroked with
  its caps and joins, which is what Skia does (`SkDashPathEffect`
  before stroking) and so matches the oracle. lyon has no dash effect
  of its own as far as the lead knows; a walk over the flattened path
  by arc length is about 100 lines. A bound `stroke-dashoffset` (a
  marching-ants animation) is known variable geometry: the split runs
  again at run time, counted as `shapes_tessellated`, as S6 counts a
  bound `d`. A fragment-shader dash (discard by `advancement` against
  the pattern, one uniform for the offset) would make the offset free,
  but it cannot cap the dash ends, so it is not taken; it is kept as an
  option if a measurement shows the split too slow, for butt caps only.
  `vector-effect: non-scaling-stroke` means the stroke is `stroke-width`
  device pixels whatever the transform; the exact way is to transform
  the path to device space and stroke there, which is run-time
  tessellation on every transform or scale change, keyed by the
  transform. Refused until V22.
- **Oracle:** paint fixtures of each join and cap at widths 1, 1.5 and
  4, at 1x and 2x; a dashed circle and a dashed polyline with round
  caps; touche's icons stay as the regression.
- **Cost:** no new dependency; the dash walk in `vapors-shapes`; a
  dashed path has one subpath mesh per dash, so a long finely dashed
  path has many triangles (to be measured; a dashed circle of 70
  segments at a 4-unit dash is about 150 subpaths).
- **Verdict:** support (V3, V4); `non-scaling-stroke` refused until V22.

### 3.3 Paint servers: gradients and patterns

- **Table:** paint is per item, not part of the entry (one-scene 1.3).
- **Technique:** an item's paint becomes `Paint::Solid(colour)` or
  `Paint::Gradient(index)` into a gradient buffer the shape pipelines
  read (a read-only storage buffer in the fragment stage, which WebGPU
  has in core). A record holds the kind, a 2x3 matrix from the item's
  local space into gradient space, the end points or centre, radius
  and focal point, the spread method and a range into a stops array of
  offset and colour. The matrix composes `gradientUnits` (the shape's
  bounding box for `objectBoundingBox`, known at build time for literal
  geometry, recomputed with the geometry for a bound `d`),
  `gradientTransform` and the item's transform, on the CPU. The shader
  computes `t`: a projection for linear, the two-point conical
  solution for radial with a focal point (`r` and `fr` as the two
  circles); applies the spread (`pad` clamps, `reflect` folds, `repeat`
  takes the fraction); then walks the stops, interpolating colour and
  opacity in non-premultiplied sRGB, which is what Chromium does for
  SVG gradients (it ignores `color-interpolation`; the first fixture
  confirms this reading). `href` between gradients is resolved at
  build time (attributes and stops inherited from the referenced
  gradient). One `paint.wgsl` function, concatenated into both shape
  shaders: one answer to "what colour is this fragment". Stops that
  read a custom property (`stop-color: var(--brand)`) are known
  variable: a restyle rewrites the record, no geometry. Patterns: a
  tile rendered to a texture at the pattern's device scale and
  sampled with repeat is what Chromium does, and it bakes the tile's
  coverage at that resolution, which is early antialiasing: exact only
  while the pattern's device transform is unchanged, and rebuilt on
  every change. The exact alternative, instancing the tile's shapes
  over the filled region clipped to the shape, has no bound on the
  instance count. Patterns are V23, refused until then, and come with
  a rebuild-on-change rule and a count.
- **Oracle:** paint fixtures: a linear gradient at three angles with
  each spread method, a radial gradient with and without a focal
  point, a gradient under `gradientTransform` and under a rotated
  shape. Chromium dithers gradients (Blink sets Skia's dither flag on
  gradient paints), which the rules absorb: a gradient's neighbourhood
  is not flat, so rule 1 does not fire, and dither is under the
  strongly-off floor of 3; to be confirmed by the first fixture.
- **Cost:** about 80 bytes per gradient record and 20 per stop in the
  shape table for literal gradients; the `paint.wgsl` function in two
  pipelines; no new dependency.
- **Verdict:** support (V8, V9); patterns refused until V23.

### 3.4 Clipping, masks, group opacity: layers

- **Table:** nothing. Today the clip is one scissor rectangle per item
  (`crates/scene/src/display.rs` `Scene::set_clip`), rounded clips are
  refused (`svg_style_checks.rs`), and `opacity` is deferred to F3.
- **Technique, closed-form clips:** a `clip-path` whose `<clipPath>`
  holds one rectangle, rounded rectangle, circle or ellipse, or a
  `<basic-shape>` of those, becomes a clip descriptor (kind, 2x3,
  parameters) in a per-draw uniform that every pipeline reads (quads,
  shadows, text, images, shapes): coverage is multiplied by the clip's
  closed-form coverage, through one shared `clip.wgsl` function. Up
  to two such clips per item; a third nested closed-form clip goes
  through the coverage route below once V12 lands, counted; until
  then it is refused naming V12. The same uniform gives HTML
  `overflow: hidden` with `border-radius`, refused today, with no
  further work.
- **Technique, path clips and masks:** device-space coverage. The
  clip's shapes (with `clip-rule`) are drawn into an R8 texture over
  the clipped items' device bounds, intersected with the clip's; each
  clipped item's draw samples it. A mask is the same with an RGBA
  target, its luminance (for `mask-type: luminance`; the coefficients
  and whether Chromium takes them in linear or sRGB are read from the
  first fixture, not assumed) or alpha multiplied in. Under IU4's
  retained GPU paint the texture is kept between frames and rendered
  again only when the clip, the mask or the transform under them
  changes, counted as `coverage_textures_rendered`; idle renders none.
- **Technique, group opacity:** an offscreen RGBA layer over the
  group's device bounds (intersected with the clip), the group's items
  drawn into it, then one composite draw at the opacity. The layer is
  at whole device pixels and never resampled, so compositing adds no
  antialiasing error. The compiler can prove a layer unnecessary: a
  group whose draws are one (one shape, fill only or stroke only) is
  composited exactly by multiplying the alpha into the paint, which is
  what Skia does for a single-draw layer. Anything else takes the
  layer; a layer texture pool holds them; a change inside a group
  renders that layer again, the rest of the frame kept, counted as
  `layers_rendered`. `fill-opacity`, `stroke-opacity` and `opacity` on
  a single shape are paint alphas, no layer. This is the one layer
  mechanism the brief asks for (decision 2), and F3's HTML `opacity`
  and `filter` land on it (decision D3).
- **Oracle:** paint fixtures: a rounded-rect clip on an image and on
  text; a star-shaped clip (evenodd) on a gradient; a luminance mask
  and an alpha mask; a translucent group of two overlapping circles
  (the overlap must not double); the same group with a single circle
  (no layer, counted).
- **Cost:** the clip uniform and function in five pipelines; a layer
  and coverage pool in the painter (about 300 lines); per layer, its
  device bounds times 4 bytes of GPU memory while it lives, 1 byte per
  pixel for a coverage texture. MSAA, had S3b chosen it, would make
  every layer and coverage target multisampled with a resolve each.
- **Verdict:** support (V10, V11, V12).

### 3.5 Filters

The SVG 1.1 primitives, all of which Chromium implements, and what
this plan says of each:

| Primitive | Realistic on the GPU | Verdict |
|---|---|---|
| `feGaussianBlur` | yes: separable, two passes, `stdDeviation` per axis scaled by the CTM; a large sigma by downsample, blur, upsample | V13 |
| `feOffset`, `feFlood`, `feMerge` | yes: trivial passes | V13 |
| `feComposite` (`over`, `in`, `out`, `atop`, `xor`, `arithmetic`) | yes: one pass over two inputs | V13 |
| `feBlend` (the 16 modes) | yes: one pass | V13 |
| `feColorMatrix` (`matrix`, `saturate`, `hueRotate`, `luminanceToAlpha`) | yes: one pass | V13 |
| `feDropShadow` | yes: the blur, offset, flood and composite above | V13 |
| `feComponentTransfer` (`identity`, `table`, `discrete`, `linear`, `gamma` per channel) | yes: a small table per channel | V14 |
| `feMorphology` (`erode`, `dilate`) | yes: separable min or max over the radius | V14 |
| `feConvolveMatrix` | yes for small orders; `edgeMode`, `divisor`, `bias`, `preserveAlpha` | V14 |
| `feTurbulence` | computable (the spec gives the algorithm), but per pixel per frame and niche | out |
| `feDiffuseLighting`, `feSpecularLighting` with `feDistantLight`, `fePointLight`, `feSpotLight` | computable, niche | out |
| `feDisplacementMap` | computable, niche | out |
| `feImage` | needs an external or referenced image as an input | out |
| `feTile` | niche | out |
| Inputs `BackgroundImage`, `BackgroundAlpha`, `FillPaint`, `StrokePaint` | Chromium does not implement them (removed in SVG 2) | refused |

- **Table:** nothing.
- **Technique:** a filter is a chain of passes over the element's
  layer (3.4), each primitive one full-region draw reading the named
  inputs (`SourceGraphic`, `SourceAlpha`, a previous `result`) from
  pooled textures. The working format is `rgba16float` (a core WebGPU
  render target), in linearRGB, since `color-interpolation-filters`
  is `linearRGB` by default and 8-bit linear would band; `sRGB` on a
  primitive converts at its edges. The filter region
  (`x y width height`, default -10% / 120% of the bounding box,
  `filterUnits`, `primitiveUnits` and per-primitive subregions) sizes
  the layer. CSS filter functions (`blur()`, `drop-shadow()`,
  `grayscale()`, ...) are the spec's equivalent primitive chains, so
  F3's HTML `filter` is the same passes.
- **Oracle:** paint fixtures per primitive. Chromium's blur is Skia's:
  a Gaussian kernel at small sigma and three box blurs above a
  threshold, so the soft region may differ from an exact Gaussian by
  more than rule 1's 2 levels where Chromium's neighbourhood is within
  1; the first fixture measures it, and if it fails, vapors
  implements Skia's triple-box above the same threshold rather than
  widening the rule.
- **Cost:** one pass per primitive, two for a blur, per filtered
  element per change; 8 bytes per pixel per live layer; shaders for
  about ten primitives (one WGSL file each, under 100 lines). Idle
  renders nothing.
- **Verdict:** support the two sets (V13, V14); the rest out
  (section 4), refused naming the primitive.

### 3.6 Markers

- **Table:** a marker's content is shapes, deduplicated as any.
- **Technique:** a marker is an instance of its content at each
  vertex of the path (segment ends, not flattened points) with a
  transform: the vertex position, the orientation (`orient`: an angle,
  `auto` from the bisector of the incoming and outgoing tangents at a
  mid vertex and the tangent at an end, `auto-start-reverse`), the
  scale (`markerUnits="strokeWidth"` multiplies by the stroke width),
  `refX`/`refY`, and the marker's own `viewBox` and
  `preserveAspectRatio`. A marker clips to its viewport by default
  (`overflow: hidden`), a closed-form rectangle clip (3.4). For a
  literal path every vertex and tangent is known at build time, so
  the instances are constant items; for a bound `d` they are
  recomputed with the geometry.
- **Oracle:** paint fixtures: arrowheads at the ends and joins of a
  polyline with acute and obtuse angles, `auto-start-reverse`, a
  marker on a curve's end (tangent from the control point), scaled by
  stroke width.
- **Cost:** items only.
- **Verdict:** support (V15).

### 3.7 `<defs>`, `<use>`, `<symbol>`

- **Table:** the same geometry in two places is one entry (one-scene
  1.3), so an expanded `<use>` costs no geometry.
- **Technique:** compile-time expansion. `<use href="#id" x y width
  height>` with a literal `href` naming an element in the same
  template is replaced by a copy of that element's subtree under a
  group translated by `(x, y)`; a `<symbol>` adds the viewport
  transform its `width`, `height`, `viewBox` and `preserveAspectRatio`
  give, with a rectangle clip. Each copied element is a template
  element with its own `StyleKey` (its site includes the `<use>`'s
  site, so two uses of one symbol are two keys), and the copy inherits
  from the `<use>`, as SVG's shadow-tree model says. Where the model
  and a plain copy differ: SVG matches document rules against the
  referenced element in its original place (its ancestors under
  `<defs>`), while the copy sits under the `<use>`. The compiler
  refuses the difference: a conditional rule whose match depends on
  an ancestor compound reaching an element under `<defs>` is refused
  at build time naming the rule and the fix (style it by a class on
  the element). Refused: a bound `href={expr}` (the target is not
  analysable), an `href` into another file (ids are document-global
  and a component mounts anywhere), a cycle, a reference to nothing.
  The alternative, one element drawn many times (true instancing in
  the tree), would give one `StyleKey` for every instance while each
  inherits a different `fill` from its `<use>`, and so was not taken
  (decision D4).
- **Oracle:** paint fixtures: one symbol used three times with three
  `fill`s on the uses; the accessibility oracle for what Chromium puts
  in the tree for a use instance (to be read, not assumed).
- **Cost:** one element per instance in the tree (property 8's bound
  per element applies); no geometry.
- **Verdict:** support (V7).

### 3.8 Transforms, `viewBox`, `preserveAspectRatio`, sizing

- **Table:** the item's transform is one 2x3 affine (one-scene 1.4).
- **Technique:** an element's local transform (the `transform`
  attribute's list of `matrix`, `translate`, `scale`, `rotate`,
  `skewX`, `skewY`, or CSS `transform` with `transform-origin`, whose
  initial value on an SVG element is `0 0`, and `transform-box`,
  initial `view-box`) composes with its ancestors' inside the svg;
  that product is folded at build time when every transform on the
  chain is literal, and stored in the item. The svg's own
  viewBox-to-content-box transform depends on layout, so it is a
  per-svg uniform (translate and scale) applied in the vertex shader;
  a layout shift of the svg (IU3 3c-3e) rewrites that uniform, not
  the items. A known-variable transform (bound, or in a toggled class)
  recomputes its subtree's products on change, O(subtree), and
  rewrites those items' instance bytes, counted
  `shape_transforms_written`; nothing is tessellated. 3D transform
  functions on SVG elements are refused. CSS `transform` stays
  deferred on HTML elements (its layout, stacking and hit-testing
  work is its own slice): the element tree refuses it off an SVG
  element, as `svg_style_checks.rs` refuses CSS sizes on an svg
  today. `preserveAspectRatio`: `meet` takes the smaller scale,
  `slice` the larger with the overflow clipped to the viewport (the
  svg's content box is axis-aligned, so today's scissor is the clip;
  a nested svg under a rotation takes a closed-form rectangle clip),
  `none` scales each axis, and the ten alignments place the viewBox;
  `overflow: visible` lifts the clip. CSS `width` and `height` on an
  svg override its attributes per axis, `auto` on one axis follows the
  viewBox ratio, and both `auto` without attributes give Chromium's
  default replaced size; `crates/elements/tests/svg_size.rs` holds
  Chromium's numbers today and grows with each form.
- **Oracle:** the layout oracle for every sizing form; paint fixtures
  for a rotated and a skewed shape (what they check is coverage after
  the transform), each alignment of `preserveAspectRatio` with `meet` and
  `slice`, and a nested svg.
- **Cost:** 24 bytes per item for the matrix (already in S3a1's
  layout); one uniform per svg.
- **Verdict:** support (V5, V6).

### 3.9 Text: `<text>`, `<tspan>`, `<textPath>`

- **Table:** glyph outlines through the shape model are the brief's
  "Text" section; nothing is built.
- **Technique:** V16, upright: a `<text>` is shaped by parley as a run
  in the font CSS gives it, placed by `x y dx dy` (per glyph where
  lists are given), `text-anchor` (the run's advance), and
  `dominant-baseline`; its paint is `fill`, not `color`; a `stroke` on
  text is refused until V17. It draws through the glyph atlas when the
  svg's total transform is axis-aligned with a stable scale, the
  brief's rule, at the device size the scale gives (the atlas already
  keys by device size). V17, transformed: glyph outlines from skrifa
  tessellated at run time into entries keyed by font, glyph and
  tolerance, cached, drawn as shape items with per-glyph transforms;
  `<textPath>` places each glyph along the path's arc length with its
  rotation, which is always the transformed case. The switch's
  thresholds are measured against Chromium, as the brief says.
  `textLength` and `lengthAdjust` are refused.
- **Oracle:** the text comparison's measures
  (`crates/text/src/comparison/measure.rs`) for upright text; paint
  fixtures for transformed text and text on a path; the accessibility
  oracle for `<text>` as static text.
- **Cost:** the tessellator in the runtime behind the feature (3.13),
  since outlines are tessellated at run time; a glyph mesh cache.
- **Verdict:** support (V16, V17).

### 3.10 Hit testing

- **Table:** entries hold the mesh or the closed form, which the hit
  test reads.
- **Technique:** the kept hit list (`crates/elements/src/kept_hits.rs`,
  `(NodeId, BoxRect)` today) gains a shape variant: the entry, the
  item's transform, and which of fill and stroke count, from
  `pointer-events` (`visiblePainted` by default: fill where `fill` is
  not `none`, stroke where `stroke` is not `none`, and only while
  visible; the other keywords as the property defines them;
  `bounding-box` is the box test). A point is inverse-transformed into
  the shape's space (a singular transform hits nothing, as in
  Chromium) and tested: closed forms analytically, meshes by
  point-in-triangle over the tessellated fill or stroke mesh after a
  bounds test, so what is hit is what is painted, which differs from
  Chromium's test on the path itself only by the flattening tolerance.
  Order is paint order, last first, as today. The svg element's own
  box stays a hit for the svg, as a replaced box. Until V18 a listener,
  `bind:`, or a `:hover`, `:active` or `:focus` rule on an element
  inside an svg is refused naming V18, so no event is delivered by a
  box test that Chromium would deliver by the shape.
- **Oracle:** the events oracle (`crates/elements-corpus/examples/
  events_oracle.rs`, `.events` scripts): clicks inside a shape, in the
  hole of an evenodd ring, on a stroke and just off it, on a hidden
  shape, under each `pointer-events` keyword; Chromium's target is the
  expectation.
- **Cost:** a bounds rect per shape in the kept list; the triangle test
  only on shapes whose bounds hold the point.
- **Verdict:** support (V18).

### 3.11 Accessibility

- **Table:** nothing.
- **Technique:** the projection (`crates/elements/src/a11y_*.rs`) takes
  each SVG form the oracle records: a `<title>` as the name of its
  parent (the svg root, or a shape with a role), `<desc>` as the
  description, `<text>` as static text, `role` and `aria-*` on shapes
  and groups as on any element, the svg root's own role when it has
  none. What Chromium 155 puts in the tree for each is read from
  `ax_oracle`, not assumed; today's fixture covers only `aria-hidden`
  and `role="img"` with `aria-label`. An SVG form the projection has
  no recorded answer for is refused at the update naming the form, as
  the icon subset refuses a root with neither attribute today.
- **Oracle:** `ax_oracle` over a fixture per form, `--check` in CI.
- **Cost:** projection code per form; no run-time cost beyond today's
  mirror (R6).
- **Verdict:** support (V19).

### 3.12 CSS styling and presentation attributes

- **Table:** S4a adds `fill`, `stroke` and `stroke-width` to
  `vapors-css` and maps the attributes.
- **Technique:** each slice adds its properties to `vapors-css`'s
  table and to `honoured.rs`'s `TABLE`, whose test holds the two
  together, so a property is refused until the slice that reads it:
  `fill-rule`, `fill-opacity`, `stroke-opacity`, `stroke-linecap`,
  `stroke-linejoin`, `stroke-miterlimit`, `stroke-dasharray`,
  `stroke-dashoffset`, `paint-order`, `vector-effect`, `clip-path`,
  `clip-rule`, `mask`, `mask-type`, `marker-start`, `marker-mid`,
  `marker-end`, `stop-color`, `stop-opacity`, `flood-color`,
  `flood-opacity`, `text-anchor`, `dominant-baseline`,
  `color-interpolation-filters`, `transform-origin`, `transform-box`,
  and `pointer-events`' SVG keywords. Presentation attributes enter
  the cascade as author declarations ranked below every author rule
  (specificity 0, before the sheets), as Chromium ranks them, through
  the route S4a builds. A bound presentation attribute
  (`fill={expr}`) is refused: its run-time value could name a paint
  server the analysis cannot see; paint changes go through `class:`
  toggles and custom properties, the forms the analysable-sources
  decision names. A `url(#id)` in `fill`, `stroke`, `clip-path`,
  `mask`, `marker-*` or `filter` must resolve at build time to an
  element in the same template, whether written in an attribute, a
  rule, or a custom property declared literally in a sheet; a custom
  property bound from the template may carry a colour, length or
  number, never a reference, and the compiler refuses a `var()` in a
  reference position whose property is bound. Inheritance of the SVG
  properties and `currentColor` are `vapors-css`'s existing machinery.
- **Oracle:** the paint fixtures of each slice, written with the
  property in a rule, in an attribute, and in both (the rule wins).
- **Cost:** property entries; the analysis extends to the new
  properties as the build-time styles plan extends it to every
  longhand (`docs/plans/build-time-styles.md`, section 1).
- **Verdict:** support, per slice.

### 3.13 Bloat: where the code goes

Three places, each bounded on its own:

- **The compiler** takes `vapors-shapes` with lyon (parse, classify,
  tessellate, dash, expand `<use>`, fold transforms, resolve
  references). Build-time only; its size does not reach the binary.
- **The runtime** takes `vapors-shapes` without the tessellator unless
  the crate feature `runtime-shapes` is on; the feature is required
  by a bound `d`, a bound `stroke-dashoffset`, `non-scaling-stroke`
  and SVG text outlines, and a template using one of them in a crate
  without the feature fails the build naming the feature. A page of
  literal SVG ships no tessellator. lyon's size on wasm is to be
  measured in S2 (`cargo xtask web` prints the wasm size); the brief's
  rule stands that a size budget moves only with a measurement.
- **The painter** gains the shape pipelines (S3a), `paint.wgsl` and
  `clip.wgsl` shared by every pipeline, the layer and coverage pools,
  and one small shader per filter primitive. Each is one copy; nothing
  is generated per use.

Per-page memory: an item is about 64 bytes (entry id, 2x3, paint,
clip); a layer or coverage texture is its device area times 4 or 1
bytes while it lives; an entry is its vertices. Per-frame work on an
idle page is zero: every item, layer, coverage texture and record is
kept until something it depends on changes, and each change is
counted.

## 4. Left out, and why

Each is refused at build time naming the element or property and, where
one exists, the form to use instead. None has a slice; a later plan can
add one.

| Left out | Why |
|---|---|
| SMIL (`<animate>`, `<animateTransform>`, `<animateMotion>`, `<set>`, `<mpath>`) | A second animation system beside CSS animations, which `vapors-css` already parses. `<animate attributeName="d">` morphs geometry per frame, re-tessellating every frame. CSS transitions and animations on paint, opacity and transform (V21) cover motion; the refusal names them. |
| `<foreignObject>` | HTML laid out inside an SVG viewport under its transform: HTML under `transform` is the deferred HTML `transform` slice, not SVG work. |
| `<script>`, `<style>` inside an svg, `<a>`, `<switch>`, `systemLanguage`, `requiredExtensions` | Document-level features: a `.wisp` component's script and `<style>` are the one place for code and rules; links and conditional processing have no vapors form yet. |
| `xlink:*`, `xml:*` attributes | Already refused by the compiler (`codegen/attribute_refusals.rs`); `href` is the SVG 2 form. |
| `feTurbulence`, lighting, `feDisplacementMap`, `feImage`, `feTile`; the `BackgroundImage`, `FillPaint` and `StrokePaint` inputs | Section 3.5: niche or not implemented by Chromium. |
| `mix-blend-mode`, `isolation` | They need the layer mechanism (3.4) and a blend pass with the backdrop, the same pass `feBlend` uses; cheap after V11 and V13 but no user yet. Added when something needs it. |
| `stroke-linejoin: miter-clip | arcs` | Chromium draws them as `miter`; vapors does not draw one thing as another. |
| CSS `d: path()` | Geometry is bound through `d={expr}` (S6); a second route to the same parameter would be a second thing for the analysis to track. |
| `shape-rendering`, `text-rendering`, `image-rendering`, `color-interpolation` | Hints a renderer may ignore; Chromium ignores `color-interpolation`. Coverage policy is the renderer's, and a hint accepted and ignored would be a silent approximation. |
| `textLength`, `lengthAdjust`, `<altGlyph>`, SVG fonts, `enable-background` | Removed or deprecated in SVG 2, or niche. |
| External references (`url(file.svg#id)`), `<image>` of an external SVG | A reference the compiler cannot see at build time. |
| `<mesh>` gradients, `<hatch>` | Not in any browser. |

## 5. Slices

Each slice is one PR under about 400 lines (estimate), built by a
fresh agent, each check proven by breaking what it guards. Every slice
shows something drawn that the previous could not, and is compared
with Chromium through the oracle its row names. Ordered by value: what
real SVG files need most first (icon sets and logos are shapes,
strokes with round caps and joins, groups, transforms and `<use>`;
then gradients and clips; text, masks, filters and patterns are
rarer). The S rows of `one-scene.md` come first; V1 starts after S6.

| PR | Content | Accepted when | Lines | Depends on |
|---|---|---|---|---|
| V1 | `<ellipse>`, `<line>` (closed forms), `<polyline>`, `<polygon>`; `S` and `T` in the parser | Paint fixture of each, filled and stroked, at 1x and 2x; `layout_oracle` boxes unchanged; a `points` list with an odd count refused naming the element and line | 350 | S6 |
| V2 | `fill-rule: evenodd`, `fill-opacity`, `stroke-opacity`, per-shape `opacity`, `paint-order`; the properties in `vapors-css` and `honoured.rs` | A ring drawn by `evenodd` has a hole; a translucent fill over a stroke compounds as Chromium's; `paint-order: stroke` draws the stroke under the fill; `opacity` on a `<g>` still refused naming V11 | 300 | V1 |
| V3 | `stroke-linejoin` (miter, round, bevel), `stroke-linecap` (butt, round, square), `stroke-miterlimit`; `miter-clip` and `arcs` refused | Fixtures of each join and cap at widths 1, 1.5, 4 at 1x and 2x pass the rules; the canonical form distinguishes joins (two hashes) | 350 | V2, S3b's method |
| V4 | `stroke-dasharray`, `stroke-dashoffset`, literal; a bound offset through `runtime-shapes` | A dashed circle and polyline with round caps pass; a bound offset change tessellates that shape once, counted, others 0 | 350 | V3 |
| V5 | `<g>`; the `transform` attribute and CSS `transform` with `transform-origin` and `transform-box` on SVG elements; build-time folding; a known-variable transform rewriting its subtree, counted; `transform` on an HTML element still refused | A rotated and a skewed shape pass at 1x and 2x; a class toggle moving a group writes N items' bytes (counted) and tessellates nothing; `rotate3d()` refused naming the fix | 400 | V2 |
| V6 | `preserveAspectRatio`, `overflow` on the svg, nested `<svg>`, CSS `width`/`height`/`auto` on an svg; the ratio refusal retired | `layout_oracle` holds Chromium's boxes for each sizing form; a fixture of the ten alignments with `meet` and `slice` passes | 400 | V5 |
| V7 | `<defs>`, `<use>` with a literal same-template `href`, `<symbol>`; compile-time expansion; the refusals (bound `href`, cross-file, cycle, missing target, ancestor-dependent rule under `<defs>`) | One symbol used three times with three fills on the uses passes; the shape table has one entry (snapshot); each refusal fails the build naming file and line | 400 | V6, S5a |
| V18 | Hit testing on painted shapes; `pointer-events`' SVG keywords; the refusal of listeners and state rules inside an svg retired | Events-oracle fixture: clicks inside, in an evenodd hole, on and just off a stroke, on a hidden shape, under each keyword, target Chromium's; a page of 1000 shapes hovers in time flat in shape count (`--scaling`) | 400 | V7 |
| V8 | `<linearGradient>`: units, transform, spread, stops, `href` inheritance; `paint.wgsl` | Three angles times three spreads, under `gradientTransform` and on a rotated shape, pass; the dither reading recorded | 400 | V7 |
| V9 | `<radialGradient>` with `fx`, `fy`, `fr` | Fixtures with and without a focal point pass | 250 | V8 |
| V10 | Closed-form `clip-path` (`<clipPath>` with one rect, rounded rect, circle or ellipse; `<basic-shape>` of those); `clip.wgsl` in every pipeline; HTML `overflow: hidden` with `border-radius` through it | A rounded clip on an image, text and a gradient pass; two nested clips analytic, a third through V12 refused until V12 naming it | 400 | V9 |
| V11 | The layer mechanism: group `opacity` on `<g>` and `<svg>`; the single-draw proof; the pool; `layers_rendered` | Two overlapping translucent circles in a group do not double; the single-circle group counts 0 layers; idle 0; a change inside a group renders 1 | 400 | V10 |
| V12 | Device-space coverage: path `clip-path` with `clip-rule`, `<mask>` with `mask-type`, units; `coverage_textures_rendered` | A star clip on a gradient, a luminance and an alpha mask pass; the luminance coefficients recorded from the fixture; idle 0 | 400 | V11 |
| V19 | Accessibility: `<title>`, `<desc>`, `<text>` as static text, roles on shapes and groups, the root's role, `<metadata>` dropped | `ax_oracle` fixtures per form match; an unrecorded form refused at the update naming it | 300 | V7 |
| V16 | `<text>`, `<tspan>` upright through the atlas; `x y dx dy rotate`, `text-anchor`, `dominant-baseline`; `fill` on text | The text comparison's measures hold on an SVG text fixture at 1x and 2x; `stroke` on text refused naming V17 | 400 | V6 |
| V17 | Text through outlines under a transform; `<textPath>`; the switch rule and thresholds measured | Rotated text and text on a circle pass the paint rules; the switch's thresholds recorded in `docs/text.md` | 400 | V16, V5 |
| V13 | Filters, first set: `feGaussianBlur`, `feOffset`, `feFlood`, `feMerge`, `feComposite`, `feBlend`, `feColorMatrix`, `feDropShadow`; the region; linearRGB in `rgba16float`; CSS filter functions mapped | A fixture per primitive passes, or the blur's reading is recorded and Skia's triple-box adopted (3.5); idle 0 | 400 (two PRs if over) | V11 |
| V14 | Filters, second set: `feComponentTransfer`, `feMorphology`, `feConvolveMatrix` | A fixture per primitive passes | 350 | V13 |
| V15 | Markers | Arrowheads at ends and joins, `auto-start-reverse`, `strokeWidth` units pass | 350 | V7 |
| V20 | `<image>` inside an svg through the image pipeline, under `preserveAspectRatio` and the transform | An image on a rotated svg passes; a missing image refused as `img`'s is | 250 | V6 |
| V21 | CSS transitions and animations on `fill`, `stroke`, `opacity`, `stroke-dashoffset`, `transform` of SVG elements | A transition writes paint or transform bytes per frame and tessellates nothing except a dash offset (counted); the CSS animation slice's own oracle | 300 | the CSS animation slice ("a slice after F2c"), V5 |
| V22 | `vector-effect: non-scaling-stroke` through run-time tessellation keyed by the transform | A 1-px stroke stays 1 px under a 4x scale, compared with Chromium; counted per transform change | 300 | V5, `runtime-shapes` |
| V23 | `<pattern>`: a tile rendered at the pattern's device scale, rebuilt on change, counted | A fixture passes while the transform is unchanged; a scale change rebuilds once | 400 | V12 |

Critical path for icon sets and logos: V1 to V7 (seven PRs). V18 and
V19 close the "element in the tree" promise. V8 to V12 are the paint
and compositing batch; V13 to V17 and V20 to V23 follow by demand.

## 6. Decisions for Tom

**Decided 2026-11-13 (Tom): D1 to D10 all as recommended.** D1 (a) flatten at build time, tolerance per entry; D2 (a) closed forms analytic, paths and masks through a coverage texture (S2's shapes antialias by a half-pixel outline extrusion, #306); D3 layers built in V11; D4 (a) `<use>` by compile-time expansion with the ancestor-rule refusal; D5 the two filter sets; D6 (a) atlas upright, outlines under a transform; D7 SMIL out; D8 (a) hit testing by the painted shape; D9 (a) bound presentation attributes refused; D10 (a) after property 9, V1 to V7 first.

**D1. Curves: flattened at build time, or evaluated in the shader.**
(a) Flatten at build time at a tolerance for the entry's largest
scale, recorded per entry; a draw past that scale tessellates again at
run time at a finer tolerance, counted, and a debug check names it.
(b) Loop-Blinn style: curve segments as triangles whose fragment shader
evaluates the implicit curve, exact at any scale; cubics need splitting
at inflections and loops; strokes of curves still need an offset
curve, which is flattened anyway. (c) Both, (b) for fills only.
Recommendation: (a). Every row above works with it, the cost of a
mesh is small (3.1), and S3b's decal-at-zoom measurement says whether
(b) is ever needed. Changing to (b) later touches the tessellator and
one shader, not the elements, the table format or the oracles.

**D2. Clips and masks by device-space coverage, not stencil or MSAA.**
(a) Closed forms analytic; paths and masks through a coverage texture
(3.4). (b) A stencil buffer for path clips: no texture, but aliased
edges unless the whole target is multisampled. (c) MSAA for
everything, if S3b chooses it for shapes: then (b) becomes viable and
the coverage route is unnecessary. Recommendation: (a) under S3b's
analytic choice; if S3b chooses MSAA, (c), and V10 and V12 are
re-planned before they start.

**D3. The layer mechanism is built here (V11) and F3 lands on it, or
F3 builds it and V11 waits.** Recommendation: build it in V11. SVG
group opacity has a fixture that needs it and nothing else; F3's HTML
`opacity` and `filter` then add the HTML side only. If F3 is scheduled
first, V11 becomes the SVG half of F3's slice.

**D4. `<use>` by compile-time expansion, with the ancestor-rule
refusal (3.7).** (a) Expansion: one element per instance, exact
inheritance from the `<use>`, geometry deduplicated, rules depending
on ancestors under `<defs>` refused. (b) Run-time instancing: one
element drawn many times; one `StyleKey` for instances that inherit
different paints, so the shared-style analysis does not hold. (c)
Expansion without the refusal, matching rules against the original
place at run time: a second matcher context. Recommendation: (a).

**D5. Filters: the two sets in 3.5, the rest out.** The alternative
is all 17 primitives, about 7 more shaders, for `feTurbulence`,
lighting, displacement, `feImage` and `feTile`, with no user.
Recommendation: the two sets; a primitive is added when a page needs
it.

**D6. Text in SVG follows the brief's rule: atlas when upright at a
stable scale, outlines under a transform.** (a) As the brief, with
thresholds measured in V17. (b) Outlines always for SVG text:
simpler, but body-size text in an svg then differs from HTML text
beside it. (c) Atlas always, resampling under transforms: early
antialiasing, which the brief rules out. Recommendation: (a).

**D7. SMIL is out; CSS animations carry motion.** Recommendation: out,
as section 4. The alternative, SMIL on paint and transform only (no
`d`), is a second timeline beside CSS's for the same properties.

**D8. Hit testing by the painted shape.** (a) The tessellated mesh and
closed forms (3.10), exact to what is painted. (b) The bounding box,
as today: cheaper, wrong for any non-rectangular shape, and the events
oracle would fail on the first click beside a circle. Recommendation:
(a).

**D9. Bound presentation attributes are refused (3.12).** (a) Refuse
`fill={expr}` and the others: paint changes through `class:` toggles
and custom properties. (b) Allow a bound colour-valued attribute,
refusing only references: the compiler cannot see the value's kind,
so it would be a run-time refusal. Recommendation: (a), the same
answer as `style={..}`.

**D10. Placement.** (a) After the finish line's property 9, V1 to V7
as the first SVG batch, then V18 and V19, then by demand. (b) After
the comparative benchmark and the website, which the finish line
says wait for the properties only. Recommendation: (a), since the
benchmark and the site need no SVG beyond icons; Tom decides the
order against the website's needs.

## 7. Risks

- **S3b's method.** Every row assumes analytic coverage on
  tessellated meshes passes the paint rules. If S3b finds it does not
  (the brief's decision 3 returns the question to Tom), V3's round
  joins and V17's outlines are the rows most exposed, and D2 changes.
- **Chromium's own approximations.** Skia's blur above its threshold
  (3.5), gradient dither (3.3), the Mute-crossing pixels already
  declared in `exact.rs`: each may need an exact-coverage declaration
  or a matched implementation. The rule is to match Chromium where it
  is exact and declare where it is not, never to widen the paint
  rules.
- **The `<use>` refusal.** Real SVG files from editors put ids and
  classes under `<defs>` and style them from the root; the
  ancestor-rule refusal (3.7) will fire on some of them, and the fix
  (a class on the element) is editing the file. Measured by running
  the compiler over a corpus of exported icons before V7 starts; if
  the hit rate is high, D4 (c) is costed.
- **Text thresholds.** The atlas-or-outlines switch (D6) is measured,
  not known; if no threshold keeps both sides within the rules, SVG
  text under slow zoom will show a visible switch.
- **Memory of expanded uses.** A symbol used 1,000 times is 1,000
  elements; property 8 bounds each, and the Mount workload's
  per-element figure applies. An icon sprite sheet would be better
  served by components, which the docs say.
- **Per-page counts.** Each slice adds counters (`shapes_tessellated`,
  `shape_transforms_written`, `layers_rendered`,
  `coverage_textures_rendered`); `vapors-perf --scaling` judges them
  flat only if a workload exercises them, so V5, V11 and V12 each add
  a workload or extend one.
- **wasm size.** lyon in the runtime (behind `runtime-shapes`) and the
  filter shaders are the two additions whose size is unmeasured; the
  first measurement is S2's, and the feature keeps a literal-only
  page's size where it was.
- **Chromium version drift.** Every oracle fixture records Chromium
  155; a Chromium update that changes an SVG pixel or AX node fails
  `--check`, which is the intended loud failure, and the fixtures are
  re-read, not relaxed.
