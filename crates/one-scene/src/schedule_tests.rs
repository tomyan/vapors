use super::*;

#[test]
fn a_light_is_clicked_every_two_seconds_cycling_through_the_three() {
    // When
    let clicks: Vec<(usize, usize)> = (0..RUN_FRAMES)
        .filter_map(|frame| click_at(frame).map(|light| (frame, light)))
        .collect();

    // Then: 29 clicks, at 2 s, 4 s, ..., 58 s
    assert_eq!(clicks.len(), 29);
    assert_eq!(&clicks[..4], &[(125, 0), (250, 1), (375, 2), (500, 0)]);
    assert_eq!(click_at(0), None);
}

#[test]
fn a_second_click_on_a_light_turns_it_off() {
    // Then: light 1 on at 2 s, off at 8 s
    assert_eq!(expected(124).lit, [false; 3]);
    assert_eq!(expected(125).lit, [true, false, false]);
    assert_eq!(expected(500).lit, [false, true, true]);
}

#[test]
fn the_night_comes_and_goes_in_turn_and_every_cube_stays() {
    // Then
    assert!(
        (0..RUN_FRAMES)
            .step_by(25)
            .all(|frame| (0..CUBES).all(|cube| expected(frame).cube(cube).1))
    );
    assert!(!night(Duration::from_millis(14_999)) && night(Duration::from_secs(15)));
    assert!(!night(Duration::from_secs(30)) && night(Duration::from_secs(45)));
}

#[test]
fn every_kind_of_change_is_named_in_the_frame_it_is_due() {
    // Given: 15 s is frame 937.5, so frame 938 is the first night
    let names = |frame| changes_at(frame).join(", ");

    // Then
    assert_eq!(names(125), "click light 1");
    assert_eq!(names(313), "");
    assert_eq!(names(625), "click light 2, pointer over the cubes");
    assert_eq!(names(1250), "click light 1, pointer off the cubes");
    assert_eq!(names(938), "night on");
    assert_eq!(
        names(1875),
        "click light 3, night off, pointer over the cubes"
    );
    assert_eq!(names(1), "");
}

#[test]
fn the_group_turns_with_the_clock() {
    // Then
    assert!(turn(Duration::ZERO).abs() < f32::EPSILON);
    assert!((turn(Duration::from_secs(2)) - 1.0).abs() < 1e-6);
}

#[test]
fn the_pointer_rests_over_the_cubes_in_every_other_ten_seconds() {
    // Given
    let at = |millis| pointing(Duration::from_millis(millis));

    // When
    let (before, opened, closed, again) = (at(9_999), at(10_000), at(20_000), at(30_000));

    // Then
    assert_eq!((before, opened, closed, again), (false, true, false, true));
}

#[test]
fn the_hovered_middle_cube_takes_the_groups_hover_colour_over_its_lit_colour() {
    // Given: frame 1100 (17.6 s) is in the pointer's window, light 2
    // lit by the click at frame 1000
    let now = expected(1100);

    // When
    let middle = now.cube(HOVER_CUBE);

    // Then
    assert!(now.hovered && now.lit[HOVER_CUBE]);
    assert_eq!(middle, (GROUP_HOVER, true));
}

#[test]
fn a_lit_cube_the_hover_does_not_colour_keeps_its_lit_colour() {
    // Given: light 1, lit at frame 875, in the window at frame 1100
    let now = expected(1100);

    // When
    let first = now.cube(0);

    // Then
    assert!(now.hovered && now.lit[0]);
    assert_eq!(first.0, LIT);
}

#[test]
fn outside_the_window_the_lit_middle_cube_is_lit() {
    // Given: frame 500 (8 s), light 2 lit at frame 250
    let now = expected(500);

    // When
    let middle = now.cube(HOVER_CUBE);

    // Then
    assert!(!now.hovered);
    assert_eq!(middle.0, LIT);
}
