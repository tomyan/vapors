//! What an inline `<svg>` icon refuses (docs/icons.md, The subset),
//! before layout, naming the element, the attribute and the fix: on an
//! `svg`, attributes outside the subset, its `viewBox`, `width` and
//! `height`, how it is named to the a11y tree, and children other than
//! `path`s and whitespace; on a `path`, `svg_path_checks`; CSS on
//! either, `svg_style_checks`.

use std::collections::BTreeMap;

use crate::node_map::NodeMap;

use vapors_dom::{NodeId, SCOPE_ATTRIBUTE};

use crate::arena::Arena;
use crate::check::describe;
use crate::error::ElementError;
use crate::node::Kind;
use crate::style::Used;
use crate::svg::{NAMESPACE, positive, view_box};

/// The attributes an `svg` takes.
const ATTRIBUTES: [&str; 9] = [
    "viewBox",
    "width",
    "height",
    "aria-hidden",
    "role",
    "aria-label",
    "id",
    "class",
    "xmlns",
];

/// The refusals of every `svg` and `path` among `nodes`.
pub(crate) fn refusals(
    arena: &Arena,
    nodes: &[NodeId],
    used: &NodeMap<Used>,
    scale: f32,
) -> Vec<ElementError> {
    nodes
        .iter()
        .flat_map(|&node| {
            let element = describe(arena, node);
            reasons(arena, node, used, scale)
                .into_iter()
                .map(move |reason| {
                    ElementError::UnsupportedAttribute(format!("{element}: {reason}"))
                })
        })
        .collect()
}

fn reasons(arena: &Arena, node: NodeId, used: &NodeMap<Used>, scale: f32) -> Vec<String> {
    let Some(element) = arena.get(node, "svg_checks").element() else {
        return Vec::new();
    };
    let mut found = match element.tag.as_str() {
        "svg" => svg_reasons(arena, node, &element.attributes, scale),
        "path" => crate::svg_path_checks::reasons(arena, node, &element.attributes),
        _ => return Vec::new(),
    };
    found.extend(crate::svg_style_checks::reasons(arena, node, used));
    found
}

fn svg_reasons(
    arena: &Arena,
    node: NodeId,
    attributes: &BTreeMap<String, String>,
    scale: f32,
) -> Vec<String> {
    let mut found = outside(attributes, &ATTRIBUTES, "<svg>");
    found.extend(sizes(attributes));
    found.extend(crate::svg_limit::reason(attributes, scale));
    found.extend(named(attributes));
    found.extend(namespace(attributes));
    found.extend(children(arena, node));
    found
}

/// Each of `attributes` not in `allowed`, which `tag` does not take.
/// The compiler's scoping attribute is always accepted: a component with
/// a `<style>` tags every element with it, and it is not painted or
/// exposed to the a11y tree.
pub(crate) fn outside(
    attributes: &BTreeMap<String, String>,
    allowed: &[&str],
    tag: &str,
) -> Vec<String> {
    attributes
        .keys()
        .filter(|name| !allowed.contains(&name.as_str()) && *name != SCOPE_ATTRIBUTE)
        .map(|name| format!("its {name} attribute is not in the icon subset (docs/icons.md), which takes {} on an {tag}; remove it", allowed.join(", ")))
        .collect()
}

/// The `viewBox`, `width` and `height`, and that the two agree.
fn sizes(attributes: &BTreeMap<String, String>) -> Vec<String> {
    let view = attributes.get("viewBox").map(|text| (text, view_box(text)));
    let mut found: Vec<String> = view_box_reason(view).into_iter().collect();
    let size = ["width", "height"].map(|name| size(attributes, name));
    found.extend(size.iter().filter_map(|size| size.as_ref().err().cloned()));
    if let (Some((_, Some([.., vw, vh]))), [Ok(w), Ok(h)]) = (view, size) {
        found.extend(ratio([w, h], [vw, vh]));
    }
    found
}

