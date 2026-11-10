# Brief: native SVG in vapors

Status: design direction for review. Not in the plan yet; the proposed
placement is at the end. Written 2026-11-10 from a discussion between Tom
and the lead. Names of types and functions below are proposals.

## Objective

Let `.wisp` templates hold inline SVG (`<svg>`, `<g>`, `<path>`, basic
shapes, `<text>`) as elements in the vapors tree, styled by vapors' CSS,
bound to signals like any other element, and drawn by the vapors painter
in paint order with HTML content. The design must be able to grow to full
static SVG without replacing the element, style or item layers.

## Where vapors is today

- Icons (docs/icons.md, vapors #143, #144, #148, #149; the Chromium
  comparison is #151): a subset of
  `<svg>`/`<path>` (stroke only, M L H V A Z, `currentColor`), refused
  outside the subset, sized by CSS from its attributes as Chromium does.
  Each path is stroked on the CPU by zeno into a coverage mask, cached in
  the glyph atlas at device size, and drawn as one tinted quad per path
  (`TextPipeline::draw_icons`). An svg over 256 device px is refused.
  zeno 0.3.3 bevels joins under 90 degrees whatever the miter limit
  (dfrg/zeno#29); `icon_miter.rs` draws the missing corners.
- Boxes are instanced quads whose fragment shader takes coverage from the
  signed distance to a rounded rectangle (`crates/renderer/src/quads.wgsl`):
  analytic antialiasing per pixel.
- Text and icons are CPU coverage masks in an atlas, drawn as quads
  (`crates/text/src/text.wgsl`).
- `opacity` and `filter` are refused as deferred to F3 "with the canvas
  compositing work" (`crates/elements/src/style/honoured.rs`,
  `docs/F2c.md`). `transform` and rounded clips are also deferred.

The icon route cannot grow to general SVG: rasterising depends on device
scale, so nothing can move to compile time, every change re-rasterises,
and size is capped by the atlas page.

## What "full static SVG" needs

| Feature | Kind of work |
|---|---|
| Fills (nonzero, evenodd), strokes (joins, caps, miter limit, dashes) | Geometry |
| `<rect>`, `<circle>`, `<ellipse>`, `<line>`, `<polyline>`, `<polygon>`, all path commands | Geometry |
| `<g>`, `<use>`, `<defs>`, `<symbol>`, `transform` | Tree and instancing |
| Linear and radial gradients, patterns | Paint (shader) |
| `clipPath` | Clipping |
| `mask`, group `opacity`, `filter`, `mix-blend-mode` | Offscreen layers and compositing |
| `<text>`, text on a path | Glyph pipeline |
| Markers | Instanced geometry |
| `foreignObject`, SMIL animation | Out of scope; CSS animations cover motion |

Only the first three rows depend on how paths become pixels. The layer row
is the same compositing work F3 needs for HTML `opacity` and `filter`.

## Rendering routes considered

- **A. Tessellation.** Paths become triangle meshes on the CPU (lyon is the
  likely crate), drawn by the vapors painter as display-list items with the
  same batching, scissor clips and paint order as boxes and text. Static
  shapes can be tessellated at build time. Costs: antialiasing needs a
  choice (MSAA, or an edge-fringe strip that gives each outline a soft
  edge); a path whose shape changes is re-tessellated on the CPU
  (transforms and colours are not).
- **B. GPU vector renderer** (vello is the likely crate). Compute shaders
  on wgpu with analytic coverage. Strong antialiasing and large scenes.
  Costs: a second renderer beside the painter whose output is composited
  back in, like a `gpu-surface`; needs WebGPU compute, unverified on
  SwiftShader (the gate) and on the Pixel 9 Pro; a heavy dependency; as far
  as the lead knows, no SVG filters, so B does not avoid the layer work.
- **C. Extend the icon route.** Least new code; limited to small static
  graphics for the reasons above.
- **D. resvg/usvg.** Broad static SVG on the CPU into a texture, but with
  its own parser and style resolution outside vapors' DOM and CSS: no
  bindings, no cascade. Does not compose.

Closest to how vapors draws HTML: in structure, A (same painter and
display list); in pixel quality, B (analytic coverage like the box
shader). Styling is the same for every route, since it is the element and
CSS layer.

## Recommendation

Route A, built so that B can be added later without touching elements or
CSS. Three decisions make that possible, and they are taken first:

1. **A renderer-neutral display item.** Elements and CSS produce a vector
   item: path geometry (or a reference to compiled geometry), fill and
   stroke paint, transform, clip, and the layer it draws into. The painter
   consumes it. A backend (A now, B later, per item or per surface) is
   chosen in the painter.
2. **Offscreen layers as shared painter infrastructure.** One mechanism
   renders a subtree to a texture and composites it, used by HTML
   `opacity`/`filter` (F3) and SVG `mask`/`opacity`/`filter`. Built once.
3. **Antialiasing happens last, chosen against the Chromium oracle in the
   first slice.** See "Antialiasing after every transform" below. The
   candidates are MSAA and analytic coverage in the fragment shader;
   measured on touche's icons at 1x and 2x, and rotated and scaled, with the
   existing paint_check rules. If neither is close enough at 1x for
   untransformed small icons, those stay on the coverage-mask route (or move
   to B) and the item format is unchanged.

## Antialiasing after every transform

SVG and CSS allow arbitrary transforms: rotation, non-uniform scale, a zoom
past the scale content was prepared at, and in a 3D scene, perspective.
Antialiasing done before a transform is resampled by it, and edges blur or
alias. So coverage is computed per device pixel, after all transforms, at
draw time. Consequences:

- Anything that bakes coverage early is limited to untransformed content:
  the icon route's atlas masks (rasterised at one device size, placed axis
  aligned at whole device pixels), and an antialiasing fringe built on the
  CPU (its width is a screen-space quantity).
