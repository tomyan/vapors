//! Keyframe resolution (`src/animation/keyframes.rs`). Translated from sumi `resolve_keyframes_test.go`.

mod support;

use support::animation_sample::pulse_animation;
use support::{cascade, chain, css, env, sheet};
use vapors_css::animation::animations;
use vapors_css::cascade::Cascade;
use vapors_css::property::PropertyId;
use vapors_css::value::ColorScheme;

#[test]
fn keyframe_stops_resolve_var_from_node_scope() {
    // Given: --glow set on the root, referenced inside @keyframes.
    let src = "html { --glow: red; } .pulse { animation: pulse 1s linear both; } @keyframes pulse { from { color: var(--glow); } to { color: blue; } }";
    let anim = pulse_animation(src, ColorScheme::Light);
    let color = PropertyId::from_name("color").expect("color");

    // Then
    let at = |ms: f64| {
        anim.sample(ms)
            .values
            .into_iter()
            .find(|(id, _)| *id == color)
            .map(|(_, v)| v.to_css())
    };
    assert_eq!(at(0.0).as_deref(), Some("rgb(255 0 0)"));
    assert_eq!(at(1000.0).as_deref(), Some("rgb(0 0 255)"));
}

#[test]
fn keyframe_stops_resolve_light_dark_for_the_scheme() {
    let src = ".pulse { animation: fade 1s; } @keyframes fade { from { color: light-dark(white, black); } to { color: red; } }";
    let color = PropertyId::from_name("color").expect("color");
    let dark = pulse_animation(src, ColorScheme::Dark);
    let first = dark
        .sample(0.0)
        .values
        .into_iter()
        .find(|(id, _)| *id == color)
        .map(|(_, v)| v.to_css());
    assert_eq!(first.as_deref(), Some("rgb(0 0 0)"));
}

#[test]
fn unknown_keyframes_name_is_refused() {
    let (tree, leaf) = chain(&["div.x"]);
    let cascade = cascade(".x { animation: nope 1s; }");
    let style = cascade
        .compute(&tree.node(leaf), None, &env())
        .expect("computes");
    let spec = animations(&style).into_iter().next().expect("spec");
    let err = cascade.animation(&spec, &style, &env()).unwrap_err();
    assert!(err.message.contains("nope"), "{err}");
}

#[test]
fn missing_stop_values_come_from_the_base_style() {
    // Given: 50% sets opacity only; the ends use the element's own values.
    let src = ".pulse { opacity: 1; width: 10px; animation: blink 100ms linear; } @keyframes blink { 50% { opacity: 0; width: 20px; } }";
    let anim = pulse_animation(src, ColorScheme::Light);
    let s = anim.sample(25.0);
    let opacity = PropertyId::from_name("opacity").expect("opacity");
    assert_eq!(
        s.values
            .iter()
            .find(|(id, _)| *id == opacity)
            .map(|(_, v)| v.to_css())
            .as_deref(),
        Some("0.5")
    );
    let (tree, leaf) = chain(&["div.pulse"]);
    let style = support::computed(src, &tree, leaf);
    assert_eq!(
        css(&style, "width"),
        "10px",
        "the animation never touches the computed style"
    );
}

#[test]
fn unknown_keyframes_error_names_the_declaration_position() {
    let (tree, leaf) = chain(&["div.x"]);
    let cascade = Cascade::new().author(sheet(".x {\n  animation: nope 1s;\n}"));
    let style = cascade
        .compute(&tree.node(leaf), None, &env())
        .expect("computes");
    let spec = vapors_css::animation::animations(&style)
        .into_iter()
        .next()
        .expect("spec");
    let err = cascade.animation(&spec, &style, &env()).unwrap_err();
    assert_eq!((err.position.line, err.position.column), (2, 3), "{err}");
}
