use super::*;

const WINDOW: [f32; 2] = [800.0, 600.0];
const SIZE: [f32; 2] = [100.0, 40.0];

fn pointer_at(x: f32, y: f32, size: [f32; 2], window: [f32; 2]) -> Point {
    place_tooltip(Trigger::Pointer(Point { x, y }), size, window).unwrap()
}

fn keyboard_at(anchor: [f32; 4], size: [f32; 2], window: [f32; 2]) -> Point {
    let [x, y, width, height] = anchor;
    let anchor = BoxRect {
        x,
        y,
        width,
        height,
    };
    place_tooltip(Trigger::Keyboard(anchor), size, window).unwrap()
}

fn at(x: f32, y: f32) -> Point {
    Point { x, y }
}

fn inside(p: Point, size: [f32; 2], window: [f32; 2]) -> bool {
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
                let p = place_tooltip(Trigger::Pointer(hotspot), size, WINDOW).unwrap();
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
fn keyboard_tooltip_is_centred_below_the_element_with_a_gap() {
    let p = keyboard_at([100.0, 100.0, 80.0, 20.0], SIZE, WINDOW);
    assert_eq!(p, at(90.0, 124.0));
}

#[test]
fn keyboard_tooltip_flips_above_when_it_does_not_fit_below() {
    let p = keyboard_at([100.0, 560.0, 80.0, 20.0], SIZE, WINDOW);
    assert_eq!(p, at(90.0, 516.0));
}

#[test]
fn keyboard_tooltip_that_just_fits_below_stays_below() {
    // Given an element whose bottom plus gap plus height is the window's height
    let p = keyboard_at([100.0, 536.0, 80.0, 20.0], SIZE, WINDOW);
    // Then the tooltip's bottom edge is the window's
    assert_eq!(p, at(90.0, 560.0));
}

#[test]
fn keyboard_tooltip_is_clamped_at_each_side() {
    let left = keyboard_at([0.0, 100.0, 20.0, 20.0], SIZE, WINDOW);
    let right = keyboard_at([780.0, 100.0, 20.0, 20.0], SIZE, WINDOW);
    assert_eq!((left.x, right.x), (0.0, 700.0));
}

#[test]
fn keyboard_tooltip_with_no_room_above_or_below_is_placed_below_and_clamped() {
    let p = keyboard_at([100.0, 30.0, 80.0, 20.0], [100.0, 80.0], [800.0, 100.0]);
    assert_eq!(p, at(90.0, 20.0));
}

#[test]
fn unusable_inputs_are_refused_naming_the_input() {
    // Given each kind of bad input
    let hotspot = Trigger::Pointer(at(1.0, 1.0));
    let bad_box = Trigger::Keyboard(BoxRect {
        x: 0.0,
        y: 0.0,
        width: 5.0,
        height: -1.0,
    });
    let cases = [
        (Trigger::Pointer(at(f32::NAN, 1.0)), SIZE, WINDOW, "trigger"),
        (bad_box, SIZE, WINDOW, "trigger"),
        (hotspot, [-1.0, 5.0], WINDOW, "size"),
        (hotspot, [5.0, f32::INFINITY], WINDOW, "size"),
        (hotspot, SIZE, [f32::INFINITY, 600.0], "window"),
    ];
    for (trigger, size, window, input) in cases {
        // When placed
        let result = place_tooltip(trigger, size, window);
        // Then it is refused, naming the input
        assert_eq!(result, Err(PlacementError { input }));
    }
}
