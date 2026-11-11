use std::hash::BuildHasher;

use rustc_hash::FxBuildHasher;
use vapors_dom::NodeId;
use vapors_text::{Align, LineHeight};

use super::*;
use crate::style::Face;

/// "some words" with one span over "some", held by `node`.
fn run(node: u64) -> InlineRun {
    let style = TextStyle {
        face: Face::Geist,
        size: 16.0,
        weight: 400.0,
        line_height: LineHeight::Normal,
        color: vapors_scene::Color::default(),
        letter_spacing: 0.0,
        align: Align::Start,
        italic: false,
    };
    let span = InlineSpan {
        node: NodeId(node),
        range: 0..4,
        visible: true,
        style,
    };
    InlineRun {
        text: "some words".to_owned(),
        style,
        visible: true,
        spans: vec![span],
    }
}

#[test]
fn keys_of_one_run_are_equal_and_hash_alike_and_a_span_of_another_node_is_another_key() {
    // Given
    let (first, again, other) = (run(1), run(1), run(2));

    // When
    let keys = [&first, &again, &other].map(|run| Key::of(run, Some(80.0), 1.0));

    // Then
    assert!(keys[0] == keys[1]);
    assert_eq!(
        FxBuildHasher.hash_one(&keys[0]),
        FxBuildHasher.hash_one(&keys[1])
    );
    assert!(keys[0] != keys[2]);
}

#[test]
fn keys_with_the_same_hash_and_different_parts_are_not_equal() {
    // Given
    let (first, other) = (
        Key::of(&run(1), Some(80.0), 1.0),
        Key::of(&run(2), Some(80.0), 1.0),
    );
    let forced = Key {
        hash: first.hash,
        parts: other.parts.clone(),
    };

    // When
    let equal = first == forced;

    // Then
    assert!(!equal);
}
