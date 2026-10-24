use super::*;

const VIEWPORT: Size = Size {
    width: 100,
    height: 50,
};

#[test]
fn a_clip_inside_the_viewport_is_its_own_scissor() {
    // Given
    let clip = Rect::new(10.0, 5.0, 30.0, 25.0);

    // When
    let scissor = clip.scissor(VIEWPORT);

    // Then
    assert_eq!(
        scissor,
        Some(Scissor {
            x: 10,
            y: 5,
            width: 20,
            height: 20
        })
    );
}

#[test]
fn a_clip_is_cut_to_the_viewport() {
    // Given: a clip past every edge
    let clip = Rect::new(-10.0, -5.0, 130.0, 60.0);

    // When
    let scissor = clip.scissor(VIEWPORT);

    // Then
    assert_eq!(
        scissor,
        Some(Scissor {
            x: 0,
            y: 0,
            width: 100,
            height: 50
        })
    );
}

#[test]
fn fractional_edges_round_to_the_nearest_pixel() {
    // Given
    let clip = Rect::new(10.4, 5.6, 30.5, 25.2);

    // When
    let scissor = clip.scissor(VIEWPORT);

    // Then
    assert_eq!(
        scissor,
        Some(Scissor {
            x: 10,
            y: 6,
            width: 21,
            height: 19
        })
    );
}

#[test]
fn a_clip_with_no_area_in_the_viewport_has_no_scissor() {
    // Given: empty, crossed, and wholly outside
    let clips = [
        Rect::new(10.0, 10.0, 10.0, 20.0),
        Rect::new(20.0, 10.0, 10.0, 20.0),
        Rect::new(200.0, 10.0, 220.0, 20.0),
        Rect::new(10.0, -30.0, 20.0, -10.0),
    ];

    // When
    let scissors: Vec<Option<Scissor>> = clips.iter().map(|c| c.scissor(VIEWPORT)).collect();

    // Then
    assert_eq!(scissors, [None, None, None, None]);
}

#[test]
fn two_scissors_intersect_to_their_overlap() {
    // Given
    let a = Scissor {
        x: 0,
        y: 0,
        width: 20,
        height: 20,
    };
    let b = Scissor {
        x: 10,
        y: 5,
        width: 30,
        height: 10,
    };

    // When
    let both = a.intersect(b);

    // Then
    assert_eq!(
        both,
        Some(Scissor {
            x: 10,
            y: 5,
            width: 10,
            height: 10
        })
    );
}

#[test]
fn scissors_that_only_touch_have_no_intersection() {
    // Given
    let a = Scissor {
        x: 0,
        y: 0,
        width: 10,
        height: 10,
    };
    let b = Scissor {
        x: 10,
        y: 0,
        width: 10,
        height: 10,
    };

    // When
    let both = a.intersect(b);

    // Then
    assert_eq!(both, None);
}
