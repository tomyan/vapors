# What vapors supports

Generated from the source tables by `crates/compiler-examples/tests/supported`;
do not edit by hand. Regenerate with
`UPDATE_SUPPORTED=1 cargo test -p vapors-compiler-examples --test supported`.
The unit tests fail when this file is out of date.

Anything not listed is refused with a message naming the file, line and
column. Some refusals happen only when the page runs: the CSS parser
accepts a property's grammar, and the element tree can still refuse a
value it does not lay out yet (see Known refusals).

## Elements

Drawn by the element tree:
`a`, `abbr`, `article`, `aside`, `b`, `blockquote`, `button`, `circle`, `code`, `div`, `em`, `figcaption`, `figure`, `footer`, `gpu-surface`, `h1`, `h2`, `h3`, `h4`, `h5`, `h6`, `header`, `hr`, `i`, `img`, `input`, `kbd`, `main`, `mark`, `nav`, `p`, `path`, `rect`, `samp`, `section`, `span`, `strong`, `svg`, `var`.

Void (they close themselves): `br`, `col`, `hr`, `img`, `input`.

Accepted by the compiler but refused by the element tree when the page runs, by the slice that brings them:

- F2d, the first item after F2c: `br`
- F3: `canvas`
- F4: `textarea`, `select`, `option`, `optgroup`, `label`, `fieldset`, `legend`, `ul`, `ol`, `li`, `pre`, `table`, `caption`, `col`, `colgroup`, `tbody`, `thead`, `tfoot`, `tr`, `td`, `th`, `meter`, `progress`
- F5: `dialog`, `details`, `summary`

## Refused in a template

Any other lowercase tag is refused. The compiler's words:

- `<title>x</title>`: <title> is not an element here: the window title is a platform concern, set from Rust
- `<slot></slot>`: slots were removed; declare a {snippet name()} inside the component tag and {render name()} in the component template
- `<blink></blink>`: unknown element `blink`: HTML elements are lowercase (div, span, button, ...); a component tag is capitalised, like `<Blink>`
- `<!-- note -->`: there is no comment syntax in a template: put the note in <script> as a `//` comment

## Inputs and attributes

`<input type>` values the tree builds: `range`, `checkbox`, `number`, `text`; an `input` with no `type` is `text`.

These HTML boolean attributes are on by being present, so the compiler refuses them from an expression (`disabled={..}`); write the literal on an element inside `{if}`, or use `bind:checked`: `allowfullscreen`, `alpha`, `async`, `autofocus`, `autoplay`, `checked`, `compact`, `controls`, `declare`, `default`, `defer`, `disablepictureinpicture`, `disableremoteplayback`, `disabled`, `formnovalidate`, `hidden`, `inert`, `ismap`, `itemscope`, `loop`, `multiple`, `muted`, `nomodule`, `noresize`, `noshade`, `novalidate`, `nowrap`, `open`, `playsinline`, `readonly`, `required`, `reversed`, `selected`, `shadowrootclonable`, `shadowrootdelegatesfocus`, `shadowrootserializable`, `switch`, `truespeed`, `typemustmatch`.

## Events

The element tree fires these; `on<name>={handler}` listens for one: `click`, `dblclick`, `pointerdown`, `pointermove`, `pointerup`, `gotpointercapture`, `lostpointercapture`, `pointercancel`, `keydown`, `keyup`, `focus`, `blur`, `focusin`, `focusout`, `input`, `change`, `selectionchange`, `paste`, `scroll`.

## CSS properties

The parser's table: each property, whether it inherits, and its grammar. Custom properties (`--name`) are accepted too.

