//! What an `img` refuses (F2c S10), checked on every `img`, with a `src`
//! or without: a `src` naming no registered image; an inline `img` (an
//! image in a line of text is inline-block content, not built); a
//! rounded one (no rounded clip yet); percentage min and max sizes and
//! padding, which its measure does not resolve; and the `width`,
//! `height` and `srcset` attributes, which are not read.

use std::collections::HashMap;

use vapors_dom::NodeId;

use crate::arena::Arena;
use crate::check::describe;
use crate::error::ElementError;
use crate::images::{Images, src};
use crate::style::Used;

/// The refusals of every `img` among `nodes`.
pub(crate) fn refusals(
    images: &Images,
    arena: &Arena,
    nodes: &[NodeId],
    used: &HashMap<NodeId, Used>,
) -> Vec<ElementError> {
    let imgs = nodes.iter().copied().filter(|&node| is_img(arena, node));
    imgs.flat_map(|node| {
        let reasons = reasons(images, arena, node, used.get(&node));
        let element = describe(arena, node);
        reasons
            .into_iter()
            .map(move |reason| ElementError::UnsupportedAttribute(format!("{element}: {reason}")))
    })
    .collect()
}

fn is_img(arena: &Arena, node: NodeId) -> bool {
    arena
        .get(node, "img_checks")
        .element()
        .is_some_and(|element| element.tag == "img")
}

fn reasons(images: &Images, arena: &Arena, node: NodeId, used: Option<&Used>) -> Vec<String> {
    let mut found: Vec<String> = attributes(arena, node);
    if let Some(name) = src(arena, node).filter(|name| !images.has(name)) {
        found.push(format!("its src {name:?} names no image registered with the document; register it first with Document::register_image({name:?}, image)"));
    }
    found.extend(used.into_iter().flat_map(styled));
    found
}

fn attributes(arena: &Arena, node: NodeId) -> Vec<String> {
    let Some(element) = arena.get(node, "img_checks").element() else {
        return Vec::new();
    };
    ["width", "height", "srcset"]
        .into_iter()
        .filter(|name| element.attributes.contains_key(*name))
        .map(|name| format!("its {name} attribute is not read; size an img with CSS, and register one image per name"))
        .collect()
}

fn styled(used: &Used) -> Vec<String> {
    let mut found = Vec::new();
    if used.inline {
        found.push("is inline, and an image inside a line of text is inline-block content, which vapors does not lay out yet (a slice after F2c); give it display: block, or put it in a flex or grid container".to_owned());
    }
    if used.decoration.radii.iter().any(|r| *r > 0.0) {
        found.push("has a border-radius, and an image is not clipped to rounded corners yet (rounded clips are refused until a slice needs them, docs/F2c.md); drop the radius".to_owned());
    }
    found.extend(percentages(&used.layout));
    found
}

/// Percentage min and max sizes and padding, which `layout::replaced`
/// does not resolve.
fn percentages(style: &taffy::Style) -> Vec<String> {
    let limits = [
        ("min-width", style.min_size.width.into_raw()),
        ("min-height", style.min_size.height.into_raw()),
        ("max-width", style.max_size.width.into_raw()),
        ("max-height", style.max_size.height.into_raw()),
    ];
    let padding = [
        style.padding.left,
        style.padding.right,
        style.padding.top,
        style.padding.bottom,
    ]
    .map(|side| ("padding", side.into_raw()));
    let named: std::collections::BTreeSet<&str> = limits
        .into_iter()
        .chain(padding)
        .filter(|(_, raw)| relative(*raw))
        .map(|(name, _)| name)
        .collect();
    named
        .into_iter()
        .map(|name| format!("has a percentage or calc() {name}, which an img's size does not resolve yet; give it in px"))
        .collect()
}

fn relative(raw: taffy::CompactLength) -> bool {
    raw.tag() == taffy::CompactLength::PERCENT_TAG || raw.is_plain_calc()
}
