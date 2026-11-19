use super::*;

#[test]
fn a_colour_keeps_its_srgb_channels_with_alpha_one() {
    // When
    let colour = Colour::srgb(1.0, 0.5, 0.0).unwrap();

    // Then
    assert_eq!(colour.rgba(), [1.0, 0.5, 0.0, 1.0]);
}

#[test]
fn a_channel_outside_zero_to_one_is_refused_by_name() {
    // When
    let refused = Colour::srgb(0.0, 1.5, 0.0);

    // Then
    assert_eq!(
        refused,
        Err(SceneError::ChannelOutOfRange {
            channel: "green",
            value: 1.5
        })
    );
}

#[test]
fn a_non_finite_channel_is_refused_naming_the_channel() {
    let cases = [
        (Colour::srgb(f32::NAN, 0.0, 0.0), "Colour::srgb red"),
        (Colour::srgb(0.0, f32::INFINITY, 0.0), "Colour::srgb green"),
        (Colour::srgb(0.0, 0.0, f32::NAN), "Colour::srgb blue"),
    ];
    for (refused, field) in cases {
        // Then
        assert_eq!(refused, Err(SceneError::NonFinite { field }));
    }
}

#[test]
fn the_channel_limits_zero_and_one_are_accepted() {
    // When
    let colour = Colour::srgb(0.0, 1.0, 0.0).unwrap();

    // Then
    assert_eq!(colour.rgba(), [0.0, 1.0, 0.0, 1.0]);
}

#[test]
fn a_negative_channel_is_refused_by_name() {
    // When
    let refused = Colour::srgb(0.0, 0.0, -0.1);

    // Then
    assert_eq!(
        refused,
        Err(SceneError::ChannelOutOfRange {
            channel: "blue",
            value: -0.1
        })
    );
}

#[test]
fn linear_undoes_the_srgb_transfer_function() {
    // Given: the toe, the knee's neighbour and the ends
    let colour = Colour::srgb(0.0, 0.04, 1.0).unwrap();

    // When
    let [r, g, b] = colour.linear();

    // Then
    assert_eq!(r, 0.0);
    assert!((g - 0.04 / 12.92).abs() < 1e-7);
    assert!((b - 1.0).abs() < 1e-6);
    assert!((Colour::srgb(0.5, 0.5, 0.5).unwrap().linear()[0] - 0.214_041).abs() < 1e-5);
}