- Geometry is independent of antialiasing, so compile-time tessellation
  stays valid. The exception is curve flattening, which depends on the
  final scale: either flatten at the largest supported scale, or evaluate
  curves in the fragment shader (Loop-Blinn style), which is exact at any
  scale.
- Late options under A: MSAA, resolved by the GPU after every transform,
  with coverage resolution limited to the sample count; or analytic coverage
  in the fragment shader from screen-space derivatives (`fwidth`) of the
  distance to an edge or curve, as vapors' box shader already does. Both
  work under rotation and perspective.
- B antialiases in its 2D target after affine transforms, which is late for
  2D; under 3D perspective its output is a texture that is then mapped, so
  coverage is baked again.

## Compile time

The compiler already turns `.wisp` templates into Rust. For SVG it can:

- refuse literal `d`, `viewBox` and attributes outside the supported set at
  build time, naming file and line;
- parse literal path data once and embed the commands;
- tessellate static shapes once and embed vertex and index buffers. Curves
  are flattened at a tolerance set for the largest supported scale, so the
  mesh does not depend on the device;
- leave bound values to run time: a bound `d` is parsed and tessellated
  when it changes; bound transforms and colours are per-draw uniforms and
  need no new geometry.

## Composition

- **HTML and CSS.** The outer `<svg>` is a replaced box in CSS layout (as
  in icon slice 2a); inside it, geometry uses the viewBox coordinate
  system. Presentation attributes map to CSS properties in vapors' cascade
  (`fill`, `stroke`, `stroke-width`, `opacity`, `transform`), as Chromium
  does.
- **Hit testing and accessibility.** Pointer events land on the painted
  shape, not its bounding box. `role`, `<title>` and `aria-*` go through the
  existing accessibility projection, checked against Chromium's tree.
- **3D.** Two ways, with different antialiasing. An `<svg>` rendered to a
  texture is a material, through 3D slice 3b (a layer texture as a
  material); its coverage is baked at the texture's resolution, which suits
  distant surfaces with mipmaps but is soft up close. For crisp vectors in a
  scene, the vector item is drawn directly in the 3D pass with late
  antialiasing; a path's mesh is then ordinary geometry for flat or extruded
  shapes, under the 3D brief's shared geometry and material ownership. An `<svg>` over a `gpu-surface` is
  ordinary UI.
- **Incremental contracts** (docs/design/incremental-contracts.md). Each
  element keeps its mesh; a transform or colour change updates without
  re-tessellating; a removed element releases its geometry.

## Direction: one shape model

The renderer-neutral item (decision 1) can grow into one model for
everything vapors paints, so boxes, icons and SVG share pipelines and
antialiasing rules. Status: direction, agreed with Tom on 2026-11-10;
not a slice yet.

- **Every painted thing is a shape with coverage computed at draw time,**
  after transforms. Rounded boxes already are: the quad shader takes
  coverage from the closed-form distance to a rounded rectangle.
  Rectangles, circles, ellipses and lines also have closed forms and can
  share that shader. General paths and glyph outlines use an edge or curve
  representation evaluated in the fragment shader.
