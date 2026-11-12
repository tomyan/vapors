use vapors_scene::{Color, Item, Quad, Rect, Run, Shadow, SurfaceId};

use super::same_bits;

fn quads(top: f32) -> Run {
    let rect = Rect::new(0.0, top, 8.0, 8.0);
    Run::Quads(vec![Quad::fill(rect, Color::from_rgba8(255, 0, 0, 255))])
}

fn shadows(blur: f32) -> Run {
    let rect = Rect::new(0.0, 0.0, 8.0, 8.0);
    Run::Shadows(vec![Shadow {
        rect,
        radii: [0.0; 4],
        blur,
        color: Color::from_rgba8(0, 0, 0, 255),
        caster: rect,
        caster_radii: [0.0; 4],
    }])
}

fn surfaces(left: f32) -> Run {
    let rect = Rect::new(left, 0.0, 8.0, 8.0);
    Run::Surfaces(vec![Item::Surface {
        surface: SurfaceId(1),
        rect,
    }])
}

#[test]
fn equal_runs_are_the_same_to_the_bit() {
    // When / Then
    assert!(same_bits(&quads(0.0), &quads(0.0)));
    assert!(same_bits(&shadows(2.0), &shadows(2.0)));
    assert!(same_bits(&surfaces(0.0), &surfaces(0.0)));
}

#[test]
fn runs_whose_zeros_differ_in_sign_are_equal_but_not_the_same_to_the_bit() {
    // Given: each pair equal under ==
    let pairs = [
        (quads(-0.0), quads(0.0)),
        (shadows(0.0), shadows(-0.0)),
        (surfaces(-0.0), surfaces(0.0)),
    ];

    // When / Then
    for (was, now) in pairs {
        assert_eq!(was, now);
        assert!(!same_bits(&was, &now), "{was:?} and {now:?}");
    }
}

#[test]
fn runs_that_differ_are_not_the_same_to_the_bit() {
    // When / Then
    assert!(!same_bits(&quads(0.0), &quads(1.0)));
    assert!(!same_bits(&quads(0.0), &shadows(0.0)));
}
