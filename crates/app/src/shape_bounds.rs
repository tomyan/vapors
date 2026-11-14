//! A shape item's bounds against what its entry draws: culling reads
//! only the bounds, so an item whose drawn pixels reach past them would
//! be culled while visible, with no error. The drawn box is the entry's
//! box grown by half its stroke, under the item's transform, grown by
//! the half pixel of ramp outside the outline; it must lie within the
//! item's culling bounds (`Item::bounds`, its own grown by the fringe).

use vapors_scene::{Item, Rect, Shape};
use vapors_shapes::Geometry;

/// How far outside the outline the closed-form ramp covers, device
/// pixels (`closed_forms.wgsl`: coverage 0.5 minus the distance).
const RAMP: f32 = 0.5;

/// Slack for f32 rounding of the transformed corners.
const SLACK: f32 = 1e-3;

/// The device-pixel box `geometry` stroked `stroke` wide draws under
/// `item`'s transform, when it lies outside `item`'s culling bounds.
pub(crate) fn outside_bounds(
    geometry: &Geometry,
    stroke: Option<f32>,
    item: &Shape,
) -> Option<Rect> {
    let [lo, hi] = local_box(geometry, stroke.unwrap_or(0.0) / 2.0)?;
    let drawn = transformed([lo, hi], item.transform).grown(RAMP);
    let bounds = Item::Shape(*item).bounds().grown(SLACK);
    let within = drawn.left >= bounds.left
        && drawn.top >= bounds.top
        && drawn.right <= bounds.right
        && drawn.bottom <= bounds.bottom;
    (!within).then_some(drawn)
}

/// The closed form's box in its own units, grown by `by`.
fn local_box(geometry: &Geometry, by: f32) -> Option<[[f32; 2]; 2]> {
    let [lo, hi] = match *geometry {
        Geometry::Rect { origin, size, .. } => [origin, [origin[0] + size[0], origin[1] + size[1]]],
        Geometry::Ellipse { centre, radii } => [
            [centre[0] - radii[0], centre[1] - radii[1]],
            [centre[0] + radii[0], centre[1] + radii[1]],
        ],
        Geometry::Mesh { .. } => return None,
    };
    Some([[lo[0] - by, lo[1] - by], [hi[0] + by, hi[1] + by]])
}

/// The device-pixel box of the four corners of `[lo, hi]` under
/// `transform`.
fn transformed([lo, hi]: [[f32; 2]; 2], m: [f32; 6]) -> Rect {
    let corners = [
        [lo[0], lo[1]],
        [hi[0], lo[1]],
        [lo[0], hi[1]],
        [hi[0], hi[1]],
    ];
    let points = corners.map(|[x, y]| [m[0] * x + m[2] * y + m[4], m[1] * x + m[3] * y + m[5]]);
    let fold = |pick: fn(f32, f32) -> f32, axis: usize| {
        points.iter().map(|p| p[axis]).fold(points[0][axis], pick)
    };
    Rect::new(
        fold(f32::min, 0),
        fold(f32::min, 1),
        fold(f32::max, 0),
        fold(f32::max, 1),
    )
}
