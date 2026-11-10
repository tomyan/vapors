//! What a `gpu-surface` element paints and refuses (docs/gpu-surface.md):
//! a surface item over its content box, named by its `name`, which the
//! painter resolves to a mounted renderer; nothing while it is hidden or
//! has no area. Refused here: no `name`, or an empty one; the style it
//! refuses is in `style::surface`.

use vapors_dom::NodeId;
use vapors_scene::{Item, Rect, SurfaceId};

use crate::arena::Arena;
use crate::check::describe;
use crate::error::ElementError;
use crate::layout::{BoxRect, Content, LayoutBox};
use crate::paint::PaintInput;
use crate::painted::Painted;

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
    let rect = crate::paint::snapped(
        crate::clip::content_box(layout_box, border_box),
        input.scale,
    );
    if input.used[&node].visible && rect.width() > 0.0 && rect.height() > 0.0 {
        push(out, name, rect);
    }
}

fn push(out: &mut Painted, name: &str, rect: Rect) {
    let id = SurfaceId(u32::try_from(out.surfaces.len()).expect("fewer than 2^32 surfaces"));
    out.surfaces.push(name.to_owned());
    out.scene.push(Item::Surface { surface: id, rect });
}

/// The refusals of every `gpu-surface` among `nodes` without a name;
/// the style a surface refuses is refused with its other style
/// (`style::surface`).
pub(crate) fn refusals(arena: &Arena, nodes: &[NodeId]) -> Vec<ElementError> {
    nodes
        .iter()
        .filter(|&&node| is_surface(arena, node) && name(arena, node).is_none())
        .map(|&node| {
            ElementError::UnsupportedAttribute(format!("{}: {UNNAMED}", describe(arena, node)))
        })
        .collect()
}

const UNNAMED: &str = "a gpu-surface draws the renderer its name attribute names, and this one has no name or an empty one; give it name=\"<renderer>\" and mount a renderer of that name with ScenePainter::mount(name, renderer) (vapors-app)";

/// Whether `node` is a `gpu-surface`.
pub(crate) fn is_surface(arena: &Arena, node: NodeId) -> bool {
    arena
        .get(node, "surfaces")
        .element()
        .is_some_and(|element| element.tag == TAG)
}

/// `node`'s `name`, when it is a `gpu-surface` with a name that is not
/// empty.
pub(crate) fn name(arena: &Arena, node: NodeId) -> Option<&str> {
    let element = arena.get(node, "surfaces").element()?;
    if element.tag != TAG {
        return None;
    }
    let name = element.attributes.get("name").map(String::as_str);
    name.filter(|name| !name.is_empty())
}
