//! The lyon path: a segment after a close starts at the subpath's
//! start, as SVG's pen does.

use lyon_tessellation::path::PathEvent;

use super::*;
use crate::path::IconPath;

/// Every point lyon holds for the events after the first close.
fn points_after_close(data: &str) -> Vec<LyonPoint> {
    let path = IconPath::parse(data).unwrap_or_else(|e| panic!("{data:?}: {e}"));
    lyon_path(path.segments())
        .iter()
        .skip_while(|event| !matches!(event, PathEvent::End { close: true, .. }))
        .skip(1)
        .flat_map(|event| match event {
            PathEvent::Begin { at } => vec![at],
            PathEvent::Line { from, to } => vec![from, to],
            PathEvent::Quadratic { from, ctrl, to } => vec![from, ctrl, to],
            PathEvent::Cubic {
                from,
                ctrl1,
                ctrl2,
                to,
            } => vec![from, ctrl1, ctrl2, to],
            PathEvent::End { last, first, .. } => vec![last, first],
        })
        .collect()
}

#[test]
fn an_arc_after_a_close_starts_at_the_subpaths_start() {
    // Given: a triangle closed back to (0, 0), then with no M a half
    // circle of radius 1 to (2, 0); SVG starts it at (0, 0), not at the
    // triangle's last point (4, 4), from which the radii would scale up
    // to reach (2, 0)
    let data = "M0 0h4v4zA1 1 0 0 1 2 0";

    // When
    let points = points_after_close(data);

    // Then: every point of the half circle, control points included,
    // is inside its bounding square
    assert!(!points.is_empty(), "{data}");
    for p in points {
        assert!(
            (-1e-5..=2.00001).contains(&p.x) && (-1.00001..=1.00001).contains(&p.y),
            "{p:?} is off the half circle from (0, 0) to (2, 0)"
        );
    }
}
