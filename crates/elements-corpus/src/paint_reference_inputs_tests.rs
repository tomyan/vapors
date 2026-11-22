//! The tree's manifest fields: the pinned browser, as `vapors_text`
//! launches it, and its render flags.

use std::path::Path;

use serde_json::{Map, json};

use super::*;

#[test]
fn the_pin_read_here_is_the_build_vapors_text_launches() {
    // Given
    let version = pinned("version").expect("pinned");

    // When
    let launched = vapors_text::comparison::pinned_binary(Path::new("/home"));

    // Then
    let expected = format!("/home/.local/share/factory/chromium-{version}/");
    assert!(
        launched.to_string_lossy().starts_with(&expected),
        "{launched:?}, {expected}"
    );
}

#[test]
fn the_trees_fields_record_the_pinned_browser_and_its_render_flags() {
    // When
    let fields = expected("text_line_box_paint", 2).expect("fields");

    // Then
    let flags = vapors_text::comparison::render_flags(WIDTH, HEIGHT, 2.0);
    assert_eq!(fields["flags"], json!(flags));
    assert_eq!(
        fields["browser"]["version"],
        json!(pinned("version").expect("pinned"))
    );
    assert_eq!(fields["fonts"].as_object().map(Map::len), Some(2));
}
