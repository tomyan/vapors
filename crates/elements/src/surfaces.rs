//! What a `gpu-surface` element paints and refuses (docs/gpu-surface.md):
//! a surface item over its content box, named by its `name`, which the
//! painter resolves to a mounted renderer; nothing while it is hidden or
//! has no area. Refused: no `name`; padding, so that a pointer event's
//! offset on the surface is surface-local; a border-radius (no rounded
//! clip); and `display: inline` (inline-block content is not laid out).

use std::collections::HashMap;

use vapors_dom::NodeId;
use vapors_scene::{Item, SurfaceId};

use crate::arena::Arena;
use crate::check::describe;
use crate::error::ElementError;
use crate::layout::{BoxRect, Content, LayoutBox};
use crate::paint::PaintInput;
use crate::painted::Painted;
use crate::style::Used;

const TAG: &str = "gpu-surface";

/// Paints the surface item of `layout_box` (border box `border_box`),
/// when it is a visible `gpu-surface` with a name and an area.
pub(crate) fn paint(
    layout_box: &LayoutBox,
    border_box: BoxRect,
    input: &PaintInput<'_>,
    out: &mut Painted,
) {
    let (Some(name), &Content::Element(node)) = (&layout_box.surface, &layout_box.content) else {
        return;
    };
    let content = crate::clip::content_box(layout_box, border_box);
    let rect = crate::paint::snapped(content, input.scale);
    if !input.used[&node].visible || rect.width() <= 0.0 || rect.height() <= 0.0 {
        return;
    }
    let id = SurfaceId(u32::try_from(out.surfaces.len()).expect("fewer than 2^32 surfaces"));
    out.surfaces.push(name.to_string());
    out.scene.push(Item::Surface { surface: id, rect });
}

/// The refusals of every `gpu-surface` among `nodes`.
pub(crate) fn refusals(
    arena: &Arena,
    nodes: &[NodeId],
    used: &HashMap<NodeId, Used>,
) -> Vec<ElementError> {
    let surfaces = nodes
        .iter()
        .copied()
        .filter(|&node| is_surface(arena, node));
    surfaces
        .flat_map(|node| {
            let element = describe(arena, node);
            reasons(arena, node, used.get(&node))
                .into_iter()
                .map(move |reason| {
                    ElementError::UnsupportedAttribute(format!("{element}: {reason}"))
                })
        })
        .collect()
}

fn reasons(arena: &Arena, node: NodeId, used: Option<&Used>) -> Vec<String> {
    let mut found = Vec::new();
    if name(arena, node).is_none() {
        found.push(format!("a {TAG} draws the renderer its name attribute names; give it name=\"<renderer>\" and mount a renderer of that name on the painter"));
    }
    found.extend(used.into_iter().flat_map(styled));
    found
}

fn styled(used: &Used) -> Vec<String> {
    let mut found = Vec::new();
    if used.inline {
        found.push(format!("a {TAG} is inline, and inline-block content is not laid out yet; give it display: block, or put it in a flex or grid container"));
    }
    if used.decoration.radii.iter().any(|r| *r > 0.0) {
        found.push(format!("a {TAG} has a border-radius, and a surface is not clipped to rounded corners (docs/gpu-surface.md); drop the radius"));
    }
    if padded(&used.layout) {
        found.push(format!("a {TAG} has padding, which would make pointer offsets on it differ from surface-local coordinates (docs/gpu-surface.md); put the padding on a wrapping element"));
    }
    found
}

fn padded(style: &taffy::Style) -> bool {
    let sides = [
        style.padding.left,
        style.padding.right,
        style.padding.top,
        style.padding.bottom,
    ];
    sides
        .iter()
        .any(|side| *side != taffy::LengthPercentage::length(0.0))
}

fn is_surface(arena: &Arena, node: NodeId) -> bool {
    arena
        .get(node, "surfaces")
        .element()
        .is_some_and(|element| element.tag == TAG)
}

/// `node`'s `name`, when it is a `gpu-surface` with one.
pub(crate) fn name(arena: &Arena, node: NodeId) -> Option<&str> {
    let element = arena.get(node, "surfaces").element()?;
    if element.tag != TAG {
        return None;
    }
    element.attributes.get("name").map(String::as_str)
}
