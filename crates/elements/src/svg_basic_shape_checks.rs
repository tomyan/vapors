//! What a `<rect>` or `<circle>` of an inline svg refuses at update
//! time (one-scene S4b), as `svg_path_checks` does for a path. The
//! compiler refuses what a template writes literally at build time,
//! naming its line (`vapors_compiler`'s `basic_shape_attributes`). This
//! catches what the build cannot see: a bound value, and one set by
//! `set_attribute`. Both read through `vapors_shapes::refusal` and
//! `not_taken`. Refused: a place outside an `svg`; attributes outside
//! its geometry and paint, `transform` among them; a number SVG would
//! read differently from a bare one in viewBox units (a unit, a
//! percentage); a negative size or radius; an absent size; content.

use std::collections::BTreeMap;

use vapors_dom::{NodeId, SCOPE_ATTRIBUTE};

use crate::arena::Arena;

type Attributes = BTreeMap<String, String>;

/// Why `node`, a `tag` element (`rect` or `circle`) with `attributes`,
/// is refused.
pub(crate) fn reasons(
    arena: &Arena,
    node: NodeId,
    tag: &str,
    attributes: &Attributes,
) -> Vec<String> {
    let mut found: Vec<String> = placement(arena, node, tag).into_iter().collect();
    found.extend(not_taken(tag, attributes));
    found.extend(numbers(tag, attributes));
    if !arena.children(node).is_empty() {
        found.push(format!("holds content, and a {tag} holds none; remove it"));
    }
    found
}

/// A refusal when `node`'s parent is not an `svg`.
fn placement(arena: &Arena, node: NodeId, tag: &str) -> Option<String> {
    let parent = arena.get(node, "svg_basic_shape_checks").parent;
    if parent.is_some_and(|parent| crate::svg::is(arena, parent, "svg")) {
        return None;
    }
    Some(format!(
        "is not inside an <svg>; a {tag} is drawn only as part of an svg, so move it into one"
    ))
}

/// Each attribute a `tag` element does not take. The scoping attribute
/// is accepted, as `svg_checks::outside` accepts it.
fn not_taken(tag: &str, attributes: &Attributes) -> Vec<String> {
    let allowed = vapors_shapes::allowed(tag).expect("a basic shape");
    attributes
        .keys()
        .filter(|name| !allowed.contains(&name.as_str()) && *name != SCOPE_ATTRIBUTE)
        .map(|name| vapors_shapes::not_taken(tag, name))
        .collect()
}

/// Each geometry attribute's refusal.
fn numbers(tag: &str, attributes: &Attributes) -> Vec<String> {
    let names = vapors_shapes::geometry_names(tag).expect("a basic shape");
    names
        .iter()
        .filter_map(|name| vapors_shapes::refusal(name, attributes.get(*name).map(String::as_str)))
        .collect()
}
