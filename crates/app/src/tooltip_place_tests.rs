use super::*;

pub(super) const WINDOW: [f32; 2] = [800.0, 600.0];
pub(super) const SIZE: [f32; 2] = [100.0, 40.0];

fn pointer_at(x: f32, y: f32, size: [f32; 2], window: [f32; 2]) -> Point {
    place_tooltip(Anchor::Pointer(Point { x, y }), size, window).unwrap()
}

pub(super) fn keyboard_at(anchor: [f32; 4], size: [f32; 2], window: [f32; 2]) -> Point {
    let [x, y, width, height] = anchor;
    let anchor = BoxRect {
        x,
        y,
        width,
        height,
    };
    place_tooltip(Anchor::Keyboard(anchor), size, window).unwrap()
}

pub(super) fn at(x: f32, y: f32) -> Point {
    Point { x, y }
}

pub(super) fn inside(p: Point, size: [f32; 2], window: [f32; 2]) -> bool {
    p.x >= 0.0 && p.y >= 0.0 && p.x + size[0] <= window[0] && p.y + size[1] <= window[1]
}

#[test]
fn pointer_in_the_middle_places_at_the_cursor_offset() {
    // Given a hotspot with room on every side
    // When placed
    let p = pointer_at(300.0, 200.0, SIZE, WINDOW);
    // Then the corner is the hotspot plus (10, 15)
    assert_eq!(p, at(310.0, 215.0));
}

#[test]
fn pointer_at_the_right_edge_slides_left_into_the_window() {
    assert_eq!(pointer_at(780.0, 200.0, SIZE, WINDOW), at(700.0, 215.0));
}

#[test]
fn pointer_at_the_left_edge_keeps_the_offset() {
    assert_eq!(pointer_at(0.0, 200.0, SIZE, WINDOW), at(10.0, 215.0));
}

#[test]
fn pointer_at_the_top_edge_goes_below() {
    assert_eq!(pointer_at(300.0, 0.0, SIZE, WINDOW), at(310.0, 15.0));
}

#[test]
fn pointer_at_the_bottom_edge_flips_above_with_its_bottom_on_the_hotspot() {
    assert_eq!(pointer_at(300.0, 590.0, SIZE, WINDOW), at(310.0, 550.0));
}

#[test]
fn pointer_at_each_window_corner_is_inside_and_clear() {
    // Given the four corners
    let cases = [
        (0.0, 0.0, at(10.0, 15.0)),
        (799.0, 0.0, at(700.0, 15.0)),
        (0.0, 599.0, at(10.0, 559.0)),
        (799.0, 599.0, at(700.0, 559.0)),
    ];
    for (x, y, expected) in cases {
        // When placed
        let p = pointer_at(x, y, SIZE, WINDOW);
        // Then the corner is as expected
        assert_eq!(p, expected, "hotspot ({x}, {y})");
    }
}

#[test]
fn pointer_hotspot_is_never_covered_by_a_tooltip_that_fits_the_window() {
    // Given hotspots on a grid, including the edges, and several sizes
    let sizes = [[100.0, 40.0], [300.0, 200.0], [790.0, 20.0], [50.0, 590.0]];
    for size in sizes {
        for hx in (0..=800_u16).step_by(20) {
            for hy in (0..=600_u16).step_by(20) {
                let hotspot = at(f32::from(hx), f32::from(hy));
                // When placed
                let p = place_tooltip(Anchor::Pointer(hotspot), size, WINDOW).unwrap();
                // Then it is inside the window and the hotspot is outside it
                assert!(inside(p, size, WINDOW), "{size:?} at {hotspot:?}: {p:?}");
                let covered = hotspot.x >= p.x
                    && hotspot.x < p.x + size[0]
                    && hotspot.y >= p.y
                    && hotspot.y < p.y + size[1];
                assert!(!covered, "{size:?} covers {hotspot:?} at {p:?}");
            }
        }
    }
}

#[test]
fn pointer_in_a_short_window_goes_right_of_the_hotspot() {
    // Given a window too short for the tooltip above or below
    // When placed
    let p = pointer_at(300.0, 50.0, [100.0, 80.0], [800.0, 100.0]);
    // Then it is beside the hotspot, slid down into the window
    assert_eq!(p, at(310.0, 20.0));
}

#[test]
fn pointer_in_a_short_window_at_the_right_edge_goes_left_of_the_hotspot() {
    let p = pointer_at(750.0, 50.0, [100.0, 80.0], [800.0, 100.0]);
    assert_eq!(p, at(650.0, 20.0));
}

#[test]
fn pointer_tooltip_with_no_clear_side_is_placed_below_and_clamped_even_over_the_hotspot() {
    // Given a window where above, below, right and left all fail
    let (size, window) = ([150.0, 80.0], [200.0, 100.0]);
    // When placed
    let p = pointer_at(100.0, 50.0, size, window);
    // Then it is the clamped below position
    assert_eq!(p, at(50.0, 20.0));
}

#[test]
fn tooltip_larger_than_the_window_is_placed_at_the_top_left() {
    // Given a tooltip larger than the window on both sides
    let size = [900.0, 700.0];
    // When placed, for either trigger
    let by_pointer = pointer_at(300.0, 300.0, size, WINDOW);
    let by_keyboard = keyboard_at([300.0, 300.0, 80.0, 20.0], size, WINDOW);
    // Then the top-left corner is the window's, and the rest overflows
    assert_eq!(by_pointer, at(0.0, 0.0));
    assert_eq!(by_keyboard, at(0.0, 0.0));
}

#[test]
fn pointer_hotspot_outside_the_window_is_accepted_and_clamped() {
    // Given hotspots past each edge, as a captured drag can produce
    for (x, y) in [
        (-50.0, 300.0),
        (900.0, 300.0),
        (300.0, -50.0),
        (300.0, 700.0),
    ] {
        // When placed
        let p = pointer_at(x, y, SIZE, WINDOW);
        // Then it is inside the window
        assert!(inside(p, SIZE, WINDOW), "hotspot ({x}, {y}): {p:?}");
    }
}

#[test]
fn pointer_beside_placements_are_half_open_at_the_hotspot_column() {
    // Given a window too short above or below, hotspot x = 700 so right starts at 710
    // When the right candidate's right edge is exactly the window's
    let right = pointer_at(690.0, 50.0, [100.0, 80.0], [800.0, 100.0]);
    // Then it is used
    assert_eq!(right.x, 700.0);
    // When it is one px over, the left candidate ends on the hotspot column
    let left = pointer_at(691.0, 50.0, [100.0, 80.0], [800.0, 100.0]);
    assert_eq!(left.x, 591.0);
}
