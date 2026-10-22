//! `@media` evaluation against the environment. Translated from sumi's
//! `runtime/css/media_test.go`; `display-mode: terminal` has no vapors
//! equivalent and is refused as an unknown feature. Range syntax and
//! `resolution` are vapors additions.

mod support;

use support::{chain, css, env, refusal, sheet};
use vapors_css::cascade::Cascade;
use vapors_css::value::ColorScheme;

const RED: &str = "rgb(255 0 0)";
const BLACK: &str = "rgb(0 0 0)";

fn color_under(src: &str, environment: &vapors_css::cascade::Environment) -> String {
    let (tree, leaf) = chain(&["div.card"]);
    let cascade = Cascade::new().author(sheet(src));
    let style = cascade
        .compute(&tree.node(leaf), None, environment)
        .expect("computes");
    css(&style, "color")
}

#[test]
fn unknown_media_feature_is_refused() {
    let err = refusal("@media (display-mode: terminal) { .card { color: red; } }");
    assert!(err.message.contains("display-mode"), "{err}");
}

#[test]
fn media_rules_keep_cascade_order() {
    // Given: a media rule later in the sheet overrides an equal-specificity rule.
    let src = ".card { color: red; } @media (min-width: 1px) { .card { color: green; } }";
    assert_eq!(color_under(src, &env()), "rgb(0 128 0)");
}

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
fn combined_media_conditions() {
    let src =
        "@media screen and (min-width: 400px) and (max-width: 1000px) { .card { color: red; } }";
    assert_eq!(color_under(src, &env()), RED);
    let mut e = env();
    e.viewport_width = 1200.0;
    assert_eq!(color_under(src, &e), BLACK);
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
fn media_query_list_not_and_or() {
    let mut e = env();
    e.viewport_width = 500.0;
    assert_eq!(
        color_under(
            "@media (min-width: 900px), (max-width: 600px) { .card { color: red; } }",
            &e
        ),
        RED
    );
    assert_eq!(
        color_under(
            "@media not screen and (min-width: 600px) { .card { color: red; } }",
            &e
        ),
        RED
    );
    assert_eq!(
        color_under(
            "@media (min-width: 900px) or (max-width: 600px) { .card { color: red; } }",
            &e
        ),
        RED
    );
    assert_eq!(
        color_under("@media print { .card { color: red; } }", &e),
        BLACK
    );
}

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

#[test]
fn malformed_media_queries_are_refused_with_a_position() {
    for src in [
        "@media (min-width) { }",
        "@media (min-width: red) { }",
        "@media (min-width: 600px) and { }",
        "@media (prefers-color-scheme: blue) { }",
        "@media tv { }",
    ] {
        let err = refusal(src);
        assert_eq!(err.position.column, 8, "{src}: {err}");
    }
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

#[test]
fn not_before_a_condition_is_accepted_and_only_needs_a_type() {
    // Given: review item 19.
    let mut e = env();
    e.viewport_width = 500.0;
    assert_eq!(
        color_under(
            "@media not (min-width: 600px) { .card { color: red; } }",
            &e
        ),
        RED
    );
    assert_eq!(
        color_under("@media only screen { .card { color: red; } }", &e),
        RED
    );
    let err = refusal("@media only (min-width: 1px) { }");
    assert!(
        err.message.contains("only") && err.message.contains("media type"),
        "{err}"
    );
}
