# CSS for 3D scene nodes

Design only, for after the performance finish line
(`docs/plans/perf-finish-line.md`). Nothing here is built, and nothing
here changes current work. Written 2026-11-12 from `origin/main` at
9b405f9 and the open PRs #250 to #252 (3D slice 2 row 4,
`docs/plans/3d-2.md`). Where a number is not cited it is marked "to be
measured".

The rules this design is written under (Tom, 2026-11-10 and -11):

- No API whose effect the compiler's build-time analysis cannot see
  (`docs/decisions/2026-11-11-analysable-sources.md`): classes are
  toggled by booleans or chosen by enums, never strings; a custom
  property changes only through a binding with a literal name.
- Generated code calls a small shared runtime; nothing is copied per
  use.
- Runtime performance first: idle does nothing; an edit's cost is flat
  in scene size; a transform change writes one uniform.
- Every failure is loud: a property that does not apply to a scene node
  is refused naming the file and line, never ignored.

## Summary

- Today no CSS reaches a scene node. After PR 4b as `3d-2.md` row 4b
  words it, each node's anchor element feeds its computed `color`,
  `opacity` and `visibility` to its node, at run time only. Three
  things stand in 4b's way on main: `opacity` is refused on every
  element (`honoured.rs`, `Deferred("F3")`); a component tag has no
  `StyleKey` and no `class:` directive, so an anchor is unkeyed and
  invisible to the analysis; and nothing routes a pointer from a
  `gpu-surface` to a node (section 0).
- The properties that map onto a scene node are the appearance set
  (`color`, `opacity`, `visibility`, `pointer-events`, `cursor`,
  custom properties, `transition-*` and `animation-*` over those),
  state pseudo-classes through a 3D hit test, and the cascade's own
  machinery (selectors over the anchor tree, inheritance from HTML
  ancestors, `@media`), which costs nothing extra because anchors are
  elements (section 1). CSS `transform` and its 3D family, the box
  model, `z-index` and `display` do not map, and are refused naming the
  prop or the mechanism to use instead (section 3).
- The build-time fit is one addition to `build-time-styles.md`'s
  classification: a per-tag property set for `scene`, `group`, `mesh`,
  `camera` and `decal`, checked against each scene key's winners at
  build time; a run-time `SceneUsed` of five fields in place of a box
  `Used`; a change writing one 80-byte uniform (section 4).
- Eight slices (section 5), seven decisions for Tom (section 6). The
  first decision is how an anchor gets a `StyleKey`; the recommendation
  keeps 3D decision 1 (Rust components with typed props) and adds a
  compiler-emitted key for a component tag that carries `class` or
  `class:`.

## 0. What reaches scene nodes today, and after PR 4b

### Today (main at 9b405f9)

Nothing. `vapors-scene3d` is a Rust API: `NodeKind::Mesh { geometry,
colour }` (`crates/scene3d/src/node.rs`), `Transform` (`transform.rs`),
`Colour::srgb` (`colour.rs`, opaque, sRGB-encoded channels 0 to 1).
`vapors-scene3d-gpu` draws each mesh node from one uniform slot of
world matrix and colour (`crates/scene3d-gpu/src/shader.wgsl`, `struct
Mesh`), written when the node's `world_version` changes. No document
element corresponds to a node.

### After PRs 250 to 252 (open)

`<Scene>`, `<Group>`, `<Mesh>`, `<Camera>` are Rust components in
`crates/scene3d-components`. Each mounts one anchor: a `div` made with
`Dom::create_element` and the inline style `display: none`
(`src/node.rs`, `ANCHOR_STYLE`, `anchor()`), placed where the tag is in
the template, with the component's children placed inside it. A node
component finds its parent node through the anchor's
`data-scene3d-node` attribute and a per-thread table (`placement.rs`).
`transform` and `colour` are signal props; a change writes that node
alone and invalidates the surface (#252: 80 bytes, no geometry).

The anchor has a computed style, as every element does, and nothing
reads it.

### After PR 4b as planned

`3d-2.md` row 4b: "each node's anchor element takes classes, custom
properties and state pseudo-classes like any element; its computed
`color`, `opacity` and `visibility` set the node's appearance, and a
change to them updates that node alone, counted (no geometry, one
uniform). `:hover` on a mesh through a 3D hit test that sets the
anchor's hover state. Tokens reach the scene."

What main does not have for that, each with the fix this plan gives:

1. **`opacity` is refused.** `crates/elements/src/style/honoured.rs`
   lists `("opacity", Deferred("F3"))`, and `check_honoured` refuses any
   element whose computed `opacity` is not 1. Fix: the scene property
   set (section 4) reads `opacity` on scene anchors; boxes keep the
   refusal until F3.
2. **Anchors are unkeyed and take no `class:`.** The compiler turns
   every attribute of a component tag into a prop
   (`crates/compiler/src/codegen/component_tag.rs`,
   `emit_prop_values`), so `<Mesh class:braking={brake}>` is a prop
   named `class:braking`, refused by `check_props`. `StyleKey`s are
   emitted for elements only (`codegen/style_key.rs`,
   `element_class.rs`), and the anchor is created without one, so it
   has no share key, no build-time winners and no place in the
   analysis. Fix: decision D1.
3. **No hit path to a node.** `Painted::hit` returns the topmost
   element whose border box holds the point
   (`crates/elements/src/painted.rs`, `hit`); a `gpu-surface` is one
   box; the anchors have none. Fix: section 1, states, and slice C4.
4. **Group opacity and transparent meshes are undefined.** The pipeline
   is opaque and depth-tested, and the 3D brief defers "transparent
   material sorting" (`docs/briefs/GPU_SURFACE_3D_BRIEF.md`, Deferred
   work). Fix: decisions D3 and D4.

