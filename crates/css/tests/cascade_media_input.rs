//! The input and shape media features: `orientation`, `aspect-ratio`,
//! `pointer` and `hover` (`src/cascade/media_input.rs`).

use crate::support;

use support::cascade_media::{BLACK, RED, color_under};
use support::{env, refusal};
use vapors_css::cascade::Pointer;

fn red_under(feature: &str, e: &vapors_css::cascade::Environment) -> bool {
    color_under(
        &format!("@media ({feature}) {{ .card {{ color: red; }} }}"),
        e,
    ) == RED
}

fn sized(width: f64, height: f64) -> vapors_css::cascade::Environment {
    let mut e = env();
    e.viewport_width = width;
    e.viewport_height = height;
    e
}

#[test]
fn orientation_is_portrait_when_taller_and_a_square_is_landscape() {
    for ((w, h), portrait) in [
        ((400.0, 800.0), true),
        ((800.0, 400.0), false),
        ((500.0, 500.0), false),
    ] {
        let e = sized(w, h);
        assert_eq!(red_under("orientation: portrait", &e), portrait, "{w}x{h}");
        assert_eq!(
            red_under("orientation: landscape", &e),
            !portrait,
            "{w}x{h}"
        );
        assert_eq!(e.is_portrait(), portrait);
    }
}

#[test]
fn aspect_ratio_with_min_and_max() {
    let wide = sized(1600.0, 900.0);
    assert!(red_under("aspect-ratio: 16/9", &wide));
    assert!(red_under("min-aspect-ratio: 16 / 9", &wide));
    assert!(red_under("max-aspect-ratio: 2/1", &wide));
    assert!(!red_under("max-aspect-ratio: 3/2", &wide));
    assert!(!red_under("min-aspect-ratio: 2", &wide));
    assert!(red_under("min-aspect-ratio: 1.5", &wide));
}

#[test]
fn pointer_and_hover_match_the_primary_input() {
    let mut e = env();
    assert!(red_under("pointer: fine", &e) && red_under("hover: hover", &e));
    e.pointer = Pointer::Coarse;
    e.hover = false;
    assert!(red_under("pointer: coarse", &e) && !red_under("pointer: fine", &e));
    assert!(red_under("hover: none", &e) && !red_under("hover: hover", &e));
    e.pointer = Pointer::None;
    assert!(red_under("pointer: none", &e));
    assert_eq!(
        color_under("@media (pointer: fine) { .card { color: red; } }", &e),
        BLACK
    );
}

#[test]
fn values_outside_the_lists_are_refused_naming_the_accepted_ones() {
    for (src, accepted) in [
        (
            "@media (orientation: sideways) { }",
            "portrait or landscape",
        ),
        ("@media (pointer: huge) { }", "none, coarse or fine"),
        ("@media (hover: sometimes) { }", "none or hover"),
        ("@media (aspect-ratio: wide) { }", "<number> / <number>"),
        ("@media (aspect-ratio: 16/0) { }", "<number> / <number>"),
    ] {
        let err = refusal(src);
        assert!(err.message.contains(accepted), "{err}");
        assert_eq!(err.position.line, 1, "{err}");
    }
}
