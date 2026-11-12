# Analysable sources only: classes, styles and custom properties

Date: 2026-11-11. Decided by: Tom.

## Decision

1. **Classes come from booleans and enums, never strings.** A template
   toggles one literal class with `class:name={bool}`, and chooses among
   a known set with an enum. `class={expr}` with a string expression is
   refused, naming the two forms; its 8 current uses are migrated by a
   script. Tom: "Boolean and enum features, no strings."
2. **Finish-line properties 7 and 8 are restated** (agreed with Tom):
   - 7: the compiler works out the winning declarations for each
     template element at build time; computed values still depend on
     viewport, scale, colour scheme and root font size at run time.
   - 8: one style record per distinct style, not per element; total
     memory per element bounded by the per-box layout records, which
     stay because they make edits cheap.
3. **No API that allows change the analysis cannot see.** Tom: "don't
   think we want API that allows dynamism we can't optimise." vapors
   gets no Rust API for setting custom properties by name at run time.
   A custom property changes through a template binding whose property
   name is literal, so the compiler knows which `var()` reads it. Any
   existing way in that the analysis cannot see (a `style={expr}`
   string, a raw setter on a node the compiler created, a stylesheet
   added after an analysed mount) is closed the same way: refused,
   naming the template form to use.

## Context

`~/.cache/handoffs/fable-analysis.md` (lead's handoff, 2026-11-11)
found: `class={expr}` is a Rust `String` in 8 places, all layout names;
per-element `ComputedStyle` dominates memory; and no Rust API sets a
custom property today (`set_property` sets element properties such as
`value` and `checked`), so the SVG brief's sentence naming one was
wrong. The brief and `docs/plans/perf-finish-line.md` are updated to
match this record.
