//! What a `<rect>` or `<circle>` of an inline svg refuses at update
//! time (one-scene S4b), as `svg_path_checks` does for a path. The
//! compiler refuses a literal value at build time, naming its line
//! (`vapors_compiler`'s `basic_shape_attributes`). This catches what the
//! build cannot see: a bound value, and one set by `set_attribute`.
//! Both read through `vapors_shapes::refusal`. Refused: a place
//! outside an `svg`; attributes outside its geometry and paint,
//! `transform` among them; a number SVG would read differently from a
//! bare one in viewBox units (a unit, a percentage); a missing or zero
//! size; content.

use std::collections::BTreeMap;

use vapors_dom::NodeId;

use crate::arena::Arena;
use crate::svg_checks::outside;

/// Why `node`, a `tag` element (`rect` or `circle`) with `attributes`,
/// is refused.
pub(crate) fn reasons(
    arena: &Arena,
    node: NodeId,
    tag: &str,
    attributes: &BTreeMap<String, String>,
) -> Vec<String> {
    let mut found = Vec::new();
    let parent = arena.get(node, "svg_basic_shape_checks").parent;
    if !parent.is_some_and(|parent| crate::svg::is(arena, parent, "svg")) {
        found.push(format!(
            "is not inside an <svg>; a {tag} is drawn only as part of an svg, so move it into one"
        ));
    }
    let allowed = vapors_shapes::allowed(tag).expect("a basic shape");
    found.extend(outside(attributes, allowed, &format!("<{tag}>")));
    let names = vapors_shapes::geometry_names(tag).expect("a basic shape");
    found.extend(names.iter().filter_map(|name| {
        vapors_shapes::refusal(name, attributes.get(*name).map(String::as_str))
    }));
    if !arena.children(node).is_empty() {
        found.push(format!("holds content, and a {tag} holds none; remove it"));
    }
    found
}
