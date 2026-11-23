//! The page a capture loads is the fixture's, with its state's CSS
//! appended and nothing else.

use super::*;

#[test]
fn a_states_page_is_the_fixtures_with_its_css_appended() {
    // Given
    let plain = Reference::of("paint_boxes", None, 1).expect("a fixture");
    let changed = Reference::of("paint_boxes", Some("card_radius_4"), 1).expect("a state");

    // When
    let (plain, changed) = (page(&plain).expect("page"), page(&changed).expect("page"));

    // Then
    assert!(!plain.contains(".card { border-radius: 4px; }"));
    assert!(changed.contains(".card { border-radius: 4px; }"));
    assert_eq!(
        changed.replace(".card { border-radius: 4px; }", ""),
        plain,
        "only the state's CSS differs"
    );
}
