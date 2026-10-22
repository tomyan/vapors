//! Media query lists (`src/cascade/media.rs`). Translated from sumi `media_test.go`.

mod support;

use support::cascade_media::{BLACK, RED, color_under};
use support::{env, refusal};

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
fn combined_media_conditions() {
    let src =
        "@media screen and (min-width: 400px) and (max-width: 1000px) { .card { color: red; } }";
    assert_eq!(color_under(src, &env()), RED);
    let mut e = env();
    e.viewport_width = 1200.0;
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
