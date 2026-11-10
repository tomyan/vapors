//! An inline `<svg>` icon in paint (docs/icons.md, Paint): the
//! `vapors_text::Icon` its attributes and paths describe, built when its
//! box is, and one `Item::Icon` over its content box snapped to device
//! pixels, in its used `color`.

use std::rc::Rc;

use vapors_dom::NodeId;
use vapors_scene::{Color, IconId, Item};
use vapors_text::{Icon, IconPath, IconStroke};

use crate::arena::Arena;
use crate::layout::{BoxRect, Content, LayoutBox};
use crate::paint::PaintInput;
use crate::painted::Painted;
use crate::svg::{positive, view_box};

/// The icon `node` draws when it is an `svg`; `None` for any other
/// element.
///
/// # Panics
/// When the svg is outside the subset: `svg_checks` refuses it before
/// layout, so reaching here with one is a bug in those checks.
pub(crate) fn icon(arena: &Arena, node: NodeId) -> Option<Rc<Icon>> {
    let element = arena.get(node, "svg_paint").element()?;
    if element.tag != "svg" {
        return None;
    }
    let passed = "svg_checks passed this svg, so its view box and paths are in the subset";
    let view_box = view_box(&element.attributes["viewBox"]).expect(passed);
    let strokes = arena
        .children(node)
        .into_iter()
        .filter(|&child| crate::svg::is(arena, child, "path"))
        .map(|child| stroke(arena, child).expect(passed))
        .collect();
    Some(Rc::new(Icon::new(view_box, strokes).expect(passed)))
}

/// The stroke the `path` `child` draws; `None` when it is outside the
/// subset.
fn stroke(arena: &Arena, child: NodeId) -> Option<IconStroke> {
    let attributes = &arena.get(child, "svg_paint").element()?.attributes;
    let path = IconPath::parse(attributes.get("d")?).ok()?;
    let width = attributes
        .get("stroke-width")
        .map_or(Some(1.0), |w| positive(w))?;
    Some(IconStroke { path, width })
}

/// Paints the icon `layout_box` shows over its content box (border box
/// `border_box`, CSS px), when it has one and is visible.
pub(crate) fn paint(
    layout_box: &LayoutBox,
    border_box: BoxRect,
    input: &PaintInput<'_>,
    out: &mut Painted,
) {
    let (Some(icon), &Content::Element(node)) = (&layout_box.icon, &layout_box.content) else {
        return;
    };
    let used = &input.used[&node];
    if !used.visible {
        return;
    }
    let content = crate::clip::content_box(layout_box, border_box);
    push_icon(icon, content, used.text.color, input.scale, out);
}

/// One `Item::Icon` of `icon` over `content` snapped to device pixels.
fn push_icon(icon: &Rc<Icon>, content: BoxRect, colour: Color, scale: f32, out: &mut Painted) {
    let id = IconId(u32::try_from(out.icons.len()).expect("fewer than 2^32 icons"));
    out.icons.push(Rc::clone(icon));
    let rect = crate::paint::snapped(content, scale);
    out.scene.push(Item::Icon {
        icon: id,
        rect,
        colour,
    });
}
