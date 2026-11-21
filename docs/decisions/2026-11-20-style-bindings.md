# Style bindings: `style:height={expr}`

Date: 2026-11-20. Decided by: the lead's brief, and Tom's choice the
same day to complete the whole feature before merging. Applies decision
3 of `2026-11-11-analysable-sources.md` to computed values; closes the
sparkline row of `crates/gui-bench-app/FLAGS.md` (the chart bar and
avatar rows follow with their own conversions).

1. **Form.** `style:<property>={expr}` on an element (not a component
   tag); the property is a literal, `expr` a Rust expression of the
   type the property fixes: a signal read, a memo or a plain value.
2. **Properties.** Px `f32`: `width`, `height`, `top`, `left`, `margin-*`,
   `padding-*`. `vapors_dom::Rgba` (re-exported by `vapors-elements`):
   `background-color`, `color`. More when a test needs one.
3. **Wins over the sheet**, as inline style does (an `!important` sheet
   declaration still beats it, as it beats inline style).
4. **Typed.** The compiler lists the bound properties in the element's
   `StyleKey` (`bound`, `BoundProperty` variants, each once: a key
   listing one twice is refused by `StyleKey::shape_refusal`) and emits
   one effect per element calling
   `Dom::bind_style(node, BoundProperty::Height, BoundValue::Length(e))`
   for each binding, so rustc refuses any other type. A tree refuses a
   property the key does not list, a node without a key and a text
   node, so no Rust call sets a property the compiler did not see in a
   template.
5. **Shared records.** Key, winners and sheet are the element's whatever
   the value: no sheet or rule per value. Computed styles are shared per
   distinct (key, parent, states, toggles, bound values in property
   order): 1,000 bars of 12 heights are 12 styles. The value is the
   element's own slot (`Element::bound`), read by the cascade as inline
   declarations after the key's literal `style`. A keyed element's
   `style` attribute is written only by the key and by a binding
   (`set_attribute` refuses it there, `StyleKey::refusal`), so the slot
   and the attribute cannot diverge. The styles analysis treats each
   bound longhand as a variable winner (an inline value it cannot know).
6. **Narrow change.** A changed value restyles the bound element only
   and asks layout only what the property needs (a colour, nothing).
   The same value changes nothing.
7. **Compile-time refusals**, naming file, line, form and fix: a
   property not bindable (listing those that are); a non-literal
   property (`style:{name}`, an empty name); `style:` on a component
   tag; a quoted value; a value of a plainly wrong type (a string
   literal for a length, a bool); a property bound twice. Other types: rustc.
   A literal `style` may set the same property: the binding wins.
8. **Run-time refusals**, naming element and site, never clamped: NaN
   and infinities for every length; negative `width`, `height` and
   `padding-*` (`top`, `left`, `margin-*` may be negative). Kept on the
   element, returned by every update while it is in the tree. A refused
   call changes nothing: the element keeps its last valid value.
9. **One answer.** `RecordingDom` and `Document` give the same accept or
   refuse and the same `style` attribute for every shared case
   (`crates/elements/tests/bound_style_parity.rs`).

Not allowed: `style={expr}` strings; a computed name; a binding on a
component tag (`2026-11-18-style-variants.md`, 4); a Rust API setting
a property on any node; shorthands and keywords.

## Iterations

Each is tests first, the check broken and restored (recorded in the
commit message).

1. Key and `bound_style` types, `RecordingDom::bind_style` (tests:
   `bound_declarations`, `recording_bind_style`).
2. `Document::bind_style`: slot, cascade read, share key, narrow change
   (`bound_style`, 1,000 elements), refusals and the parity test.
3. Compiler: parse `style:`, refusals, key `bound`, the effect, the
   analysis' variable winner (`codegen_style_bindings`).
4. Docs, the sparkline conversion, `FLAGS.md`.
