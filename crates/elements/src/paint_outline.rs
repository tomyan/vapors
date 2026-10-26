//! Outlines are painted after everything else (CSS 2's painting order
//! puts them last), each as a ring outside its element's border box at
//! the outline's offset, under the same clip as the element.

use vapors_scene::{Color, Item, Quad, Rect};

use crate::layout::{BoxRect, Content, LayoutBox};
use crate::paint::{PaintInput, snapped};
use crate::painted::Painted;

/// Each visible element's outline, in box order.
pub(crate) fn paint_outlines(
    boxes: &[LayoutBox],
    rects: &[BoxRect],
    clips: &[Option<BoxRect>],
    input: &PaintInput<'_>,
    out: &mut Painted,
) {
    for (index, layout_box) in boxes.iter().enumerate() {
        let Content::Element(node) = layout_box.content else {
            continue;
        };
        let used = &input.used[&node];
        let Some(outline) = used.decoration.outline.filter(|_| used.visible) else {
            continue;
        };
        out.scene
            .set_clip(clips[index].map(|clip| snapped(clip, input.scale)));
        let reach = (outline.offset + outline.width) * input.scale;
        let border_box = snapped(rects[index], input.scale);
        out.scene.push(Item::Quad(Quad {
            rect: Rect::new(
                border_box.left - reach,
                border_box.top - reach,
                border_box.right + reach,
                border_box.bottom + reach,
            ),
            background: Color::default(),
            border: [outline.width * input.scale; 4],
            border_colors: [outline.color; 4],
            // the ring's outer corners follow the border's, grown by its reach
            radii: used.decoration.radii.map(|r| {
                if r > 0.0 {
                    r * input.scale + reach
                } else {
                    0.0
                }
            }),
        }));
    }
}
