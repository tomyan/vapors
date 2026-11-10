use super::*;

#[test]
fn an_animating_second_passes_with_exactly_one_frame_that_did_not_render() {
    // Given / When
    let verdict = animating((60, 59));

    // Then
    assert_eq!(verdict, Ok(()));
}

#[test]
fn an_animating_second_is_refused_unless_exactly_one_frame_did_not_render() {
    for (frames, renders) in [(60, 58), (60, 60), (60, 0), (10, 8)] {
        // Given / When
        let verdict = animating((frames, renders));

        // Then
        assert!(verdict.is_err(), "{frames} frames, {renders} renders");
    }
}

#[test]
fn an_animating_second_is_refused_below_the_fewest_frames() {
    // Given / When
    let verdict = animating((MIN_ANIMATING - 1, MIN_ANIMATING - 2));

    // Then
    assert!(verdict.is_err());
}