| Property | Inherited | Grammar |
|---|---|---|
| `accent-color` | yes | `auto \| <color>` |
| `align-content` | no | `normal \| stretch \| center \| start \| end \| flex-start \| flex-end \| space-between \| space-around \| space-evenly` |
| `align-items` | no | `normal \| stretch \| center \| start \| end \| flex-start \| flex-end \| baseline` |
| `align-self` | no | `auto \| normal \| stretch \| center \| start \| end \| flex-start \| flex-end \| baseline` |
| `animation-delay` | no | `<time>#` |
| `animation-direction` | no | `[ normal \| reverse \| alternate \| alternate-reverse ]#` |
| `animation-duration` | no | `<time [0,inf]>#` |
| `animation-fill-mode` | no | `[ none \| forwards \| backwards \| both ]#` |
| `animation-iteration-count` | no | `[ infinite \| <number [0,inf]> ]#` |
| `animation-name` | no | `[ none \| <custom-ident> ]#` |
| `animation-play-state` | no | `[ running \| paused ]#` |
| `animation-timing-function` | no | `<easing-function>#` |
| `aspect-ratio` | no | `auto \| <ratio>` |
| `background-color` | no | `<color>` |
| `border-bottom-color` | no | `<color>` |
| `border-bottom-left-radius` | no | `<length-percentage [0,inf]>` |
| `border-bottom-right-radius` | no | `<length-percentage [0,inf]>` |
| `border-bottom-style` | no | `none \| hidden \| solid \| dashed \| dotted \| double` |
| `border-bottom-width` | no | `thin \| medium \| thick \| <length [0,inf]>` |
| `border-left-color` | no | `<color>` |
| `border-left-style` | no | `none \| hidden \| solid \| dashed \| dotted \| double` |
| `border-left-width` | no | `thin \| medium \| thick \| <length [0,inf]>` |
| `border-right-color` | no | `<color>` |
| `border-right-style` | no | `none \| hidden \| solid \| dashed \| dotted \| double` |
| `border-right-width` | no | `thin \| medium \| thick \| <length [0,inf]>` |
| `border-top-color` | no | `<color>` |
| `border-top-left-radius` | no | `<length-percentage [0,inf]>` |
| `border-top-right-radius` | no | `<length-percentage [0,inf]>` |
| `border-top-style` | no | `none \| hidden \| solid \| dashed \| dotted \| double` |
| `border-top-width` | no | `thin \| medium \| thick \| <length [0,inf]>` |
| `bottom` | no | `auto \| <length-percentage>` |
| `box-shadow` | no | `none \| <shadow>#` |
| `box-sizing` | no | `content-box \| border-box` |
| `color` | yes | `<color>` |
| `column-gap` | no | `normal \| <length-percentage [0,inf]>` |
| `container-name` | no | `none \| <custom-ident>+` |
| `container-type` | no | `normal \| size \| inline-size` |
| `cursor` | yes | `auto \| default \| none \| pointer \| text \| move \| grab \| grabbing \| not-allowed \| crosshair \| col-resize \| row-resize \| ew-resize \| ns-resize \| nesw-resize \| nwse-resize \| wait \| progress \| help \| zoom-in \| zoom-out` |
| `display` | no | `block \| inline \| inline-block \| flex \| inline-flex \| grid \| inline-grid \| none \| contents` |
| `fill` | yes | `none \| <color>` |
| `filter` | no | `none \| <filter-function>+` |
| `flex-basis` | no | `auto \| content \| <length-percentage [0,inf]>` |
| `flex-direction` | no | `row \| row-reverse \| column \| column-reverse` |
| `flex-grow` | no | `<number [0,inf]>` |
| `flex-shrink` | no | `<number [0,inf]>` |
| `flex-wrap` | no | `nowrap \| wrap \| wrap-reverse` |
| `font-family` | yes | `<font-family-name>#` |
| `font-feature-settings` | yes | `normal \| [ <opentype-tag> [ <integer [0,inf]> \| on \| off ]? ]#` |
| `font-size` | yes | `<length-percentage [0,inf]>` |
| `font-style` | yes | `normal \| italic \| oblique` |
| `font-variation-settings` | yes | `normal \| [ <opentype-tag> <number> ]#` |
| `font-weight` | yes | `normal \| bold \| bolder \| lighter \| <number [1,1000]>` |
| `grid-auto-columns` | no | `<track-size>+` |
| `grid-auto-flow` | no | `row \| column \| row dense \| column dense \| dense` |
| `grid-auto-rows` | no | `<track-size>+` |
| `grid-column-end` | no | `auto \| <integer> \| span <integer [1,inf]>` |
| `grid-column-start` | no | `auto \| <integer> \| span <integer [1,inf]>` |
| `grid-row-end` | no | `auto \| <integer> \| span <integer [1,inf]>` |
| `grid-row-start` | no | `auto \| <integer> \| span <integer [1,inf]>` |
| `grid-template-columns` | no | `none \| <track-list>` |
| `grid-template-rows` | no | `none \| <track-list>` |
| `height` | no | `auto \| min-content \| max-content \| fit-content \| <length-percentage [0,inf]>` |
| `justify-content` | no | `normal \| stretch \| center \| start \| end \| flex-start \| flex-end \| space-between \| space-around \| space-evenly \| left \| right` |
| `justify-items` | no | `normal \| stretch \| center \| start \| end \| flex-start \| flex-end \| baseline \| left \| right` |
| `justify-self` | no | `auto \| normal \| stretch \| center \| start \| end \| flex-start \| flex-end \| baseline \| left \| right` |
| `left` | no | `auto \| <length-percentage>` |
| `letter-spacing` | yes | `normal \| <length>` |
| `line-height` | yes | `normal \| <number [0,inf]> \| <length-percentage [0,inf]>` |
| `margin-bottom` | no | `auto \| <length-percentage>` |
| `margin-left` | no | `auto \| <length-percentage>` |
| `margin-right` | no | `auto \| <length-percentage>` |
| `margin-top` | no | `auto \| <length-percentage>` |
| `max-height` | no | `none \| min-content \| max-content \| fit-content \| <length-percentage [0,inf]>` |
| `max-width` | no | `none \| min-content \| max-content \| fit-content \| <length-percentage [0,inf]>` |
| `min-height` | no | `auto \| min-content \| max-content \| fit-content \| <length-percentage [0,inf]>` |
| `min-width` | no | `auto \| min-content \| max-content \| fit-content \| <length-percentage [0,inf]>` |
| `opacity` | no | `<number [0,1]> \| <percentage [0,100]>` |
| `outline-color` | no | `<color>` |
| `outline-offset` | no | `<length>` |
| `outline-style` | no | `auto \| none \| solid \| dashed \| dotted \| double` |
| `outline-width` | no | `thin \| medium \| thick \| <length [0,inf]>` |
| `overflow-x` | no | `visible \| hidden \| clip \| scroll \| auto` |
| `overflow-y` | no | `visible \| hidden \| clip \| scroll \| auto` |
| `padding-bottom` | no | `<length-percentage [0,inf]>` |
| `padding-left` | no | `<length-percentage [0,inf]>` |
| `padding-right` | no | `<length-percentage [0,inf]>` |
| `padding-top` | no | `<length-percentage [0,inf]>` |
| `pointer-events` | yes | `auto \| none` |
| `position` | no | `static \| relative \| absolute \| fixed \| sticky` |
| `right` | no | `auto \| <length-percentage>` |
| `row-gap` | no | `normal \| <length-percentage [0,inf]>` |
| `scrollbar-color` | yes | `auto \| <color> <color>` |
| `scrollbar-width` | no | `auto \| thin \| none` |
| `stroke` | yes | `none \| <color>` |
| `stroke-width` | yes | `<length [0,inf]> \| <number [0,inf]>` |
| `text-align` | yes | `start \| end \| left \| right \| center \| justify` |
| `text-decoration` | no | `none \| underline line-through \| line-through underline \| underline \| line-through` |
| `top` | no | `auto \| <length-percentage>` |
| `transform` | no | `none \| <transform-function>+` |
| `transition-delay` | no | `<time>#` |
| `transition-duration` | no | `<time [0,inf]>#` |
| `transition-property` | no | `none \| [ all \| <custom-ident> ]#` |
| `transition-timing-function` | no | `<easing-function>#` |
| `user-select` | no | `auto \| text \| none \| contain \| all` |
| `visibility` | yes | `visible \| hidden \| collapse` |
| `white-space` | yes | `normal \| nowrap \| pre \| pre-wrap \| pre-line` |
| `width` | no | `auto \| min-content \| max-content \| fit-content \| <length-percentage [0,inf]>` |
| `z-index` | no | `auto \| <integer>` |

