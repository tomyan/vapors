//! Painting an svg on the shapes route (`svg_shapes`): each path's
//! shapes under the transform from its view box to its content box in
//! device pixels, the view box's width over the content's giving the
//! scale (the checks hold their ratio to the attributes').

use crate::layout::BoxRect;
use crate::paint::PaintInput;
use crate::painting::Painting;
use crate::svg_shapes::{SvgShapes, acquired, item};

/// Pushes the shape items of `svg`'s `shapes` over `content` (CSS px),
/// acquired for it.
///
/// # Panics
/// When a path is unstyled, or the cache refuses a path the checks
/// passed: both are faults in this crate.
pub(crate) fn paint(
    (svg, shapes): (vapors_dom::NodeId, &SvgShapes),
    content: BoxRect,
    input: &PaintInput<'_>,
    out: &mut Painting,
) {
    let transform = transform(shapes.view_box, content, input);
    let parts: Vec<_> = shapes
        .parts
        .iter()
        .map(|path| (path.site.as_str(), &path.data, &*input.used[&path.node]))
        .collect();
    // Flattened for the viewport's scale only: nothing scales a shape
    // after paint yet (CSS `transform` is deferred, `style/honoured.rs`).
    // Lifting that deferral, or anchoring an svg in 3D (one-scene S7),
    // must pass the larger scale here, or curves flatten coarser than
    // 0.1 px by the extra factor.
    let checked = input.shapes.borrow().svgs.get(&svg).copied().flatten();
    assert_eq!(
        checked,
        Some(transform[0]),
        "the svg's shapes were checked at another scale than paint draws them at; svg_shape_refusals must run after layout on every svg on the shapes route"
    );
    let made = acquired(input.shapes, svg, &parts, transform[0])
        .unwrap_or_else(|e| panic!("the svg checks passed these parts: {e}"));
    for shape in &made {
        out.scene.push(item(input.shapes, shape, transform));
    }
}

/// `matrix(a, b, c, d, e, f)` from view box units to device pixels: the
/// view box fitted inside the content box and centred in it (SVG's
/// default `xMidYMid meet`), from the content box's origin snapped to a
/// device pixel, as Chromium places a replaced element's content (and
/// the icon route snaps its rect).
fn transform(view_box: [f32; 4], content: BoxRect, input: &PaintInput<'_>) -> [f32; 6] {
    let [vx, vy, vw, vh] = view_box;
    let scale = input.scale;
    let per_unit = crate::svg_shapes::per_unit(view_box, content, scale);
    let origin = input.snapped(content);
    let spare = [
        content.width * scale - vw * per_unit,
        content.height * scale - vh * per_unit,
    ];
    [
        per_unit,
        0.0,
        0.0,
        per_unit,
        origin.left + spare[0] / 2.0 - vx * per_unit,
        origin.top + spare[1] / 2.0 - vy * per_unit,
    ]
}
