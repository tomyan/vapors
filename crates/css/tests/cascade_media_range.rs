//! Media range syntax (`src/cascade/media_range.rs`). vapors-specific.

mod support;

use support::cascade_media::{BLACK, RED, color_under};
use support::env;

#[test]
fn media_range_syntax() {
    let mut e = env();
    e.viewport_width = 500.0;
    assert_eq!(
        color_under("@media (width >= 400px) { .card { color: red; } }", &e),
        RED
    );
    assert_eq!(
        color_under(
            "@media (400px < width < 600px) { .card { color: red; } }",
            &e
        ),
        RED
    );
    assert_eq!(
        color_under("@media (width > 500px) { .card { color: red; } }", &e),
        BLACK
    );
}

#[test]
fn media_range_operators_on_either_side_and_two_sided() {
    // Given: review item 6.
    let mut e = env();
    e.viewport_width = 500.0;
    for query in [
        "(width <= 600px)",
        "(600px >= width)",
        "(width >= 400px)",
        "(400px <= width)",
        "(400px <= width <= 800px)",
        "(800px >= width >= 400px)",
        "(width = 500px)",
    ] {
        assert_eq!(
            color_under(&format!("@media {query} {{ .card {{ color: red; }} }}"), &e),
            RED,
            "{query}"
        );
    }
    for query in [
        "(width >= 600px)",
        "(600px <= width)",
        "(600px <= width <= 800px)",
        "(width < 500px)",
    ] {
        assert_eq!(
            color_under(&format!("@media {query} {{ .card {{ color: red; }} }}"), &e),
            BLACK,
            "{query}"
        );
    }
}

#[test]
fn media_vw_resolves_against_the_environment_viewport() {
    let mut e = env();
    e.viewport_width = 500.0;
    assert_eq!(
        color_under("@media (width >= 50vw) { .card { color: red; } }", &e),
        RED
    );
    assert_eq!(
        color_under("@media (width >= 150vw) { .card { color: red; } }", &e),
        BLACK
    );
}