## Shorthands

Each sets these longhands:

- `animation`: `animation-name`, `animation-duration`, `animation-timing-function`, `animation-delay`, `animation-iteration-count`, `animation-direction`, `animation-fill-mode`, `animation-play-state`
- `border`: `border-top-width`, `border-right-width`, `border-bottom-width`, `border-left-width`, `border-top-style`, `border-right-style`, `border-bottom-style`, `border-left-style`, `border-top-color`, `border-right-color`, `border-bottom-color`, `border-left-color`
- `border-bottom`: `border-bottom-width`, `border-bottom-style`, `border-bottom-color`
- `border-color`: `border-top-color`, `border-right-color`, `border-bottom-color`, `border-left-color`
- `border-left`: `border-left-width`, `border-left-style`, `border-left-color`
- `border-radius`: `border-top-left-radius`, `border-top-right-radius`, `border-bottom-right-radius`, `border-bottom-left-radius`
- `border-right`: `border-right-width`, `border-right-style`, `border-right-color`
- `border-style`: `border-top-style`, `border-right-style`, `border-bottom-style`, `border-left-style`
- `border-top`: `border-top-width`, `border-top-style`, `border-top-color`
- `border-width`: `border-top-width`, `border-right-width`, `border-bottom-width`, `border-left-width`
- `flex`: `flex-grow`, `flex-shrink`, `flex-basis`
- `gap`: `row-gap`, `column-gap`
- `grid-column`: `grid-column-start`, `grid-column-end`
- `grid-row`: `grid-row-start`, `grid-row-end`
- `inset`: `top`, `right`, `bottom`, `left`
- `margin`: `margin-top`, `margin-right`, `margin-bottom`, `margin-left`
- `outline`: `outline-width`, `outline-style`, `outline-color`
- `overflow`: `overflow-x`, `overflow-y`
- `padding`: `padding-top`, `padding-right`, `padding-bottom`, `padding-left`
- `transition`: `transition-property`, `transition-duration`, `transition-timing-function`, `transition-delay`

