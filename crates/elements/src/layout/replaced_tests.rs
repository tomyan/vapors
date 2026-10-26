use super::*;

const NONE: Limits = Limits {
    min: [0.0, 0.0],
    max: [f32::INFINITY, f32::INFINITY],
};

fn sized(width: Option<f32>, height: Option<f32>, limits: Limits) -> [f32; 2] {
    let found = size(taffy::Size { width, height }, [40.0, 20.0], limits);
    [found.width, found.height]
}

#[test]
fn with_nothing_known_or_limited_the_natural_size() {
    assert_eq!(sized(None, None, NONE), [40.0, 20.0]);
}

#[test]
fn one_known_length_sets_the_other_through_the_ratio_then_its_limits() {
    assert_eq!(sized(Some(80.0), None, NONE), [80.0, 40.0]);
    assert_eq!(sized(None, Some(50.0), NONE), [100.0, 50.0]);
    let capped = Limits {
        max: [f32::INFINITY, 30.0],
        ..NONE
    };
    assert_eq!(sized(Some(80.0), None, capped), [80.0, 30.0]);
}

#[test]
fn a_max_height_shrinks_both_lengths_keeping_the_ratio() {
    // CSS 2.1 10.4: h > max-height
    let limits = Limits {
        max: [f32::INFINITY, 10.0],
        ..NONE
    };
    assert_eq!(sized(None, None, limits), [20.0, 10.0]);
}

#[test]
fn a_min_width_grows_both_lengths_keeping_the_ratio() {
    let limits = Limits {
        min: [100.0, 0.0],
        ..NONE
    };
    assert_eq!(sized(None, None, limits), [100.0, 50.0]);
}

#[test]
fn both_lengths_over_their_maximums_follow_the_tighter() {
    // max-width/w = 0.5 <= max-height/h = 0.75: width decides
    let limits = Limits {
        max: [20.0, 15.0],
        ..NONE
    };
    assert_eq!(sized(None, None, limits), [20.0, 10.0]);
}

#[test]
fn a_min_and_a_max_that_conflict_give_both_limits() {
    // w < min-width and h > max-height
    let limits = Limits {
        min: [60.0, 0.0],
        max: [f32::INFINITY, 10.0],
    };
    assert_eq!(sized(None, None, limits), [60.0, 10.0]);
}
