# Style bindings: `style:height={expr}`

Date: 2026-11-20. Decided by: the lead's brief for this slice. Applies
decision 3 of `2026-11-11-analysable-sources.md` (no change the compiler
cannot see) to computed values; closes the rows "sparkline bar heights",
"chart bar heights" and "message avatar colours" of
`crates/gui-bench-app/FLAGS.md`.

Status: the key, `Dom::bind_style`, the tree and the run-time refusals
land first; the compiler's `style:` parsing, its refusals and emission
(decisions 1 and 7) follow in the next change.

## Decision

1. **The form.** `style:<property>={expr}` on an element in a `.wisp`
   template. The author writes the CSS property itself; it is a literal
   in the template, never an expression. `expr` is a Rust expression
   (`bar_height(i)`, `colour.get()`) whose type the property fixes.
   It is not allowed on a component tag in this slice.
2. **Bindable properties and their types.** Lengths, a px `f32`:
   `width`, `height`, `top`, `left`, `margin-top`, `margin-right`,
   `margin-bottom`, `margin-left`, `padding-top`, `padding-right`,
   `padding-bottom`, `padding-left`. Colours, a `vapors_dom::Rgba`
   (8-bit channels): `background-color`, `color`. More are added when a
   test needs one, each with its type.
3. **It wins over the sheet, as an inline style does.** The binding is a
   declaration of the element's own, above every selector of the same
   importance.
4. **Typed all the way down.** The compiler lists the bound properties in
   the element's `StyleKey` (`bound`, as `BoundProperty` variants, not
   names) and emits one effect per binding calling
   `Dom::bind_style(node, BoundProperty::Height, BoundValue::Length(expr))`.
   `BoundValue::Length` takes an `f32` and `Color` an `Rgba`, so rustc
   refuses any other type. A tree refuses `bind_style` for a property the
   node's key does not list, and on a node without a key, so no Rust call
   sets a property the compiler did not see in a template.
5. **Shared style records.** The compiler's winners, the key and the sheet
   are the element's, whatever the bound value: elements differing only
   in bound values keep one `StyleKey` and one set of compiled winners,
   and no sheet or rule is added per value. Computed styles are shared
   per distinct (key, parent, states, toggles, bound values): 1,000 bars
   with 12 distinct heights are 12 computed styles, and an element's
   value is its own slot (`Element::bound`), read by the cascade as its
   inline declarations. (A hidden custom property per element was
   considered; it needs the same slot and adds a `var()` indirection,
   so the declaration is bound directly.)
6. **Change is narrow.** A changed value restyles the bound element only
   (its children only when its computed style changes) and asks layout
   only for what the property needs: a colour repaints and lays out
   nothing. Writing the same value changes nothing.
7. **Compile-time refusals**, each naming file, line, the form and the
   fix: a property that is not bindable (listing the bindable ones); a
   non-literal property (`style:{name}={..}`, `style:=`); `style:` on a
   component tag; a quoted value (`style:height="3px"`: the value is an
   expression) and an expression that is plainly the wrong type (a string
   literal); the same property bound twice on one element. Any other
   wrong type is rustc's error at the generated `BoundValue::Length(..)`.
8. **Run-time refusals.** A value outside the property's domain is
   refused loudly, naming the element and the template site, never
   clamped or defaulted: NaN and infinities for every length; negative
   `width`, `height` and `padding-*`; a colour channel needs no check
   (`u8`). The refusal is kept on the element and returned by every
   update while it is in the tree, as the other keyed-element refusals.
   `top`, `left` and `margin-*` may be negative.

## Deliberately not allowed

- `style={expr}` strings, still refused; a `style:` name that is
  computed; a property bound on a component tag (a parent cannot reach
  into a child's elements: `2026-11-18-style-variants.md`, decision 4).
- A Rust API that sets a property on any node: `bind_style` is for
  generated code and refuses what the key does not list.
- Clamping or defaulting a bad value; shorthands (`padding`, `margin`)
  and keywords (`height: auto` is the sheet's); both are refused as not
  bindable.
