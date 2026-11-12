//! The CSS an inline `<svg>` icon refuses (docs/icons.md, The subset):
//! on an `svg`, CSS sizes (it is sized by its attributes only), being
//! inline (inline-block content, not built), percentage padding
//! (`layout::replaced` reads lengths only) and a border-radius (no
//! rounded clip); on a `path`, a colour, `display` or visibility of its
//! own, since all of an icon's paths are drawn in its svg's.

use crate::node_map::NodeMap;

use vapors_dom::NodeId;

use crate::arena::Arena;
use crate::style::Used;

/// Why `node`'s used style is refused, when it is an `svg` or `path`.
pub(crate) fn reasons(arena: &Arena, node: NodeId, used: &NodeMap<Used>) -> Vec<String> {
    let Some(own) = used.get(&node) else {
        return Vec::new();
    };
    if crate::svg::is(arena, node, "svg") {
        return svg(own);
    }
    let parent = arena.get(node, "svg_style_checks").parent;
    match parent.and_then(|parent| used.get(&parent)) {
        Some(svg) if crate::svg::is(arena, node, "path") => path(own, svg),
        _ => Vec::new(),
    }
}

fn svg(used: &Used) -> Vec<String> {
    let mut found = sizes(&used.layout);
    if used.inline {
        found.push("is inline, and an icon inside a line of text is inline-block content, which vapors does not lay out yet; give it display: block, or put it in a flex or grid container".to_owned());
    }
    if relative_padding(&used.layout) {
        found.push("has a percentage or calc() padding, which an svg's size does not resolve; give it in px".to_owned());
    }
    if used.decoration.radii.iter().any(|r| *r > 0.0) {
        found.push("has a border-radius, and an icon is not clipped to rounded corners (rounded clips are refused until a slice needs them); drop the radius".to_owned());
    }
    found
}

/// CSS sizes, each refused: an svg is sized by its attributes only.
fn sizes(style: &taffy::Style) -> Vec<String> {
    [
        ("width", style.size.width.into_raw()),
        ("height", style.size.height.into_raw()),
        ("min-width", style.min_size.width.into_raw()),
        ("min-height", style.min_size.height.into_raw()),
        ("max-width", style.max_size.width.into_raw()),
        ("max-height", style.max_size.height.into_raw()),
    ]
    .into_iter()
    .filter(|(_, raw)| !raw.is_auto())
    .map(|(name, _)| format!("has a CSS {name}, and an svg icon is sized by its width and height attributes only (docs/icons.md); remove the CSS {name} and set the attributes"))
    .collect()
}

fn relative_padding(style: &taffy::Style) -> bool {
    let padding = &style.padding;
    [padding.left, padding.right, padding.top, padding.bottom]
        .into_iter()
        .any(|side| crate::img_checks::relative(side.into_raw()))
}

fn path(own: &Used, svg: &Used) -> Vec<String> {
    let mut found = Vec::new();
    if own.text.color != svg.text.color {
        found.push("has a color of its own, and every path of an icon is stroked in its svg's color; set color on the svg instead".to_owned());
    }
    if own.layout.display == taffy::Display::None {
        found.push(
            "has display: none, and an icon draws every one of its paths; remove the path instead"
                .to_owned(),
        );
    }
    if own.visible != svg.visible {
        found.push("has a visibility of its own, and an icon's paths are shown with their svg; set visibility on the svg instead".to_owned());
    }
    found
}
