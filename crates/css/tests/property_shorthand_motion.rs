//! `transition` and `animation` shorthands (`src/property/shorthand_motion.rs`).

mod support;

use support::property_shorthand::longhands;
use support::refusal;

#[test]
fn transition_shorthand_expands_per_item() {
    // When
    let got = longhands("transition: color 200ms ease-out 50ms, background-color 1s");

    // Then
    assert_eq!(
        got,
        [
            "transition-property: color, background-color",
            "transition-duration: 200ms, 1s",
            "transition-timing-function: ease-out, ease",
            "transition-delay: 50ms, 0s",
        ]
    );
}

#[test]
fn transition_shorthand_defaults_and_refusals() {
    assert_eq!(
        longhands("transition: all 300ms linear"),
        [
            "transition-property: all",
            "transition-duration: 300ms",
            "transition-timing-function: linear",
            "transition-delay: 0s",
        ]
    );
    assert_eq!(
        longhands("transition: none")[0],
        "transition-property: none"
    );
    // sumi dropped a transition with no duration; vapors refuses it.
    assert!(
        refusal("a { transition: color; }")
            .message
            .contains("duration")
    );
    assert!(
        refusal("a { transition: color 1s 2s 3s; }")
            .message
            .contains("time")
    );
}

#[test]
fn animation_shorthand_classifies_tokens_by_shape() {
    // When
    let got = longhands("animation: pulse 2s infinite ease-in-out");

    // Then
    assert_eq!(
        got,
        [
            "animation-name: pulse",
            "animation-duration: 2s",
            "animation-timing-function: ease-in-out",
            "animation-delay: 0s",
            "animation-iteration-count: infinite",
            "animation-direction: normal",
            "animation-fill-mode: none",
            "animation-play-state: running",
        ]
    );
    let got = longhands("animation: bounce 1s alternate 3 500ms forwards paused");
    assert!(got.contains(&"animation-iteration-count: 3".to_owned()));
    assert!(got.contains(&"animation-direction: alternate".to_owned()));
    assert!(got.contains(&"animation-delay: 500ms".to_owned()));
    assert!(got.contains(&"animation-fill-mode: forwards".to_owned()));
    assert!(got.contains(&"animation-play-state: paused".to_owned()));
    assert!(
        refusal("a { animation: fade; }")
            .message
            .contains("duration")
    );
}
