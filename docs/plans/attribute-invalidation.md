# Attribute invalidation: restyle only what the selectors can reach

Finding (#296 review): `mark_changed` marks the whole subtree on any attribute write
(`crates/elements/src/dom_impl.rs:188-194` -> `restyle.rs:89-100`), while `mark_flag`
already narrows by where the selectors use the flag (`restyle.rs:103-118`). Off-path's
Reclass control frame: 514 M instructions per frame at 1k items from invalidation alone.

1. PR A (vapors-css, ~250 lines): `Cascade::restyle_scope_for_class(name)` and
   `restyle_scope_for_attribute(name)` (`id` through the attribute form): the same
   `RestyleScope` the flag path gets from `restyle_scope_of_selectors`, computed by where
   `.name`, `[name…]` or `#name` sits in each selector (subject only -> element; an ancestor
   compound -> descendants; after `+`/`~` -> later siblings and theirs; inside `:has()` or a
   nested list whose position cannot be told -> everything; read by no selector -> nothing).
   One walk shared with the flag scope (`selector_scope` over a "compound reads X"
   predicate), so a new selector form fails there once. Table tests per shape, each proven
   by breaking. The runtime answers it (it has every sheet); B1's per-key facts stay what
   `wisp-check --styles` reports.
2. PR B (vapors-elements, ~350 lines): `write_attribute` does nothing when the value is
   unchanged (setting `class="moved"` again restyles 3,001 elements today); otherwise
   `mark_changed(node, name, old, new)`: for `class`, the union of scopes over the names
   that differ between old and new lists, memoized per name per `Kept` (cleared when sheets
   change); other attributes, the attribute's scope; `dirty.element` when the scope is the
   element alone (children restyled only if the element's computed style changes, the
   inheritance pass). Debug oracle: after a narrowed update, restyle everything from scratch
   and compare every kept `ComputedStyle`, panicking naming element, property and attribute
   (shape of `check_shared`); counts `restyle_narrowed` / `restyle_subtree` pinned on
   off-path Animate (container alone restyled; oracle proven by forcing `element` scope for a
   descendant-reaching class).
3. Measurement in PR B: off-path Animate style instructions per frame at 1,000 items before
   and after, with #296's Reclass control frame.

Out of scope: compiler recording reach per class (revisit if the per-name memo shows in a profile).
Status: waits until after the site launch (Tom, 2026-11-14 budget).
