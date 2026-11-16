//! What a `path` of an inline `<svg>` icon refuses (docs/icons.md, The
//! subset): a place outside an `svg`, attributes outside the subset,
//! path data `vapors_shapes::IconPath` does not read (the parser paint
//! uses, so a path that passes rasterises), and content. Its paint,
//! which its presentation attributes and CSS give, is checked on its
//! used style (`svg_style_checks.rs`).

use std::collections::BTreeMap;

use vapors_dom::NodeId;
use vapors_shapes::IconPath;

use crate::arena::Arena;
use crate::svg_checks::outside;

/// The attributes a `path` takes.
const ATTRIBUTES: [&str; 4] = ["d", "fill", "stroke", "stroke-width"];

/// Why `node`, a `path` with `attributes`, is refused.
pub(crate) fn reasons(
    arena: &Arena,
    node: NodeId,
    attributes: &BTreeMap<String, String>,
) -> Vec<String> {
    let mut found = Vec::new();
    let parent = arena.get(node, "svg_path_checks").parent;
    if !parent.is_some_and(|parent| crate::svg::is(arena, parent, "svg")) {
        found.push("is not inside an <svg>; a path is drawn only as part of an svg icon, so move it into one".to_owned());
    }
    found.extend(outside(attributes, &ATTRIBUTES, "<path>"));
    found.extend(data(attributes.get("d")));
    if !arena.children(node).is_empty() {
        found.push("holds content, and a path holds none; remove it".to_owned());
    }
    found
}

fn data(d: Option<&String>) -> Option<String> {
    let Some(d) = d else {
        return Some(
            "its d attribute is required: the path data, such as d=\"M2 8h12\"".to_owned(),
        );
    };
    let error = IconPath::parse(d).err()?;
    Some(format!("its d attribute {d:?}: {error}"))
}
