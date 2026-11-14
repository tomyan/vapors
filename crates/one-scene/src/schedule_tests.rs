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
fn the_third_cube_hides_and_the_night_comes_in_turn() {
    // Then
    assert!(!gone(Duration::from_millis(4_999)) && gone(Duration::from_secs(5)));
    assert!(!gone(Duration::from_secs(10)));
    assert!(!night(Duration::from_millis(14_999)) && night(Duration::from_secs(15)));
    assert!(!night(Duration::from_secs(30)) && night(Duration::from_secs(45)));
}

#[test]
fn every_kind_of_change_is_named_in_the_frame_it_is_due() {
    // Given: 5 s is frame 312.5, so frame 313 is the first hidden
    let names = |frame| changes_at(frame).join(", ");

    // Then
    assert_eq!(names(125), "click light 1");
    assert_eq!(names(313), "cube 3 hidden");
    assert_eq!(names(625), "click light 2, cube 3 shown");
    assert_eq!(names(938), "cube 3 hidden, night on");
    assert_eq!(names(1), "");
}

#[test]
fn the_group_turns_with_the_clock() {
    // Then
    assert!(turn(Duration::ZERO).abs() < f32::EPSILON);
    assert!((turn(Duration::from_secs(2)) - 1.0).abs() < 1e-6);
}
