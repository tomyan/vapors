use super::*;

/// A 100 x 50 padding box at (10, 20).
const PADDING: BoxRect = BoxRect {
    x: 10.0,
    y: 20.0,
    width: 100.0,
    height: 50.0,
};

#[test]
fn a_vertical_thumb_runs_down_the_right_edge_in_proportion() {
    // Given: a 200 px tall box showing half its content, at the start
    let tall = BoxRect {
        height: 200.0,
        ..PADDING
    };
    let found = thumbs(tall, [0.0, 0.0], [0.0, 200.0], 8.0);

    // Then: a 196 px track (2 px in at each end), half of it
    assert_eq!(
        found,
        [BoxRect {
            x: 10.0 + 100.0 - 2.0 - 8.0,
            y: 22.0,
            width: 8.0,
            height: 98.0,
        }]
    );
}

#[test]
fn the_thumb_reaches_the_end_of_its_track_at_the_end() {
    // When
    let found = thumbs(PADDING, [0.0, 150.0], [0.0, 150.0], 8.0);

    // Then: its bottom is 2 px above the padding box's
    let thumb = found[0];
    assert!(
        (thumb.y + thumb.height - (20.0 + 50.0 - 2.0)).abs() < 1e-4,
        "{thumb:?}"
    );
}

#[test]
fn a_thumb_is_never_shorter_than_its_minimum() {
    // Given: 50 px shown of 5,050
    let found = thumbs(PADDING, [0.0, 0.0], [0.0, 5000.0], 8.0);

    // Then
    assert_eq!(found[0].height, MIN_LENGTH);
}

#[test]
fn both_thumbs_leave_the_corner_to_each_other() {
    // When: scrolled to the end both ways, so each thumb is at its
    // track's end, by the corner
    let found = thumbs(PADDING, [100.0, 150.0], [100.0, 150.0], 8.0);

    // Then: neither runs into the 10 px corner
    let [vertical, horizontal] = [found[0], found[1]];
    assert!(
        vertical.y + vertical.height <= 20.0 + 50.0 - 2.0 - 8.0,
        "{vertical:?}"
    );
    assert!(
        horizontal.x + horizontal.width <= 10.0 + 100.0 - 2.0 - 8.0,
        "{horizontal:?}"
    );
    assert_eq!(horizontal.y, 20.0 + 50.0 - 2.0 - 8.0);
}

#[test]
fn nothing_to_scroll_or_no_width_draws_no_thumb() {
    assert_eq!(thumbs(PADDING, [0.0, 0.0], [0.0, 0.0], 8.0), []);
    assert_eq!(thumbs(PADDING, [0.0, 0.0], [0.0, 150.0], 0.0), []);
}
