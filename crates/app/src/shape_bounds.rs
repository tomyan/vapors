//! A shape item's bounds against what its entry draws: culling reads
//! only the bounds, so an item whose drawn pixels reach past them would
//! be culled while visible, with no error. The drawn box is the entry's
//! box grown by half its stroke, under the item's transform, grown by
//! the half pixel of ramp outside the outline; it must lie within the
//! item's culling bounds (`Item::bounds`, its own grown by the fringe).

use vapors_scene::{Item, Rect, Shape};
use vapors_shapes::{Geometry, transformed_box};

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
    let local = geometry.local_box(stroke.unwrap_or(0.0) / 2.0);
    let [lo, hi] = transformed_box(local, item.transform);
    let drawn = Rect::new(lo[0], lo[1], hi[0], hi[1]).grown(RAMP);
    let bounds = Item::Shape(*item).bounds().grown(SLACK);
    let within = drawn.left >= bounds.left
        && drawn.top >= bounds.top
        && drawn.right <= bounds.right
        && drawn.bottom <= bounds.bottom;
    (!within).then_some(drawn)
}