Decision D5 asks whether 4b is built as `3d-2.md` words it (unkeyed
anchors, run-time only, reworked later) or as slices C1 to C3 here.

## 1. The property table

The anchor of a scene node is an element in the document, so the
cascade, selectors, inheritance, custom properties, `@media` and state
flags apply to it with no new code. What is new is which computed
values a node reads, how a change reaches the GPU, and what is
refused. Each row says where the value goes and what a change costs.

### 1.1 Appearance

| Property | On which nodes | Maps to | A change costs |
|---|---|---|---|
| `color` | `mesh`; `path` under a `decal` reads `fill`/`stroke` instead (`one-scene.md` 1.5) | the mesh's colour, multiplied in the shader by the per-vertex shade (`one-scene.md` D4); the colour's alpha multiplies into the node's alpha with `opacity`, as an element's text alpha composes with its opacity | one uniform (80 bytes today: world matrix and colour, `shader.wgsl`), no geometry |
| `opacity` | `mesh`; refused on `group` (D3) | alpha of the node; 1 draws in the opaque pass, under 1 in the blended pass (1.4) | one uniform; a node crossing 1 moves between passes, no reallocation |
| `visibility` | `mesh`, `group`, `decal` | `hidden` and `collapse`: the node is not drawn; children inherit through the anchors, and a child declaring `visible` is drawn, as CSS says | the draw list skips the node: one less draw, no uniform |
| `pointer-events` | `mesh`, `group` | `none`: the 3D hit test (1.5) skips the node (a group: its subtree) | nothing on the GPU |
| `cursor` | `mesh` | the cursor while the pointer's 3D hit is this node, through `Painted::cursor` | nothing |
| custom properties (`--name`) | every scene tag | read by `var()` in the rows above; the carrier of tokens (`--craft-body`) | only through the property that reads it |
| `transition-*`, `animation-*` | over the rows above | section 1.6 | per frame while running: one uniform per transitioning node |

Mapping `color` rather than a new `--vapors-color` (as
`docs/design/09-3d.md` sketched): `color` is inherited, so a token on
`:root` or a `color` on the scene's HTML container reaches every mesh,
and `currentColor` in a decal's `fill` matches the body it sits on. The
default is CSS's initial `color`, black, which is `Colour::BLACK` in
#250. A material parameter with no CSS name (an emissive highlight when
slice 3's materials come) is a custom property the material declares,
`--vapors-emissive`; its name is literal in the sheet, so the analysis
sees every reader (`analysable-sources.md`, item 3).

Colour values: `vapors-css` computes a colour to sRGB `Rgba`
(`text_style.rs` reads `color` that way); the node takes the three
channels as `Colour::srgb` and the alpha as above. `light-dark()` and
`prefers-color-scheme` therefore work on meshes through the
`Environment` (`crates/css/src/cascade/environment.rs`).

### 1.2 Transforms: CSS `transform` and the 3D family

CSS Transforms 2 gives elements `translate3d()`, `rotate3d()`,
`rotateX/Y/Z()`, `scale3d()`, `matrix3d()`, `perspective()`,
`transform-origin` with a z component, `perspective` and
`perspective-origin` on a parent, `transform-style: preserve-3d` and
`backface-visibility`. In vapors today `transform` parses `translate`,
`translateX`, `translateY` and `scale` only
(`crates/css/src/syntax/special.rs`, `fn transform`), is `Deferred` in
`honoured.rs`, and `perspective`, `transform-origin`, `transform-style`
and `backface-visibility` are not in the property table
(`property/table.rs`, `table_rest.rs`), so a sheet using them is
refused at parse as an unknown property.

How each relates to a scene node, and what this plan does:

| CSS | What it means on an element | On a scene node | This plan |
|---|---|---|---|
| `transform` | a 4x4 matrix applied to the box, lengths in CSS px, about `transform-origin` | the node already has a local matrix from its `transform` prop (`Transform`: translation, rotation, scale; `world = parent * local`), in the application's units | refused naming the prop (D2). Two writers of one matrix would break "one uniform per change" only mildly, but CSS lengths are px and scene units are the app's (`crates/scene3d/src/lib.rs`, conventions), so `translate3d(10px, …)` has no meaning in the scene. The one CSS-only case, `:hover { transform: … }`, is D2(b) |
| `perspective`, `perspective()` | a projection applied by the parent, in px | the active `<Camera projection={Projection::Perspective { vertical_fov, near, far }}>` (`projection.rs`) | refused naming `<Camera>`: there is one answer to how the scene projects |
| `transform-origin` | the point a transform is about | a scene transform is about the node's origin; a pivot is a parent `<Group>` with the offset | refused naming `<Group>` |
| `transform-style` | whether children share the parent's 3D space (`preserve-3d`) or are flattened | every scene node is in one 3D space; there is no flattening | refused as meaningless on a scene node |
| `backface-visibility` | hide an element whose back faces the viewer | back-face culling per mesh: a pipeline state in wgpu (`cull_mode`), so `hidden` and `visible` are two pipelines, one draw either way | not in the first version; later slice C9, after the mesh winding convention is documented |
| `rotate`, `translate`, `scale` (the individual properties) | the same matrix in three properties | the same as `transform` | refused with it |

