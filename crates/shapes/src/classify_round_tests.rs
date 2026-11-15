//! Rounded rectangles: SVG's `<rect rx ry>` outline, from any of its
//! points and either way round, is a rect with radii that the closed-
//! form check draws, a pill an f32 ulp short of its radii among them.
//! The near misses are in `classify_round_near_tests.rs`.

use crate::canonical::{FillRule, Paint};
use crate::classify::{Classified, classify};
use crate::geometry::{Geometry, check_geometry};
use crate::path::IconPath;

const FILL: Paint = Paint::Fill(FillRule::NonZero);
const STROKE: Paint = Paint::Stroke { width: 1.5 };

fn classified(data: &str, paint: Paint) -> Classified {
    let path = IconPath::parse(data).unwrap_or_else(|e| panic!("{data:?}: {e}"));
    classify(&path, paint)
}

const fn rounded(radius: [f32; 2]) -> Classified {
    Classified::Rect {
        origin: [1.0, 1.0],
        size: [10.0, 6.0],
        radius,
    }
}

#[test]
fn svgs_rounded_rect_outline_is_a_rect_with_its_radii() {
    // Given: SVG 2's outline for x=1 y=1 width=10 height=6 rx=ry=2,
    // filled and stroked; open, filled; from an arc's start; the other
    // way round; elliptical corners, written relative
    let round = rounded([2.0, 2.0]);
    let spec = "M3 1H9A2 2 0 0 1 11 3V5A2 2 0 0 1 9 7H3A2 2 0 0 1 1 5V3A2 2 0 0 1 3 1Z";
    let cases = [
        (spec, STROKE, round),
        (spec, FILL, round),
        (spec.trim_end_matches('Z'), FILL, round),
        (
            "M1 3A2 2 0 0 1 3 1H9A2 2 0 0 1 11 3V5A2 2 0 0 1 9 7H3A2 2 0 0 1 1 5V3Z",
            STROKE,
            round,
        ),
        (
            "M3 1A2 2 0 0 0 1 3V5A2 2 0 0 0 3 7H9A2 2 0 0 0 11 5V3A2 2 0 0 0 9 1H3Z",
            STROKE,
            round,
        ),
        (
            "M4 1h4a3 1.5 0 0 1 3 1.5v3a3 1.5 0 0 1-3 1.5h-4a3 1.5 0 0 1-3-1.5v-3a3 1.5 0 0 1 3-1.5z",
            STROKE,
            rounded([3.0, 1.5]),
        ),
    ];

    // When, Then
    for (data, paint, expected) in cases {
        let found = classified(data, paint);
        assert_eq!(found, expected, "{data}");
        assert_eq!(check_geometry(&geometry(found)), Ok(()), "{data}");
    }
}

/// The geometry a classified rect draws.
fn geometry(found: Classified) -> Geometry<'static> {
    let Classified::Rect {
        origin,
        size,
        radius,
    } = found
    else {
        panic!("{found:?} is not a rect");
    };
    Geometry::Rect {
        origin,
        size,
        radius,
    }
}

#[test]
fn a_pill_an_ulp_short_of_its_radii_is_clamped_to_a_rect_the_check_draws() {
    // Given: SVG's outline of <rect x=1 y=0.05 width=10 height=1 rx=0.5>,
    // whose height is 1.05 - 0.05 = 0.99999994 in f32, under 2 ry (#387
    // review, H1)
    let pill = "M1.5 0.05H10.5A0.5 0.5 0 0 1 11 0.55V0.55A0.5 0.5 0 0 1 10.5 1.05H1.5A0.5 0.5 0 0 1 1 0.55V0.55A0.5 0.5 0 0 1 1.5 0.05Z";

    // When
    let found = classified(pill, FILL);

    // Then: ry is clamped to half the height, and the check draws it
    let Classified::Rect { size, radius, .. } = found else {
        panic!("{found:?}");
    };
    assert_eq!(radius, [0.5, size[1] / 2.0]);
    assert!(size[1] < 1.0, "{size:?}: the case is an ulp short");
    assert_eq!(check_geometry(&geometry(found)), Ok(()));
}
