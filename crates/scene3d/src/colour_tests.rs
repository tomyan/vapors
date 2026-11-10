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
fn a_nan_channel_is_refused() {
    // When
    let refused = Colour::srgb(f32::NAN, 0.0, 0.0);

    // Then
    assert!(matches!(refused, Err(SceneError::NonFinite { .. })));
}