So CSS 3D transforms and scene transforms are different things that
share names: the former move a flat box in a browser's px space under
a per-parent projection; the latter place geometry in the scene's
units under one camera. Section 2 states the difference against
Chromium.

### 1.3 Visibility and display

`visibility` maps (1.1). `display` does not: a scene anchor's `display`
is `none` by the user-agent sheet (slice C1 moves it there from #251's
inline style), and that is what keeps the anchor out of layout and the
accessibility tree (3D decision 1). An author's `display` on a scene
tag, any value, is refused naming `visibility: hidden` for hiding and
`{if}` for removing the node. `display: none` removing a subtree from
the scene would duplicate `{if}` with a second mechanism the analysis
would have to track.

### 1.4 Opacity and transparency ordering

Today every mesh is opaque, depth-tested, drawn in node order
(`crates/scene3d-gpu/src/draw.rs`). A mesh with alpha under 1 needs
blending, and blended fragments must be drawn after everything they
blend over and back to front among themselves, or the result depends on
draw order. Design (D4, recommendation):

- Two passes in one render: the opaque pass as today, then the blended
  pass with depth test on and depth write off, alpha blending.
- The blended pass draws the transparent nodes sorted by the view-space
  depth of each node's world origin, far to near. The sort is per frame
  and only over the transparent nodes (`t log t`, 0 work and 0
  allocation when there are none; counted as `nodes_sorted`).
