//! Special value types (`src/syntax/special.rs`): fonts, shadows, transforms, filters, easing.

mod support;

use support::property_values::{accepts, accepts_as};
use support::refusal;

#[test]
fn font_properties() {
    for decl in [
        "font-family: \"Inter Var\", system-ui, sans-serif",
        "font-family: Inter",
        "font-size: 14px",
        "font-size: 120%",
        "font-size: 1.2em",
        "font-feature-settings: \"tnum\"",
        "font-feature-settings: \"tnum\" 1, \"liga\" off",
        "font-feature-settings: normal",
        "font-variation-settings: \"wght\" 500, \"wdth\" 80",
        "font-variation-settings: normal",
    ] {
        accepts(decl);
    }
    assert!(
        refusal("a { font-feature-settings: tnum; }")
            .message
            .contains("string")
    );
}

#[test]
fn shadow_transform_and_filter_lists() {
    accepts_as(
        "box-shadow: 0 2px 8px rgb(0 0 0 / 40%)",
        "box-shadow: 0px 2px 8px rgb(0 0 0 / 0.4)",
    );
    accepts_as(
        "box-shadow: inset 0 0 0 1px red, 0 1px 2px black",
        "box-shadow: inset 0px 0px 0px 1px rgb(255 0 0), 0px 1px 2px rgb(0 0 0)",
    );
    for decl in [
        "box-shadow: none",
        "transform: none",
        "transform: translate(10px, 20%)",
        "transform: translate(10px)",
        "transform: translateX(1px) translateY(2px)",
        "transform: scale(1.1)",
        "transform: scale(1, 2)",
        "transform: translate(1px, 2px) scale(2)",
        "filter: none",
        "filter: blur(4px)",
    ] {
        accepts(decl);
    }
    for bad in [
        "transform: rotate(45deg)",
        "filter: grayscale(1)",
        "box-shadow: 1px",
    ] {
        let err = refusal(&format!("a {{ {bad}; }}"));
        assert!(
            err.message.contains("translate")
                || err.message.contains("blur")
                || err.message.contains("shadow"),
            "{bad}: {err}"
        );
    }
}

#[test]
fn steps_one_jump_none_is_refused_naming_it() {
    // Given: review item 9; jump-none needs at least two steps.
    let err = refusal("a { transition-timing-function: steps(1, jump-none); }");
    assert!(err.message.contains("jump-none"), "{err}");
    accepts("transition-timing-function: steps(2, jump-none)");
}
