use super::tests::{SIZE, WINDOW, at, inside, keyboard_at};

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
fn keyboard_tooltip_for_an_anchor_spanning_the_window_may_cover_it() {
    // Given an anchor as tall as the window, so no room above or below
    let p = keyboard_at([100.0, 0.0, 80.0, 600.0], SIZE, WINDOW);
    // Then the tooltip is below the anchor's bottom, clamped at the window's
    assert_eq!(p, at(90.0, 560.0));
    // and it overlaps the anchor, as docs/tooltips.md says
    assert!(p.y < 600.0);
}

#[test]
fn keyboard_tooltip_for_an_anchor_partly_off_screen_is_always_inside_the_window() {
    // Given anchors on a grid reaching past every edge, and several sizes
    let sizes = [[100.0, 40.0], [300.0, 200.0], [790.0, 20.0], [50.0, 590.0]];
    let starts = (-5..=45_i16).map(|n| f32::from(n) * 20.0);
    for size in sizes {
        for x in starts.clone() {
            for y in starts.clone() {
                // When placed
                let p = keyboard_at([x, y, 120.0, 60.0], size, WINDOW);
                // Then it is inside the window
                assert!(inside(p, size, WINDOW), "{size:?} anchor ({x}, {y}): {p:?}");
            }
        }
    }
}

#[test]
fn keyboard_anchor_entirely_off_screen_is_accepted_and_clamped() {
    let p = keyboard_at([-500.0, -500.0, 20.0, 20.0], SIZE, WINDOW);
    assert!(inside(p, SIZE, WINDOW), "{p:?}");
}
