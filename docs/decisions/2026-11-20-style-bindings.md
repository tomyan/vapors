# Style bindings: `style:height={expr}`

Date: 2026-11-20. Decided by: the lead's brief. Applies decision 3 of
`2026-11-11-analysable-sources.md` to computed values; closes the
sparkline, chart bar and avatar rows of `crates/gui-bench-app/FLAGS.md`.

Status: the key, `Dom::bind_style`, the tree and the run-time refusals
land first; the compiler's `style:` parsing, refusals and emission
(decisions 1, 7) follow. Until then the compiler emits `bound: &[]`
and `bind_style` refuses every call on its keys, loudly; that change
updates the compiler tests pinning it.

1. **Form.** `style:<property>={expr}` on an element (not a component
   tag); the property is a literal, `expr` a Rust expression of the
   type the property fixes.
2. **Properties.** Px `f32`: `width`, `height`, `top`, `left`, `margin-*`,
   `padding-*`. `vapors_dom::Rgba`: `background-color`, `color`. More
   when a test needs one.
3. **Wins over the sheet**, as inline style does.
4. **Typed.** The compiler lists the bound properties in the element's
   `StyleKey` (`bound`, `BoundProperty` variants, each once: a key
   listing one twice is refused by `StyleKey::shape_refusal`) and emits
   `Dom::bind_style(node, BoundProperty::Height, BoundValue::Length(e))`,
   so rustc refuses any other type. A tree refuses a property the key
   does not list and a node without a key, so no Rust call sets a
   property the compiler did not see in a template.
5. **Shared records.** Key, winners and sheet are the element's whatever
   the value: no sheet or rule per value. Computed styles are shared per
   distinct (key, parent, states, toggles, bound values in property
   order): 1,000 bars of 12 heights are 12 styles. The value is the
   element's own slot (`Element::bound`), read by the cascade as inline
   declarations. A keyed element's `style` attribute is written only by
   the key and by a binding (`set_attribute` refuses it there), so the
   slot and the attribute cannot diverge. (A hidden custom property would
   need the same slot plus a `var()`.)
6. **Narrow change.** A changed value restyles the bound element only
   and asks layout only what the property needs (a colour, nothing).
   The same value changes nothing.
7. **Compile-time refusals**, naming file, line, form and fix: a
   property not bindable (listing those that are); a non-literal
   property; `style:` on a component tag; a quoted value or plainly
   wrong-typed expression; a property bound twice. Other types: rustc.
8. **Run-time refusals**, naming element and site, never clamped: NaN
   and infinities for every length; negative `width`, `height` and
   `padding-*` (`top`, `left`, `margin-*` may be negative). Kept on the
   element, returned by every update while it is in the tree.

Not allowed: `style={expr}` strings; a computed name; a binding on a
component tag (`2026-11-18-style-variants.md`, 4); a Rust API setting
a property on any node; shorthands and keywords.
