# The CSS engine

`crates/css` (`vapors-css`) turns a stylesheet string into computed
styles for an element tree. It was written fresh against sumi's test
corpus, module by module, with sumi's tests translated first; nothing
was transliterated from Go. The modules in dependency order:

| Module | What it does |
|---|---|
| `token` | CSS Syntax Level 3 tokens; every token carries its line and column |
| `parse` | Component values, rules, declarations, `@media`, `@container`, `@supports`, `@keyframes` |
| `selector` | Selector parsing, specificity, matching against the `Element` trait |
| `value` | Lengths, colours (CSS Color Level 4), `calc()`, `var()` |
| `syntax` | The value grammar language the property table is written in |
| `property` | The table of longhands, the shorthands, declaration parsing |
| `cascade` | Origins, importance, specificity, order, inheritance, computed values, `@media` and friends |
| `animation` | Transitions, `@keyframes`, timing functions, interpolation, `sample(t)` |
| `tree` | A small arena tree that implements `Element`, for tests and `css_dump` |

```
cargo nextest run -p vapors-css
cargo clippy --target wasm32-unknown-unknown -p vapors-css --all-targets -- -D warnings
cargo run -p vapors-css --example css_dump -- sheet.css [--dark]
```

The crate reads no files and has no clock, so it compiles for wasm32;
the second command proves it. `css_dump` is a native-only example that
prints the computed style of a fixed tree for a stylesheet file.

## Using it

```rust
let sheet = vapors_css::parse(text)?;                 // refuses with line and column
let cascade = Cascade::new().user_agent(ua).author(sheet);
let styles = cascade.compute_tree(&root, &env)?;      // (element, ComputedStyle) in document order
let width = styles[0].1.get_by_name("width");         // a Value; .to_css() for text
```

`Element` is the trait the matcher walks: tag, id, classes, attributes,
parent, siblings, first child, emptiness, state flags and the nearest
container's laid-out size. Implement it on your tree as a cheap handle;
`tree::Node` is the reference implementation, with O(1) parent, first
child and sibling links and O(n) `children()`. State flags
(`:hover`, `:focus`, `:focus-visible`, `:focus-within`, `:active`,
`:checked`, `:disabled`, `:enabled`, `:invalid`) are set by the tree layer; the
engine infers none from another, so whoever moves focus also sets
`FOCUS_WITHIN` on the ancestors, and `ENABLED` is set only on form
controls that are not disabled.

`Environment` carries what the stylesheet cannot know: viewport size,
colour scheme, reduced-motion preference, resolution, the root font size
and the `ch` ratio. `@container` reads the container size from the
element; before layout there is none and the query does not match, so
run a second pass after layout (sumi did the same).

## Supported

- Selectors: type, class, id, `*`, attributes with every operator and the
  `i` flag, the four combinators, the state pseudo-classes above,
  `:empty`, `:root`, `:first-child`, `:last-child`, `:only-child`,
  `:first-of-type`, `:last-of-type`, `:only-of-type`, `:nth-child()`,
  `:nth-last-child()`, `:nth-of-type()`, `:nth-last-of-type()` with the
  full An+B grammar, `:not()`, `:is()`, `:where()`, `:has()` (relative
  selectors, no nesting). Specificity per Selectors Level 4.
- Values: `px`, `em`, `rem`, `%`, `vw`, `vh`, `ch`; colours as named,
  hex, `rgb()`, `hsl()`, `hwb()`, `lab()`, `lch()`, `oklab()`, `oklch()`,
  `transparent`, `currentColor`, `light-dark()`; `calc()`, `min()`,
  `max()`, `clamp()` with type checking; `var()` with fallbacks.
- Properties: everything in `property/table.rs` plus the shorthands in
  `property/shorthand.rs`. The flex and grid entries match Taffy 0.15's
  `Style`. Custom properties inherit and are readable from the computed
  style.