- Sorting by node origin, not per triangle, is what three.js does for
  transparent objects by default (`sortObjects`, by the object's
  position's depth) and what Unity's transparent queue does; both get
  intersecting or concave transparent meshes wrong, and so does this.
  Order-independent transparency is D4(c), not recommended.

Group opacity: in CSS, `opacity` on a parent composites the whole
subtree at that alpha; two overlapping children do not double-fade.
Doing that in a scene means rendering the subtree to its own target and
compositing it, a layer per group. Not in this plan (D3): `opacity` is
refused on `group` naming the meshes, and allowed on `mesh`. The
difference from CSS is stated in the refusal.

### 1.5 States: `:hover`, `:active`, `:focus` on meshes

The matcher's state flags are set by the tree layer
(`crates/css/src/tree/mod.rs`, `States`; `docs/css.md`, "Using it").
For HTML, `rehover_in` (`crates/elements/src/events_hit.rs`) asks the
kept hits for the element under the pointer and calls `set_flag_chain(
States::HOVER, target)`, which sets `:hover` on the target and its
ancestors. Scene nodes need one new source of hits, and then the same
chain:

- **A 3D hit test** on the scene: when `Painted::hit` lands on a
  `gpu-surface` whose renderer is a `SceneRenderer`, the surface asks
  the scene for the node under the surface-local point: a ray from the
  active camera through the point (the inverse of `view_projection`,
  `projection.rs`), tested against each mesh node's world-space
  bounding box, then its triangles (the geometry is held on the CPU in
  `Meshes`, `meshes.rs`). The nearest hit's anchor is the target. A
  node whose anchor's `pointer-events` is `none`, or `visibility` is
  not `visible`, is skipped, as HTML's `takes_pointer` and `visible`
  skip boxes (`style/used.rs`).
- **Cost**: per pointer move over the surface, one box test per mesh
  node and triangle tests for the boxes the ray enters. The racer's
  scene has about 11 nodes (`one-scene.md` 2.2); the 541 trees are 4
  merged nodes there. With `Instances` (3D brief slice 4) the box test
  is per instance; a bounding hierarchy is work for then, measured
  first. The pointer at rest costs nothing: `kept_hits.rs`'s
  `may_move_hover` already gates re-testing on a change under the
  pointer, and a scene change (a node moved) marks the surface's hit as
  possibly moved, so a moving scene under a still pointer re-tests at
  most once per rendered frame.
- `:hover` then flows up the anchor chain: `.craft:hover` on the car's
  `<Group>` matches when any of its meshes is hit, because the mesh
  anchors are inside the group's anchor.
- `:active` is the pointer down on the hit node, through the same
  dispatch as an element.
- `:focus`, `:focus-visible`, `:focus-within`: an anchor is outside the
  accessibility tree (3D decision 1) and not focusable; `:focus` rules
  on a scene tag are refused in the first version (D6).
- Events: the hit anchor is the target of `pointerdown`, `pointerup`
  and `click`, with the usual bubbling through the anchors to the
  surface and its ancestors. `pointerover`/`enter`/`out`/`leave` are
  not fired anywhere in vapors (`3d-1c.md`, open items from PR 5) and
  stay unfired here.

### 1.6 Transitions and animations, and idle at zero frames

`vapors-css` parses and samples transitions and `@keyframes`
(`crates/css/src/animation/`); no element animates yet
(`docs/design/06-animation-transitions.md`, status; `honoured.rs`
defers every `transition-*` and `animation-*`). Scene nodes get
transitions when elements do (design slice A1), with one addition:

- A transition on an anchor's `color` or `opacity` is used-style state
  in the elements crate, sampled at `frame.now` in the animation tick
  (`docs/design/02-scheduler-frame-threading.md`, "Time"), never a
  signal. Each sample that changes the node's `SceneUsed` (section 4)
  writes that node's uniform and invalidates the surface: one uniform
  per transitioning node per frame.
- **Frames**: the surface requests animation (`SurfaceHandle::
  request_animation`, `3d-1c.md` PR 1) while any node under it has a
  running transition or animation, and stops when the last ends. The
  app's loop then returns to `Next::Wait`, so an idle scene draws zero
  frames (`idle_fps`, `CLAUDE.md`). The request is made by the
  elements layer from the set of running transitions, not by app code.
- `transition` on `transform` is refused with `transform` (1.2). Motion
  of a node is the `transform` prop's signal, or a frame task
  (`09-3d.md`, "Bulk animation is a frame task"), as decided for the
  lane.
- Reduced motion applies as the framework rule says (design 06).
- Keyframe animations on `color` and `opacity` (a pulsing selection)
  follow A4 the same way.

### 1.7 Custom properties and tokens

A `var()` in a scene rule resolves as anywhere: from the anchor's own
winners, else its ancestors, HTML included. `--craft-body` set on the
scene's HTML container (`one-scene.md` 2.4) reaches every mesh under
it; a livery enum bound as `class:` toggles on the scene root changes
which rule wins. A binding form for custom properties does not exist
yet (`build-time-styles.md`, section 1, "Custom properties"); when it
is added with a literal name, the compiler knows which scene keys read
it, and the runtime restyles those anchors only.

### 1.8 Media queries

`@media` conditions read the `Environment` (viewport size, resolution,
colour scheme, reduced motion; `media_eval.rs`). A rule under `@media`
targeting a scene tag is conditional in the analysis
(`build-time-styles.md`, section 1) and is re-evaluated when the
environment changes, like any element's; nodes whose `SceneUsed`
changes write one uniform each. `@container` on a scene tag: the
anchor has no box, so no container; refused naming `@media`.

### 1.9 Selectors over the scene tree

The anchors nest as the scene nests (a mesh's anchor is inside its
group's anchor, #251 `mount_node`), in child order, so every selector
form `vapors-css` supports (`docs/css.md`, Supported) reads the scene
hierarchy: `group > mesh`, `.craft mesh`, `mesh:first-child`,
`mesh:nth-child(2)`, `group:has(mesh:hover)`. The tag in a selector is
the anchor's tag, which under D1(b) is the scene tag (`mesh`, `group`,
`camera`, `scene`, `decal`), so `mesh { color: … }` styles every mesh.
Sibling and `:has()` forms make the key unshareable, as for elements
(`restyle_share.rs`, `unshareable`), which costs one computed style per
such node rather than one per share key; the count `styles_unshareable`
shows it.

### 1.10 Inheritance from HTML ancestors

The `<Scene>` anchor sits in the document where the tag is written,
under whatever HTML element contains it. Inherited properties flow in:
`color`, `visibility`, `pointer-events`, `cursor` and custom
properties (the inherited ones among the scene set). This is what makes
tokens on `:root` and a panel's `color` reach the scene with no
plumbing, and it is also a default an author may not expect: a page
whose `body { color: #333 }` tints every mesh that declares no colour.
The user-agent rule for scene tags declares nothing, as CSS gives no
default colour to a `span` either; the docs state the inheritance.
Non-inherited properties (`opacity`) start at their initial value on
each anchor, as CSS says.

## 2. Existing practice

Stated as the real degree of similarity, not as an analogy.

- **CSS 3D transforms in browsers (CSS Transforms 2, Chromium).** An
  element stays a box laid out in CSS px; `transform` gives it a 4x4
  matrix; `perspective` on a parent gives a per-parent projection;
  `preserve-3d` lets a subtree share one 3D space, which the browser
  renders as layers sorted by depth (Chromium does not split
  intersecting planes). There is no camera, no depth buffer across
  contexts, no mesh, no lighting; `getBoundingClientRect` gives the
  projected box and `elementFromPoint` hit-tests through the inverse
  matrix. The resemblance to a vapors scene node is in the words only:
  both have a 4x4 matrix per thing. The differences: scene units are
  the app's, not px; one camera for the scene, not a projection per
  parent; a real depth buffer; geometry, not boxes. `09-3d.md`'s stage
  1 (CSS 3D transforms on elements, for touché's layer view) is a
  separate feature from scene nodes and is not changed by this plan.
- **A-Frame.** Entities are DOM elements (`<a-entity>`,
  `<a-box>`), with components set as attributes (`position="0 1 0"`,
  `material="color: red"`). Selectors are used from script to find
  entities (`querySelector`); the cascade is not read: no CSS property
  styles an entity, and reuse is through `<a-mixin>` attributes, not
  rules. vapors shares the shape (a scene node has an element in the
  document) and differs in reading the cascade for appearance.
- **three.js.** No CSS on scene objects; material fields are set in
  JavaScript. `CSS3DRenderer` goes the other way, placing HTML elements
  into a 3D scene with CSS 3D transforms. Transparent objects are
  sorted by object depth (`sortObjects`), which 1.4 takes.
- **Threlte and React Three Fiber.** Scene objects as components with
  reactive props; no cascade. 3D decision 1 already takes this model
  for the typed props; this plan adds the cascade for appearance only.
- **Unity UI Toolkit (USS).** A CSS-like stylesheet language for
  Unity's 2D UI; 3D objects are not styled by it. No precedent for
  cascading styles onto meshes there.

No framework in this list reads a CSS cascade onto scene geometry. The
closest is the SVG model, where presentation attributes and CSS both
set `fill` and `stroke` on shapes drawn by a renderer that is not the
box model; `one-scene.md` 1.5 already maps `fill` and `stroke` that
way for decal paths, and this plan treats `color` and `opacity` on a
mesh the same way.

## 3. Where CSS does not fit 3D: what is refused

A scene node is not a box. For each scene tag, a property set says
which longhands a node reads (section 4); a declaration of any other
longhand that can reach the node is refused, naming the rule's
`file:line:column`, the node's site and the mechanism to use. Refusing
a declaration, not a computed value: an anchor inherits `font-size`
from `body`, which is not a declaration on the node and is not refused.
The groups, with the message each gives:

| Refused on scene tags | Instead |
|---|---|
| the box model: `width`, `height`, `min-*`, `max-*`, `aspect-ratio`, `box-sizing`, `padding-*`, `margin-*`, `border-*`, `outline-*`, `box-shadow`, `background-color`, `overflow-*`, `white-space`, `text-align`, `letter-spacing`, `line-height`, `font-*`, `text-decoration`, `user-select`, `scrollbar-*`, `column-gap`, `row-gap`, every flex and grid longhand | geometry and `<Mesh geometry>`; the surface's box is the `gpu-surface` element's |
| layout position: `position`, `top`, `right`, `bottom`, `left` | the `transform` prop |
| `z-index` | depth: the depth buffer orders opaque meshes, 1.4 orders transparent ones; there is no paint order to index into |
| `display` | `visibility: hidden`; `{if}` |
| `transform`, `rotate`, `translate`, `scale`, `transform-origin`, `transform-style`, `perspective`, `perspective-origin`, `backface-visibility` | the `transform` prop; `<Group>` for a pivot; `<Camera projection>`; C9 for `backface-visibility` later |
| `filter`, `mix-blend-mode`, `isolation`, `clip-path`, `mask` (not in the table today; `filter` is) | nothing: a post-process on the scene is not planned (3D brief, Deferred work) |
| `opacity` on `group` | `opacity` on each `mesh` (D3) |
| `:focus`, `:focus-visible`, `:focus-within` in a selector whose subject is a scene tag | not focusable in the first version (D6) |
| `@container` conditions on scene tags | `@media` |
| `fill`, `stroke`, `stroke-width` on `mesh` | `color`; `fill` and `stroke` are for `path` under `decal` |

What is not refused: inherited values arriving from HTML (1.10),
custom properties of any name (a `--name` nobody reads is inert, as in
CSS), and the state pseudo-classes 1.5 supports.

Refusal timing: for a keyed anchor the analysis sees every rule of the
crate's sheets that can match the key, so a refused declaration fails
the build (section 4); rules from other crates' sheets and unkeyed
anchors (hand-built tests) are refused at the element's first restyle
under the update-error convention (`ElementError::Refused`,
`dom_keyed.rs`), as `check_honoured` and `style/surface.rs` refuse
today. Nothing is dropped or approximated in either case.

## 4. Fit with build-time styles

`build-time-styles.md` gives every keyed template element a
`StyleKey` and, with B1 to B3b, a `KeyWinners`: per longhand, a
constant or known-variable winner, with the conditional rules that can
override it. Scene nodes join that with three additions and no second
mechanism.

### 4.1 A scene anchor is a keyed template element

Under D1(b): a component tag that carries `class`, `class:` or `style`
gets a `StyleKey` emitted by the compiler, as an element does
(`codegen/style_key.rs`): site, scope, tag (the component's name in
lower case: `mesh`), literal classes, directive names, literal `style`.
The key is passed to the component as a generated prop `anchor:
&'static StyleKey`, and a component whose props struct has no such
field fails to compile at the tag (rustc names the field; the compiler
adds the message "`<Card>` takes no `class`: it has no anchor
element"). The scene components create their anchor with
`Dom::create_keyed_element(key)` (`crates/dom/src/lib.rs`), which fixes
the tag and literal attributes, and toggle through
`Dom::toggle_classes`. #251's inline `display: none` moves to the
user-agent sheet (`scene, group, mesh, camera, decal { display: none
}`, `crates/css/src/ua.css`), since a literal `style` on the key would be the
author's, and the anchor's attribute `data-scene3d-node` stays an
ordinary attribute set after creation.

With that, a scene anchor has what every element has: a share key
(`style/share.rs`), a place in the share table, `KeyWinners`, and the
first-miss verification and debug oracle of `build-time-styles.md`
section 2. The 541 tree-foliage meshes of a scene that gave each tree
its own node would share one computed style.

### 4.2 The scene property set, checked at build time

A table in `vapors-css` (so the compiler and the runtime read one
answer, as `build-time-styles.md` section 1 requires for selector and
value reasoning), per scene tag, of the longhands a node of that tag
reads:

| Tag | Reads |
|---|---|
| `scene` | custom properties; `visibility`, `pointer-events` (inherited to the nodes) |
| `group` | `visibility`, `pointer-events`, custom properties |
| `mesh` | `color`, `opacity`, `visibility`, `pointer-events`, `cursor`, custom properties, `transition-*`, `animation-*` |
| `camera` | custom properties |
| `decal` | `visibility`, `pointer-events`, custom properties; its `path` children read `fill`, `stroke`, `stroke-width` (`one-scene.md` 1.5) |

B1's classification runs per key over the candidate rules
(`Cascade::rules_for_key`). For a scene key, one more judgement per
candidate rule that is **always** or **conditional**: every longhand
the rule declares must be in the tag's set, or the build fails naming
the rule's `file:line:column`, the property, the element's site and
the row of section 3's table. Inline `style` on the tag is checked the
same way. That is the whole build-time refusal: a rule that **never**
matches a scene key is not checked against it, so `div { padding: 4px
}` beside `mesh { color: red }` is fine.

At run time, for rules the compiler did not see (another crate's
sheets, `winners_foreign`) and for unkeyed anchors, the same table is
applied to the element's winners on its first miss, refusing under the
update-error convention. `check_honoured`'s comparison of computed
values against initial (`honoured.rs`) is not used for scene anchors:
it would refuse inherited values (section 3).

### 4.3 `SceneUsed` in place of `Used`

`style::used` derives a box `Used` of layout, text and paint fields
(`style/used.rs`). A scene anchor derives instead:

```rust
pub(crate) struct SceneUsed {
    colour: Rgba,          // `color`, alpha kept
    opacity: f32,          // `opacity`, 1 on tags that do not read it
    visible: bool,         // `visibility: visible`
    takes_pointer: bool,   // `pointer-events: auto`
    cursor: Cursor,        // meshes only
}
```

derived by index (`build-time-styles.md` U1), shared per computed style
as U2 shares `Used`. A scene anchor builds no layout box and no paint,
as a `display: none` element does today (`boxes_built` counts none for
it).

### 4.4 A change writes one uniform

On an update that restyles a scene anchor (a class toggle, a state
flag, a token change, an environment change, a transition sample):

1. The anchor takes its shared computed style (a hit) or computes one
   (a miss, from `KeyWinners`).
2. Its `SceneUsed` is derived or shared, and compared with the kept
   one. Equal: nothing more.
3. Changed: the elements layer calls `Scene::set_appearance(node,
   colour, opacity, visible)` (extending #250's `set_colour`), which
   advances the node's `world_version`, and `SurfaceHandle::invalidate`.
   The next frame writes that node's uniform slot (80 bytes; opacity
   rides in the colour's alpha, so the slot does not grow) and draws
   or skips the node. No geometry, no other node.

Counts: `elements_restyled` equals the anchors whose style changed, the
surface's `buffer_bytes` equals 80 times the nodes whose appearance
changed, `draw_calls` falls by the nodes hidden. A token on `:root`
read by 541 nodes restyles 541 anchors (one computed style, 541 `Used`
hits) and writes 541 uniforms: flat in scene size, linear in the nodes
the change reaches. D7 asks whether nodes sharing a style should share
an appearance slot, so that case writes one uniform; not for the first
version.

Idle: no restyle, no frame. A node whose appearance properties all have
constant values (`build-time-styles.md`, "constant value") cannot
change at run time; its anchor is never marked for restyle by a toggle
or state elsewhere, which the analysis already gives every element.

### 4.5 Relation to the splice mark and the shape table

Nothing in a scene is laid out or painted in paint order, so the splice
mark (`build-time-styles.md` section 4) does not apply to scene keys:
a `{for}` of `<Mesh>` is marked by the same rule (no rule that matters
can match a row element; the scene set excludes them all, so a scene
row is always marked). Decal `path`s are shape-table entries
(`one-scene.md` S5), and their `fill` and `stroke` go through the same
`KeyWinners`; this plan adds nothing for them.

## 5. Slices

Each under about 400 lines with tests; each check proven by breaking
what it guards, watching it fail, restoring, stated in the commit. All
after the finish line, except where D5 moves C1 to C3 into PR 4b.
Dependencies name the plan they come from.

| # | Slice | Content | Accepted when | Depends on |
|---|---|---|---|---|
| C1 | Keyed anchors | The compiler emits a `StyleKey` and the `anchor` prop for a component tag carrying `class`, `class:` or `style` (D1); a component without the field fails to compile with the message; `Scene`, `Group`, `Mesh`, `Camera` create keyed anchors with their tag; `display: none` for the scene tags in the UA sheet, #251's inline style removed; `data-scene3d-node` unchanged | `<Mesh class:braking={b}>` compiles and toggles the anchor; `styles_shared` counts two `<Mesh>` of one key as one computed style; `<Card class="x">` on a component without an anchor fails the build naming the tag; snapshot of the generated key; every #251/#252 test unchanged | #252 merged; analysis PR 2d (merged) |
| C2 | The scene property set | The per-tag table in `vapors-css` (4.2); `SceneUsed` derived for scene anchors in the restyle, no box or paint; run-time refusal of a declared longhand outside the set naming rule, property, site and the alternative; `opacity` read on scene anchors, still deferred on boxes; an unclassed anchor's key is the component's own (`own_keys.rs`), which the compiler never emits, so the run-time check is its only check: test `mesh { width: 10px }` reaching an unclassed `<Mesh>` | `mesh { width: 10px }` refused at the first update naming the rule; `body { font-size: 14px }` above a scene refuses nothing; `opacity: .5` on a `div` still refused; `elements_restyled` on a class toggle equals the anchors it reaches | C1 |
| C3 | Appearance to the node | `Scene::set_appearance`; the restyle writes a changed `SceneUsed` to the node and invalidates; `visibility: hidden` skips the draw; `color` alpha times `opacity` into the uniform's alpha (opaque pass only until C5: alpha under 1 refused naming C5 until it lands); C1's pin `a_mesh_keeps_its_colour_prop_whatever_its_anchors_computed_colour` is replaced by a test that the racer HUD's `:global(body) { color }` reaches an unclassed scenery mesh only as the docs state (inherited), and the `wisp-check` note of risk 2 listing scene keys whose `color` is inherited and never declared | Pixel tests: a class toggle recolours one mesh with 80 bytes written and 0 geometry; a `--token` on `:root` recolours every reader, 80 bytes each; `visibility: hidden` drops one draw; an idle frame writes 0; `#252`'s `colour` prop removed | C2, #250 |
| C4 | 3D hit test and states | Ray from the active camera through the surface-local point; bounding box then triangles over `Meshes`; `Painted::hit` through a scene surface to the hit anchor; `HOVER` and `ACTIVE` on the anchor chain; `pointer-events: none` and hidden nodes skipped; `cursor` from the hit mesh; `hovers_tested` covers it; a scene change marks the surface's hit as possibly moved | Hovering the craft applies `.craft:hover` to its group (pixel test); a mesh behind another is not hit; `pointer-events: none` passes to the mesh behind; a still pointer over a still scene tests 0 hovers per frame and draws 0 frames; a click on a mesh reaches its anchor's listener | C3; 3D PR 5 (geometry lifetime) |
| C5 | Transparent meshes | The blended pass, depth write off, nodes sorted by origin depth far to near (D4); `nodes_sorted` count; opacity on `group` refused (D3) | Two overlapping translucent meshes render the same from both draw orders (pixel test against a reference render); an all-opaque scene sorts 0; a node crossing opacity 1 changes pass with one uniform write | C3 |
| C6 | Build-time refusal and winners for scene keys | B1's classification applies 4.2's table to scene keys; `wisp-check --styles` lists each scene key's winners; the runtime takes compiled winners for scene anchors as for elements; constant-value appearance nodes skip restyle marking | `mesh { padding: 1px }` fails `compile_dir` naming rule and element; `winners_runtime` 0 on the racer's scene page; a toggle on the HUD visits 0 scene anchors whose appearance is constant (`elements_visited` pinned) | C2; `build-time-styles.md` B3b |
| C7 | Transitions on scene nodes | `transition-*` on `color` and `opacity` for scene anchors through A1's machinery; the surface requests animation while any node transition runs and stops after; reduced motion | Stepped clock: a 200 ms colour transition writes one uniform per frame for that node and none for others; frames stop at the end (`idle_fps` 0 after); `Environment { reduced_motion: true }` makes it instant | C3; design 06 slice A1 |
| C8 | Docs | `docs/scene3d.md` (3D PR 5's lane doc) gains the property set, the refusals, inheritance from HTML, hit testing, transparency; `docs/css.md` points to it | The docs name every row of section 3 | C1 to C7 |

C1 as built (branch `gpu/c1-scene-style-key`): as the row says, with
three choices the row left open. A tag with no `class`, `class:` or
`style` gets no key from the compiler, and its anchor is made from the
component's own key (`own_keys.rs`: the scene tag and nothing else),
so every anchor is keyed and unclassed meshes share. The directives
are toggled by the parent's generated effect on the element the
component returns from `anchor_element()`, the same emission as an
element's. `display` other than `none` on a scene tag is refused at
the update naming the declaration (`style/used.rs`), pulled forward
from section 3: the user-agent rule, unlike #251's inline style, loses
to an author's rule.

C2 as built (C2a, C2b): the set is
`vapors_css::property::scene` (`SceneTag::reads`, `instead`), every
longhand in one group of section 3's rows, a test holding the groups
and the property table together. A scene anchor keeps a `SceneUsed`
(`Document::scene_used`) and no `Used`, so the layout and paint
readers never see it; C1's computed `display` refusal is now the
set's: an author's `display` of any value is refused, the user-agent
sheet's is not. A declared `transition-*` or `animation-*` on a mesh,
in the set, is refused naming C7 until it lands. `SceneUsed` is derived per
anchor at each restyle, not yet shared per computed style as 4.3 says;
C6, which gives scene anchors compiled winners, shares it. `opacity`
computed outside 0 to 1 (a `calc()`) is refused, as a literal is.

Later, not in the stack: C9 `backface-visibility` as a per-mesh cull
mode (two pipelines; the winding convention documented first); C10 a
per-style appearance slot with `Instances` (D7); keyframe animations on
nodes when A4 lands (the same path as C7).

Order: C1, C2, C3, C4, C5, C6, C7, C8. C4 and C5 are independent of
each other once C3 is in; C6 waits for the build-time stack.

## 6. Decisions

Decided by Tom, 2026-11-13: D5 (b), PR 4b is built as slices C1 to C4
(component tags carrying `class` get a compiler-emitted `StyleKey`;
scene nodes take the build-time path), not unkeyed and reworked later.
The other decisions below are still open.

Each with options, the trade-off and a recommendation.

**D1. How a scene anchor gets a `StyleKey`.**
(a) Lower-case scene tags (`<mesh>`, `<group>`) in the compiler's
vocabulary (`template/vocabulary.rs`), mounted by `vapors-elements` as
`gpu-surface` is, their attributes typed by a table. Every element
mechanism applies unchanged; the scene components of #251/#252 and
their placement table are replaced. It reverses 3D decision 1 (Tom,
2026-11-10: Rust components with typed props; "not chosen: lowercase
elements in the document").
(b) Keep the components; the compiler emits a key for a component tag
that carries `class`, `class:` or `style` and passes it as a generated
`anchor` prop; the component creates its anchor from the key (4.1). A
component without an anchor refuses the attributes at compile time.
Generalises to any component with one root element, which is also how
a `class` on a component tag would be given meaning for non-scene
components later.
(c) Anchors stay unkeyed: appearance styled at run time only. Fails
the rule that the analysis sees every source of change, and leaves
scene anchors out of the share table.
**Recommend (b):** it keeps decision 1 and the open PRs, and the
compiler change is one emission path shared with elements.

**D2. CSS `transform` on scene nodes.**
(a) Refused, naming the `transform` prop (1.2). (b) Accepted as a
second layer, `local = prop * css`, with only unitless numbers and
angles allowed (`rotate3d`, `scale3d`, `translate3d` with plain
numbers in scene units; a px length refused), which would give
`:hover { transform: … }` and a transition on it. (b) adds a second
writer to the node's matrix and a value grammar vapors-css does not
have.
**Recommend (a).** Revisit (b) when a product needs hover motion on a
node; the one-scene racer needs none.

**D3. `opacity` on `group`.**
(a) Refused, naming the meshes. (b) Multiplied down into descendants'
alpha: cheap, and not what CSS group opacity means (overlapping
children double-fade). (c) CSS semantics: the subtree rendered to its
own target and composited, a layer per group.
**Recommend (a):** (b) would be a silent difference from CSS, and (c)
is a renderer feature with no product need.

**D4. Transparent ordering.**
(a) A blended pass after the opaque pass, sorted by node origin depth
(1.4), as three.js and Unity do by default; wrong for intersecting
transparent meshes, stated in the docs. (b) No sort: draw order, which
makes the result depend on template order. (c) Order-independent
transparency (weighted blended or per-pixel lists): correct, a larger
renderer change, costs per pixel per frame.
**Recommend (a).**

**D5. The scope of 3D PR 4b.**
(a) Build 4b as `3d-2.md` row 4b words it: unkeyed anchors styled at
run time, `opacity` read on anchors, `:hover` through a hit test; then
C1, C2 and C6 bring it under the analysis after the finish line. The
run-time part is built twice (once on unkeyed anchors, once on keyed).
(b) Build 4b as C1 to C4 of this plan now, so the anchors are keyed
from the start and C6 is the only later step. About 1,200 lines
against 4b's estimate of about 400; the racer's R3b (`one-scene.md`)
depends on it either way.
**Recommend (b):** every line of (a)'s run-time styling is kept in
(b), and (b) does not build anchors twice. If the finish line's
schedule cannot take the extra rows, (a) with C1 scheduled first after
the line.

**D6. `:focus` on meshes.**
(a) Refused in the first version: anchors are outside the accessibility
tree (3D decision 1) and not focusable. (b) Focusable meshes with an
accessibility node each (a name from a prop), tab order by scene
order. (b) is an accessibility design of its own.
**Recommend (a),** and a note in `docs/scene3d.md` that a scene's
keyboard interaction is the surface's or the HUD's.

**D7. Appearance per node or per shared style.**
(a) Per node, in the existing uniform slot (4.4): a token change writes
one uniform per node that reads it. (b) A second uniform buffer of
appearance slots, one per shared computed style, each node's slot
naming its style's entry: a token change writes one slot for every
node sharing it; the racer's 4 merged scenery nodes gain nothing, 541
instanced trees would. Needs a second bind group or a slot index per
node.
**Recommend (a)** now, (b) with `Instances` (3D brief slice 4), when
the count of nodes per style is measured.

## 7. Risks

- **Component tags with keys (D1(b)) touch the compiler's component
  path** (`component_tag.rs`, `props_index.rs`), which #251 found
  cannot nest a generated component in another's children. That fix is
  needed for R3a1 anyway and lands first; C1 then has a stable base.
- **`color` inherited from HTML** (1.10) tints every mesh under a
  page's body colour. The racer's HUD sets `:global(body) { color:
  #f3f1e8 }` (`crates/racer/ui/hud.wisp:33`); R3b's `--craft-body`
  rule wins on the body meshes, but scenery with no rule of its own
  would take that off-white. The docs state it; a `wisp-check` note
  listing scene keys whose `color` is inherited and never declared
  would catch it at build time (not in the stack; add if C3 shows it
  surprising).
- **The hit test's cost with many nodes** (1.5) is one box test per
  mesh node per pointer move; fine at the racer's 11, unmeasured at
  hundreds. The `Instances` slice measures before adding a hierarchy.
- **Transitions depend on A1**, which has no date; C7 waits. Until
  then a token change recolours in one frame, which the one-scene
  acceptance (R3b) asks for anyway.
- **The transparent sort's known error** (intersecting meshes) is
  stated, not solved; a product that needs glass-on-glass needs D4(c).
- **Refusal lists drift.** The scene property set (4.2) and section
  3's table must agree; a test holds the set and the CSS table together
  (as `honoured_tests.rs` does for boxes), so a new CSS property fails
  until it is placed for scene tags too.
