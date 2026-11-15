//! An inline `<svg>` icon in paint (docs/icons.md, Paint): the
//! `vapors_text::Icon` its attributes and paths describe, built when its
//! box is, and one `Item::Icon` over its content box snapped to device
//! pixels, in its used `color`.

use std::rc::Rc;

use vapors_dom::NodeId;
use vapors_scene::{Color, IconId, Item};
use vapors_shapes::IconPath;
use vapors_text::{Icon, IconStroke};

use crate::arena::Arena;
use crate::layout::{BoxRect, Content, LayoutBox};
use crate::paint::PaintInput;
use crate::painting::Painting;
use crate::style::UsedMap;
use crate::svg::view_box;
use crate::svg_route::Route;

/// The icon `node` draws when it is an `svg` on the icon route
/// (`svg_route`); `None` for any other element.
///
/// # Panics
/// When the svg is outside the subset: `svg_checks` refuses it before
/// layout, so reaching here with one is a bug in those checks.
pub(crate) fn icon(arena: &Arena, used: &UsedMap, node: NodeId) -> Option<Rc<Icon>> {
    let element = arena.get(node, "svg_paint").element()?;
    if element.tag != "svg" || crate::svg_route::route(arena, used, node)? != Route::Icon {
        return None;
    }
    let passed = "svg_checks passed this svg, so its view box and paths are in the subset";
    let view_box = view_box(&element.attributes["viewBox"]).expect(passed);
    let strokes = arena
        .children(node)
        .into_iter()
        .filter(|&child| crate::svg::is(arena, child, "path"))
        .map(|child| stroke(arena, used, child).expect(passed))
        .collect();
    Some(Rc::new(Icon::new(view_box, strokes).expect(passed)))
}

/// The stroke the `path` `child` draws, its width from its used
/// style; `None` when it is outside the subset.
fn stroke(arena: &Arena, used: &UsedMap, child: NodeId) -> Option<IconStroke> {
    let attributes = &arena.get(child, "svg_paint").element()?.attributes;
    let path = IconPath::parse(attributes.get("d")?).ok()?;
    let width = used.get(&child)?.svg.stroke_width;
    Some(IconStroke { path, width })
}

/// Paints what the svg `layout_box` is shows over its content box
/// (border box `border_box`, CSS px): its icon or its shapes, when it is
/// visible. An svg on the icon route releases the shapes it had; a
/// hidden one's are released by its checks (`svg_shape_checks`).
///
/// # Panics
/// When an svg's box has no used style: styling gives every element
/// one before boxes are built.
pub(crate) fn paint(
    layout_box: &LayoutBox,
    border_box: BoxRect,
    input: &PaintInput<'_>,
    out: &mut Painting,
) {
    let (true, &Content::Element(node)) = (
        layout_box.icon.is_some() || layout_box.svg_shapes.is_some(),
        &layout_box.content,
    ) else {
        return;
    };
    let used = &input.used[&node];
    let content = crate::clip::content_box(layout_box, border_box);
    match (&layout_box.icon, &layout_box.svg_shapes) {
        (Some(icon), _) if used.visible => {
            input.shapes.borrow_mut().release(node);
            push_icon(icon, content, used.text.color, input.scale, out);
        }
        (_, Some(shapes)) if used.visible => {
            crate::svg_shapes_paint::paint((node, shapes), content, input, out);
        }
        // a hidden svg's shapes are released by its checks
        _ => {}
    }
}

/// One `Item::Icon` of `icon` over `content` snapped to device pixels.
fn push_icon(icon: &Rc<Icon>, content: BoxRect, colour: Color, scale: f32, out: &mut Painting) {
    let id = IconId(out.slabs.icons.insert(Rc::clone(icon)));
    let rect = crate::paint::snapped(content, scale);
    out.scene.push(Item::Icon {
        icon: id,
        rect,
        colour,
    });
}