- At-rules: `@media` (types, `not`/`only`, `and`/`or`/`not`, comma lists,
  `width`, `height`, `resolution` with `min-`/`max-` and the range
  syntax, `prefers-color-scheme`, `prefers-reduced-motion`),
  `@container` (size features, no names), `@supports` (property and
  value checked), `@keyframes`.
- Cascade: user-agent and author origins, `!important`, specificity,
  source order, `inherit`/`initial`/`unset`, inheritance per the table.
- Motion: `transition-*`, `animation-*`, their shorthands, the named
  curves, `cubic-bezier()`, `steps()`; interpolation of lengths,
  numbers, and colours in Oklab; `Transition::sample` and
  `Animation::sample` take elapsed milliseconds and return values.
  Lengths interpolate when both ends have the same unit (px with px,
  % with %); two different units snap at the halfway point, which is
  the specification's behaviour for values with no common type, since
  the computed style has already turned every unit but `%` into px.

## Refused

Every refusal names the line and column and says what would be
accepted. The parser refuses rather than drops:

- unknown properties and unknown at-rules (`@font-face`, `@import`,
  `@layer`: sumi had no layers either);
- values that fail the property's grammar, named with the grammar;
- unknown pseudo-classes and all pseudo-elements (`::before`,
  `::after`: generate the content as an element);
- sumi's terminal extras by name, with the replacement: `bold`, `dim`,
  `italic`, `underline`, `strikethrough`, `inverse`, `border-title`,
  the `cell` unit, `opacity: dim`, the box-drawing border styles;
- vendor-prefixed properties, except the short allowlist in
  `property/names.rs` which is dropped on purpose, each with a reason;
- grid line names, elliptical `border-radius`, `@media` features and
  types the engine does not evaluate.

At compute time, a `var()` that does not resolve, a `var()` cycle, and a
substituted value that fails its grammar are errors naming the
declaration, where the specification (and sumi) would drop the
declaration. The caller decides what to show.

## Incremental restyle

`Cascade::restyle_scope(flag)` says which elements a flip of one state
flag on element E can restyle, from the selectors that use that flag:

- E itself when the pseudo-class is on a subject compound;
- E's descendants when it is on a non-subject compound joined to the
  subject by child or descendant combinators only;
- E's later siblings and their descendants when any combinator between
  that compound and the subject is `+` or `~`;
- the whole tree when it appears inside `:has()`, or on a non-subject
  compound of a selector nested in `:is()`, `:not()` or `:where()`.

Only the combinator right after the flagged compound decides between
descendants and later siblings: everything further right stays inside
what that combinator selects (`.a:hover .b + .c` is in `.a`'s subtree).
A rule in the element scope whose declarations reach into the subtree
widens it to the element's descendants: an inherited property,
`font-size` (which drives `em` and `ch` below), or a custom property.

The scopes union across rules and over-approximate: a flagged element
is restyled whenever a rule could change, never less.
`RestyleScope::elements(&e)` enumerates the set in document order.

Rules are bucketed by their subject compound (id, else first class,
else tag, else universal); `Cascade::candidates(&e)` is the rules the
matcher runs on for an element, in source order.

## Adding a property

1. Add a `def(name, inherited, initial, syntax)` line to
   `property/table.rs`, keeping the table sorted. The grammar language
   is in `syntax/mod.rs`; a new `<type>` goes in `syntax/types.rs` or
   `syntax/special.rs`.
2. The initial value must parse under the grammar and serialize to
   itself: `property_table.rs` checks every entry.
3. A shorthand goes in `property/shorthand.rs` with its longhands and an
   expansion, and a test in `property_shorthand.rs`.
4. If the computed value needs more than unit and colour resolution,
   extend `cascade/compute_resolve.rs`.

## Differences from sumi, in one place

Pixel units are real; colours are sRGB with alpha and are not degraded;
there are no terminal extras; values are validated at parse; state
pseudo-classes match on any compound; `:is()` and `:not()` take complex
selectors; `:has()` exists; `@supports` checks the value; colours
interpolate in Oklab; lengths interpolate as floats; failures are errors
with positions instead of silent drops.
