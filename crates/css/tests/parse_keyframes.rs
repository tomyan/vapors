//! `@keyframes` blocks (`src/parse/keyframes.rs`). Translated from sumi `style_keyframe_test.go`.

mod support;

use support::{refusal, sheet};

#[test]
fn parse_keyframes_basic() {
    // Given
    let input = "\n@keyframes pulse {\n  0% { color: #50fa7b; }\n  50% { color: #2d8a4e; }\n  100% { color: #50fa7b; }\n}\n";

    // When
    let s = sheet(input);

    // Then
    assert_eq!(s.keyframes.len(), 1);
    let kf = &s.keyframes[0];
    assert_eq!(kf.name, "pulse");
    assert_eq!(kf.stops.len(), 3);
    assert_eq!(kf.stops[0].offset, 0.0);
    assert_eq!(kf.stops[1].offset, 0.5);
    assert_eq!(kf.stops[2].offset, 1.0);
    assert_eq!(kf.stops[0].declarations[0].value_text(), "#50fa7b");
}

#[test]
fn parse_keyframes_from_to() {
    // When
    let s = sheet("\n@keyframes fade {\n  from { color: red; }\n  to { color: blue; }\n}\n");

    // Then
    let kf = &s.keyframes[0];
    assert_eq!(kf.stops.len(), 2);
    assert_eq!(kf.stops[0].offset, 0.0);
    assert_eq!(kf.stops[1].offset, 1.0);
}

#[test]
fn parse_keyframes_with_rules() {
    // When
    let s = sheet(
        "\n.dot { color: green; }\n@keyframes pulse {\n  0% { color: red; }\n  100% { color: blue; }\n}\n.other { font-weight: bold; }\n",
    );

    // Then
    assert_eq!((s.rules.len(), s.keyframes.len()), (2, 1));
}

#[test]
fn parse_multiple_keyframes() {
    // When
    let s = sheet(
        "\n@keyframes a { from { color: red; } to { color: blue; } }\n@keyframes b { 0% { opacity: 1; } 100% { opacity: 0; } }\n",
    );

    // Then
    assert_eq!(s.keyframes.len(), 2);
    assert_eq!(
        (s.keyframes[0].name.as_str(), s.keyframes[1].name.as_str()),
        ("a", "b")
    );
}

#[test]
fn keyframe_selector_list_makes_one_stop_per_offset() {
    // When
    let s = sheet("@keyframes blink { 0%, 100% { opacity: 1; } 50% { opacity: 0; } }");

    // Then
    let offsets: Vec<f64> = s.keyframes[0].stops.iter().map(|st| st.offset).collect();
    assert_eq!(offsets, [0.0, 1.0, 0.5]);
}

#[test]
fn keyframes_missing_name_is_refused() {
    // When
    let err = refusal("@keyframes { from { color: red; } }");

    // Then
    assert!(
        err.message.contains("@keyframes") && err.message.contains("name"),
        "{err}"
    );
}

#[test]
fn keyframe_invalid_selector_is_refused_naming_it() {
    // When
    let err = refusal("@keyframes x { middle { color: red; } }");

    // Then
    assert_eq!(err.position.column, 16);
    assert!(err.message.contains("middle"), "{err}");
}

#[test]
fn keyframes_none_and_important_in_keyframes_are_refused() {
    // Given: review item 18.
    let err = refusal("@keyframes none { from { opacity: 0; } }");
    assert!(err.message.contains("none"), "{err}");
    let err = refusal("@keyframes x { from { opacity: 0 !important; } }");
    assert!(err.message.contains("!important"), "{err}");
}