/// Why the `viewBox` (its text and numbers, when given) is refused.
fn view_box_reason(view: Option<(&String, Option<[f32; 4]>)>) -> Option<String> {
    match view {
        None => Some("its viewBox attribute is required: four numbers, min-x min-y width height, such as viewBox=\"0 0 16 16\"".to_owned()),
        Some((text, None)) => Some(format!("its viewBox {text:?} is not four numbers (min-x min-y width height, separated by spaces or one comma); write it as viewBox=\"0 0 16 16\"")),
        Some((text, Some([.., w, h]))) if w <= 0.0 || h <= 0.0 => Some(format!("its viewBox {text:?} has a width or height not above 0, which draws nothing; give both above 0")),
        Some(_) => None,
    }
}

fn size(attributes: &BTreeMap<String, String>, name: &str) -> Result<f32, String> {
    let Some(text) = attributes.get(name) else {
        return Err(format!(
            "its {name} attribute is required: the icon's {name} in CSS px, such as {name}=\"16\""
        ));
    };
    positive(text).ok_or(format!("its {name} {text:?} is not a positive number with no unit; give CSS px as a bare number, such as {name}=\"16\""))
}

/// A refusal when `[w, h]` is not in the ratio of the view box's
/// `[vw, vh]` (to a part in 10^5, f32's rounding of the four numbers).
fn ratio([w, h]: [f32; 2], [vw, vh]: [f32; 2]) -> Option<String> {
    let (a, b) = (w * vh, h * vw);
    if vw <= 0.0 || vh <= 0.0 || (a - b).abs() <= 1e-5 * a.max(b) {
        return None;
    }
    Some(format!(
        "its width and height ({w} x {h}) are not in the ratio of its viewBox ({vw} x {vh}); preserveAspectRatio is not built, so give a width and height in the viewBox's ratio"
    ))
}

/// How the icon is named to the a11y tree: decorative or a named image.
fn named(attributes: &BTreeMap<String, String>) -> Option<String> {
    let get = |name| attributes.get(name).map(String::as_str);
    if get("aria-hidden") == Some("true") {
        return None;
    }
    if get("role") != Some("img") {
        return Some("needs aria-hidden=\"true\" (a decorative icon, left out of the a11y tree) or role=\"img\" with an aria-label (a named image); give one of the two".to_owned());
    }
    let label = get("aria-label").map(str::trim).unwrap_or_default();
    label.is_empty().then(|| "has role=\"img\" and no aria-label; name it with a non-empty aria-label, or mark it decorative with aria-hidden=\"true\"".to_owned())
}

fn namespace(attributes: &BTreeMap<String, String>) -> Option<String> {
    let xmlns = attributes
        .get("xmlns")
        .filter(|value| *value != NAMESPACE)?;
    Some(format!(
        "its xmlns {xmlns:?} is not the SVG namespace; remove it or write xmlns=\"{NAMESPACE}\""
    ))
}

/// Children other than `path`s and whitespace, and no `path` at all.
fn children(arena: &Arena, node: NodeId) -> Vec<String> {
    let kids = arena.children(node);
    let mut found: Vec<String> = kids
        .iter()
        .filter_map(|&child| child_reason(arena, child))
        .collect();
    if !kids
        .iter()
        .any(|&child| crate::svg::is(arena, child, "path"))
    {
        found.push(
            "holds no <path>, so it draws nothing; add a <path> or remove the svg".to_owned(),
        );
    }
    found
}

/// Why `child` of an svg is refused: anything but a `path` or whitespace.
fn child_reason(arena: &Arena, child: NodeId) -> Option<String> {
    match &arena.get(child, "svg_checks").kind {
        Kind::Text(text) if text.trim().is_empty() => None,
        Kind::Text(text) => Some(format!(
            "holds the text {text:?}, and an icon holds <path> elements and whitespace only; remove the text, or name the icon with aria-label"
        )),
        Kind::Element(element) if element.tag == "path" => None,
        Kind::Element(_) => Some(format!(
            "holds {}, and an icon holds <path> elements and whitespace only; move it out of the svg",
            describe(arena, child)
        )),
    }
}
