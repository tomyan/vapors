# 6. Animation and transitions

Status: design, 2026-10-25. `vapors-css` parses and samples transitions
and `@keyframes` (Oklab colours, lengths, numbers, timing functions);
no element animates yet. touché's next need: agent edits animate the
controls from old to new values, hover and pressed states ease, cards
reorder with motion, sheets slide in, a busy line animates.

## The model

**CSS transitions and animations as the web defines them, driven by
the frame's animation tick.** A computed-value change on a property
with a `transition-*` starts a transition from the old computed value
to the new; its used value is sampled each frame at `frame.now` and
written into the used style before layout and paint (area 2, phase 3).
`@keyframes` animations start when `animation-name` applies and sample
the same way. A transition on a layout property (`width`, `height`,
`padding`) relayouts each frame; on a paint property (`color`,
`background-color`, `opacity`, `transform`) it repaints only. The
classification is the existing `honoured` table.

**Animated values never enter the reactive graph.** A transition is
used-style state in the elements crate; no signal changes per frame,
no effect runs. App code sees the start (`transitionstart`) and end
(`transitionend`, `transitioncancel`) as events and can read the
current value of a property from the element if it needs to. This is
what keeps a hundred eased sliders from touching the graph.

**Reversing follows the specification.** A transition interrupted by a
change back to its start value plays for the proportion already
travelled (css-transitions-1 §3, the reversing-adjusted start value),
so a hover that leaves half way eases back from where it was.

**Reduced motion is honoured by the framework.** When the platform
reports `prefers-reduced-motion: reduce`, durations above a stated
threshold (200 ms, from the WCAG guidance on essential motion) become
zero unless the sheet opts a property back in under
`@media (prefers-reduced-motion: no-preference)`; `@keyframes` loops
run once. The rule is in the elements layer so a component author
cannot forget it. A transition that conveys state (the busy line) is
kept because it is under the threshold.

### Value-driven motion: agent edits animating controls

An agent turn sets a slider from 0.0 to 0.4. The slider's thumb
position is a layout property derived from `value`; the value itself is
app state, so a CSS transition on the thumb's `left` or `transform`
animates the thumb, and the readout number needs the same. Design: the
range and number inputs gain a UA transition on their visual parts and
on their text (`--vapors-value` as an animatable custom property that
the readout reads through `counter`-like formatting is not available in
CSS; instead the input animates its *displayed* value from the previous
to the new over `transition-duration`). The document's value changes
at once; the display catches up; no `input` or `change` fires, because
a programmatic value set fires none in a browser either. This is what
"controls animating" means for a value control, and it needs no app
code: the card sets `value`, the control animates. A drag by the user
sets the displayed value directly with no transition, as browsers do.

### FLIP for reorder

A keyed `{for}` whose rows move gets an `animate:` directive in Svelte;
vapors does the same with `animate` on the row element: the elements
layer records each moved row's old box, lays out the new, and starts a
`transform` transition from the old position to the new (First, Last,
Invert, Play). The directive is a template attribute,
`<li animate:flip={200}>`, compiled to a flag on the `{for}` block and
read by the layout phase. Only rows the keyed diff moved animate.

### Enter and leave

Svelte's `in:`/`out:`/`transition:` directives have one CSS equivalent
pair: `@starting-style` for enter (a rule that applies for the first
frame after insertion so a transition can start from it) and
`transition-behavior: allow-discrete` with a delayed `display: none`
for leave. vapors implements both: an inserted element's first style is
computed with `@starting-style` rules applied, then its real style, and
the difference starts transitions; an element removed by `{if}` or
`{for}` whose style has a transition on `display` is kept in the tree
as a leaving element until its transitions end, then removed. This is
the web's current mechanism and it is testable against Chromium; a
directive syntax would need its own oracle.

### Keyframe animations

`@keyframes` with `animation-*` as specified; the sampler exists. Used
for the busy progress line, a skeleton shimmer, a spinner.
`animation-play-state` and `animationend` events included.
`animation-composition` and additive animations are refused.

### Scroll-driven and view transitions

