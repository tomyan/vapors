//! Media features (`src/cascade/media_feature.rs`). Translated from sumi `media_test.go`.

mod support;

use support::cascade_media::{BLACK, RED, color_under};
use support::env;
use vapors_css::value::ColorScheme;

#[test]
fn min_width_media_against_viewport() {
    // Given
    let src = "@media (min-width: 600px) { .card { color: red; } }";
    let mut e = env();

    // Then
    e.viewport_width = 800.0;
    assert_eq!(color_under(src, &e), RED);
    e.viewport_width = 500.0;
    assert_eq!(color_under(src, &e), BLACK);
}

#[test]
fn max_height_media() {
    let src = "@media (max-height: 400px) { .card { color: red; } }";
    let mut e = env();
    e.viewport_height = 300.0;
    assert_eq!(color_under(src, &e), RED);
    e.viewport_height = 500.0;
    assert_eq!(color_under(src, &e), BLACK);
}

#[test]
fn prefers_color_scheme_media() {
    let src = "@media (prefers-color-scheme: dark) { .card { color: white; } } @media (prefers-color-scheme: light) { .card { color: red; } }";
    let mut e = env();
    e.color_scheme = ColorScheme::Dark;
    assert_eq!(color_under(src, &e), "rgb(255 255 255)");
    e.color_scheme = ColorScheme::Light;
    assert_eq!(color_under(src, &e), RED);
}

#[test]
fn prefers_reduced_motion_media() {
    let src = "@media (prefers-reduced-motion: reduce) { .card { color: red; } }";
    let mut e = env();
    assert_eq!(color_under(src, &e), BLACK);
    e.reduced_motion = true;
    assert_eq!(color_under(src, &e), RED);
    let src = "@media (prefers-reduced-motion: no-preference) { .card { color: red; } }";
    assert_eq!(color_under(src, &e), BLACK);
}

#[test]
fn resolution_media() {
    let mut e = env();
    e.resolution = 2.0;
    assert_eq!(
        color_under(
            "@media (min-resolution: 2dppx) { .card { color: red; } }",
            &e
        ),
        RED
    );
    assert_eq!(
        color_under("@media (resolution >= 1.5x) { .card { color: red; } }", &e),
        RED
    );
    e.resolution = 1.0;
    assert_eq!(
        color_under(
            "@media (min-resolution: 2dppx) { .card { color: red; } }",
            &e
        ),
        BLACK
    );
}
