use super::*;

const VIEWPORT: Size = Size {
    width: 100,
    height: 100,
};

fn scissor(x: u32, y: u32, width: u32, height: u32) -> Scissor {
    Scissor {
        x,
        y,
        width,
        height,
    }
}

#[test]
fn an_unclipped_batch_draws_where_the_target_does() {
    assert_eq!(region(None, None, VIEWPORT), Region::Whole);
    let outer = scissor(5, 5, 10, 10);
    assert_eq!(region(Some(outer), None, VIEWPORT), Region::Within(outer));
}

#[test]
fn a_clipped_batch_draws_inside_its_clip_and_the_targets_scissor() {
    // Given
    let clip = Rect::new(10.0, 10.0, 50.0, 50.0);

    // When / Then
    assert_eq!(
        region(None, Some(clip), VIEWPORT),
        Region::Within(scissor(10, 10, 40, 40))
    );
    assert_eq!(
        region(Some(scissor(0, 0, 20, 100)), Some(clip), VIEWPORT),
        Region::Within(scissor(10, 10, 10, 40))
    );
}

#[test]
fn a_batch_clipped_to_nothing_is_not_drawn() {
    // Given: a clip off the target, and one outside the target's scissor
    let off = Rect::new(200.0, 0.0, 300.0, 10.0);
    let beside = Rect::new(50.0, 0.0, 60.0, 10.0);

    // When / Then
    assert_eq!(region(None, Some(off), VIEWPORT), Region::Nowhere);
    assert_eq!(
        region(Some(scissor(0, 0, 20, 20)), Some(beside), VIEWPORT),
        Region::Nowhere
    );
}