Not implemented. `scroll-timeline` and the View Transitions API are
refused by name. touché's sheet and panel transitions are
`transform` and `opacity` transitions.

### Scene animation (area 9)

3D scene objects do not animate through CSS transitions per object;
bulk per-frame animation is a frame task that writes GPU buffers. The
exception is a scene object's presentational CSS subset (opacity,
emissive highlight), which transitions like an element's property,
one object at a time, for selection and hover.

## Taken from where

- **CSS Transitions 1, CSS Animations 1, Web Animations 1**: the
  timing model (delay, duration, timing function, iteration, direction,
  fill), the reversing rule, `transitionstart`/`end`/`cancel`. Taken as
  the specification.
- **`@starting-style` and `transition-behavior: allow-discrete`**: the
  web platform's enter/leave mechanism, shipped in Chromium 117 and
  Safari 17.5; taken instead of Svelte's directives because Chromium
  is the oracle.
- **Svelte's `animate:flip`**: the FLIP directive on keyed rows, with
  the same scope (only moved rows). Svelte's `crossfade` and custom
  transition functions are not taken.
- **GPUI and Flutter**: implicit animation of a value control's display
  is Flutter's `AnimatedSlider` idea; here it is a UA behaviour of the
  input rather than a widget variant.
- **WCAG 2.3.3 and the `prefers-reduced-motion` guidance**: the
  framework-level rule.

## Deliberately left out

- Spring physics and gesture-driven animations with velocity hand-off
  (Framer Motion, iOS). Momentum scrolling is area 3's and has its own
  curve; everything else is a timing function. If a spring is wanted,
  it is a `linear()` timing function sampled from a spring, which
  Chromium also supports.
- The Web Animations API as a script API (`element.animate`). Scripts
  write classes and styles; the cascade does the rest.
- View Transitions, scroll-driven animations, `animation-composition`,
  motion paths.
- Animating layout properties on more than a few elements at once; it
  works, and the budget overlay will say when it costs too much.

## Oracles

| Part | Oracle |
|---|---|
| Sampling and timing functions | `vapors-css`'s tests (built), and WPT `css/css-transitions/` and `css/css-animations/` testharness cases that read `getComputedStyle` at times set through the Web Animations API's `currentTime`; ported with a stepped clock. The reversing rule from the specification's worked examples |
| Transition start, end, cancel events | Chromium's event log for the same HTML with a scripted class change (the page-contract runner) |
| `@starting-style` and leave | Chromium screenshots at fixed times via virtual time budget, compared with the paint rules; computed values at `t` via `Animation.currentTime` |
| FLIP | A keyed reorder test counting one `transform` transition per moved row and none otherwise; Chromium has no FLIP primitive, so the geometry oracle checks the end state only |
| Value controls animating | A test with a stepped clock: the displayed value at each frame, no `input` or `change` fired; a user drag produces no transition and fires `input` per move |
| Reduced motion | A test under `Environment { reduced_motion: true }`: durations zero, loops once, the busy line still animating |
| Performance | The stress demo gains a hundred transitioning elements; frame time per phase recorded (area 2) |

## Slices

| Slice | Delivers | touché use |
|---|---|---|
| A1 transitions | Transitions on paint and layout properties, the tick, events, reversing, `transform` 2D and `opacity` as paint properties (opacity as a group alpha needs A2; A1 animates opacity on leaf elements, which covers controls) | Hover and pressed states ease; a card's highlight when an agent touches it |
| A2 value controls animate | The displayed value transition on range and number; the thumb's UA transition | Agent edits animate the sliders and readouts |
| A3 opacity layers and `filter: blur()` | An offscreen target per group | The before/after fade; a sheet over a blurred canvas |
| A4 `@keyframes` | Animations, play state, events | The busy progress line, the spinner |
| A5 FLIP | `animate:flip` on keyed rows | Reordering layer cards |
| A6 enter and leave | `@starting-style`, `allow-discrete`, leaving elements | Toasts and sheets slide in and out; a removed layer card leaves |
| A7 reduced motion | The framework rule, from area 7's preference | Acceptance for the accessibility pass |
