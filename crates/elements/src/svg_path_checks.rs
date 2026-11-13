//! What a `path` of an inline `<svg>` icon refuses (docs/icons.md, The
//! subset): a place outside an `svg`, attributes outside the subset,
//! path data `vapors_shapes::IconPath` does not read (the parser paint
//! uses, so a path that passes rasterises), a fill, a stroke other than
//! `currentColor`, a `stroke-width` that is not a positive number, and
//! content.

use std::collections::BTreeMap;

use vapors_dom::NodeId;
use vapors_shapes::IconPath;

use crate::arena::Arena;
use crate::svg::positive;
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
    found.extend(paint(attributes));
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

/// `fill`, `stroke` and `stroke-width`.
fn paint(attributes: &BTreeMap<String, String>) -> Vec<String> {
    let get = |name| attributes.get(name).map(String::as_str);
    let mut found = Vec::new();
    if get("fill") != Some("none") {
        found.push("needs fill=\"none\": SVG fills a path black by default, and the icon subset draws strokes only".to_owned());
    }
    if get("stroke") != Some("currentColor") {
        found.push("needs stroke=\"currentColor\": an icon is stroked in its svg's CSS color, so set the colour there".to_owned());
    }
    if let Some(width) = get("stroke-width").filter(|width| positive(width).is_none()) {
        found.push(format!("its stroke-width {width:?} is not a positive number with no unit; give viewBox units as a bare number, such as stroke-width=\"1.5\", or leave it out for 1"));
    }
    found
}