Vendor-prefixed properties are refused, except these, which are dropped:

- `-webkit-font-smoothing`: macOS browser hint; vapors sets its own text contrast (see the F1 text slice)
- `-moz-osx-font-smoothing`: Firefox twin of -webkit-font-smoothing
- `-webkit-tap-highlight-color`: mobile browser tap flash; vapors has no such flash
- `-webkit-user-select`: prefixed twin of user-select, which is supported unprefixed

## Selectors

Type, `*`, `.class`, `#id`, attribute selectors and the combinators (descendant, `>`, `+`, `~`). Pseudo-classes accepted: `active`, `checked`, `disabled`, `empty`, `enabled`, `first-child`, `first-of-type`, `focus`, `focus-visible`, `focus-within`, `hover`, `invalid`, `last-child`, `last-of-type`, `only-child`, `only-of-type`, `root`, `has`, `is`, `not`, `where`, `nth-child`, `nth-last-child`, `nth-of-type`, `nth-last-of-type`.

Refused: `any-link`, `default`, `defined`, `fullscreen`, `in-range`, `indeterminate`, `link`, `optional`, `out-of-range`, `placeholder-shown`, `read-only`, `read-write`, `required`, `target`, `valid`, `visited`, `lang`, `dir`, and every pseudo-element (`::before`, `::after`) but `::placeholder` at the end of a selector, with only `color`.

## At-rules

Accepted: `@media`, `@container`, `@supports`, `@keyframes`.

Refused: `@font-face`, `@import`, `@layer`, `@page`, `@property`, `@scope`, `@starting-style`.

## Values

Length units: `px`, `em`, `rem`, `%`, `vw`, `vh`, `ch`.

Length functions: `calc(1px + 2%)`, `min(1px, 2%)`, `max(1px, 2%)`, `clamp(1px, 2%, 3px)`, `var(--w, 1px)`.

Colours: `red`, `#abc`, `rgb(1 2 3)`, `rgba(1, 2, 3, 0.5)`, `hsl(1 2% 3%)`, `hwb(1 2% 3%)`, `lab(1 2 3)`, `lch(1 2 3)`, `oklab(0.5 0.1 0.1)`, `oklch(0.5 0.1 1)`, `transparent`, `currentColor`, `light-dark(red, blue)`.

Refused colours: `color(srgb 1 0 0)`, `color-mix(in srgb, red, blue)`.

## Scene tags

The 3D scene components (`<Scene>`, `<Group>`, `<Mesh>`, `<Camera>`) are styled through these tags, and each reads only these properties:

- `scene`: `pointer-events`, `visibility`
- `group`: `pointer-events`, `visibility`
- `mesh`: `animation-delay`, `animation-direction`, `animation-duration`, `animation-fill-mode`, `animation-iteration-count`, `animation-name`, `animation-play-state`, `animation-timing-function`, `color`, `cursor`, `opacity`, `pointer-events`, `transition-delay`, `transition-duration`, `transition-property`, `transition-timing-function`, `visibility`
- `camera`: none
- `light`: none
- `decal`: `pointer-events`, `visibility`

## Known refusals

- `white-space: nowrap` parses, but the element tree accepts only
  `white-space: normal`: the page's first update is refused, the message
  goes to `#error`, and nothing else is drawn
  ([#347](https://github.com/tomyan/vapors/issues/347)).
- `display` takes `block`, `inline`, `flex`, `grid` and `none`.
  `inline-block`, `inline-flex` and `inline-grid` are laid out only on a
  flex or grid item, and refused elsewhere when the page runs; a
  `<button>` (`inline-block` in the user-agent sheet) needs a
  `display: flex` or `grid` parent. `display: contents` is refused.
- CSS reaching a `<Mesh>` is not checked at build time: a property the
  mesh does not read (such as `padding`) passes `wisp-check` and the build
  ([#512](https://github.com/tomyan/vapors/issues/512)). Set only the
  properties listed for `mesh` under Scene tags.