- **Whole-program compile time.** The compiler sees every template, so for
  literal geometry it can classify each shape to the cheapest exact
  primitive, deduplicate identical geometry across the app (the same icon
  in twenty places is one entry), and pack all static geometry into one
  shape table embedded in the binary. At run time an item is a shape id
  with a transform, paint and clip.
- **One analysis for every painted thing** (Tom, 2026-11-10: "Anything can
  change at runtime, but we should be able to know what through static
  analysis. Shouldn't be different"; "We should not refuse things that are
  runtime, just track them"). Boxes, paths, meshes and text are classified
  the same way. For each painted thing and each property, the compiler
  records one of three states:
  - **constant:** no binding, toggled class or state pseudo-class can change
    it. Compiled: classified, deduplicated and packed into the shape table.
  - **known variable:** a binding, a class that can be toggled or a
    `:hover`/`:focus`-style state can change it, and the compiler knows
    which. A run-time parameter of the same primitive, updated when its
    source changes.
  - **unknown:** the analysis cannot see what changes it. Not refused:
    tracked. The element is treated as able to change any property the
    source could reach, and the compiler reports each unknown with its
    file, line and cause, so the cost is visible and the developer can make
    it known.
  Classes are bound from enums, never strings (Tom, 2026-11-10), so every
  class an element can take is known at build time. A custom property
  set from Rust (`set_property`, as touche sets `--bar-height`) is known
  variable when its name is a literal: its value reaches exactly the
  properties whose `var()` reads it. Which sources, if any, remain unknown
  is for the analysis slice to list; a name computed at run time is the
  candidate.
  Run-time input such as text length, viewport size and image sizes sets
  sizes and positions; it does not change which primitive a thing is.
- **Retained layout and paint use the same analysis.** Which bindings can
  change which properties tells, at build time, which subtrees can ever need
  layout or paint again; IU3 and IU4 can take their boundaries from it.
- **Fewer pipelines.** One or two shape pipelines draw boxes, icons and SVG
  together, batched in paint order.

### Text

Text needs its own handling for ordinary reading. At body sizes, upright
and at whole device pixels, readable text depends on hinting, subpixel
positioning and the coverage and gamma treatment that Chromium's text
uses; an outline rendered as a shape with analytic coverage does not match
that by default. So:

- **Upright reading text stays on the glyph atlas,** rasterised at its final
  device size and position as today, and compared with Chromium by the
  existing text oracle.
- **Transformed text uses glyph outlines through the shape model:** text
  under rotation, non-uniform or animated scale, or in a 3D scene, where an
  atlas glyph would be resampled after its coverage was baked.
- **The switch is decided per run** from its transform: axis-aligned at a
  stable scale uses the atlas; anything else uses outlines. A run animating
  its scale can switch back to the atlas once the scale settles. The rule
  and its thresholds are measured against Chromium, not assumed.
- Open: whether large display text (headings) benefits from outlines even
  when upright, and how hinting interacts with the switch.

This is what to settle before vapors implements CSS `transform` or puts
UI into a 3D scene: one coverage strategy for boxes, shapes and text.

## Proposed slices

1. The vector display item and the antialiasing test: tessellate touche's
   ten icons with A, draw them through the item with MSAA and with shader
   coverage, compare with Chromium at 1x and 2x and under a rotation and a
   non-integer scale, beside the icon route's numbers. Decides antialiasing (point 3
   above). No template change.
2. `<path>` with fill and stroke paint from CSS in templates, through the
   item; the icon subset moves onto it and the atlas route, the 256 px cap
   and `icon_miter.rs` are retired if slice 1 allows.
3. Compile-time parse, refusal and tessellation for literal geometry.
4. Basic shapes, `<g>`, `transform`, `<use>`/`<defs>`.
5. Gradients and `clipPath`.
6. Offscreen layers (with F3): group `opacity`, `mask`, `filter`.
7. `<text>` and markers.
8. B as a second backend, if measurements of a real workload call for it.

Each slice is compared with Chromium (paint and accessibility oracles) and
refuses what it does not support, naming the element, the attribute and
the fix.

## Placement

Not needed by touche's MVP or by 3D slice 1c. Proposed: after 3D slice 3b,
whose texture-as-material route slice 1's 3D composition reuses, and
alongside F3, whose layers slice 6 shares. Tom decides the placement.

## Open questions for Tom

- Route A with the three decisions above, or another route.
- Whether the icon route is retired by slice 2 or kept for small icons.
- Placement relative to H1 to H5, IU3, F3 and the 3D slices.
- The text switch rule (atlas for upright reading text, outlines under
  transform) and its thresholds, before CSS `transform` is implemented.
