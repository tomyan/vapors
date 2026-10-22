//! Interpolation and the pure `sample(t)` API. Keyframe stops resolve in
//! the element's scope as in sumi's `resolve_keyframes_test.go`; the
//! rest is vapors-specific (sumi interpolated colours in sRGB and lengths
//! in whole cells).

mod support;

use support::{cascade, chain, css, env, sheet};
use vapors_css::animation::{
    Animation, TimingFunction, Transition, TransitionProperty, TransitionSpec, animations,
    interpolate,
};
use vapors_css::cascade::Cascade;
use vapors_css::property::PropertyId;
use vapors_css::value::{Color, ColorScheme, Length, Value};
use vapors_scene::Color as Rgba;

fn px(v: f64) -> Value {
    Value::Length(Length::px(v))
}

fn rgb(r: u8, g: u8, b: u8) -> Value {
    Value::Color(Color::Rgba(Rgba::from_rgba8(r, g, b, 255)))
}

#[test]
fn lengths_numbers_and_percentages_interpolate_linearly() {
    assert_eq!(interpolate(&px(10.0), &px(20.0), 0.25), Some(px(12.5)));
    assert_eq!(
        interpolate(&Value::Number(0.0), &Value::Number(1.0), 0.5),
        Some(Value::Number(0.5))
    );
    assert_eq!(
        interpolate(&Value::Integer(0), &Value::Integer(10), 0.55),
        Some(Value::Integer(6))
    );
    let half = interpolate(&Value::Percentage(0.0), &Value::Percentage(50.0), 0.5);
    assert_eq!(half, Some(Value::Percentage(25.0)));
}

#[test]
fn colours_interpolate_in_oklab() {
    // Given: the midpoint of red and blue in Oklab is a purple with
    // lightness between the two, not the dark sRGB midpoint (128 0 128).
    let mid = interpolate(&rgb(255, 0, 0), &rgb(0, 0, 255), 0.5).expect("colour");
    let Value::Color(Color::Rgba(mid_color)) = mid else {
        panic!("{mid:?}")
    };
    let [r, g, b, a] = mid_color.to_rgba8();
    assert!(
        r > 100 && b > 100 && g < r && g < b && a == 255,
        "{r} {g} {b} {a}"
    );
    let [l0, ..] = Color::to_oklab(Rgba::from_rgba8(255, 0, 0, 255));
    let [l1, ..] = Color::to_oklab(Rgba::from_rgba8(0, 0, 255, 255));
    let [lm, ..] = Color::to_oklab(mid_color);
    assert!(
        (lm - f64::midpoint(l0, l1)).abs() < 0.01,
        "{lm} vs {l0} {l1}"
    );
}

#[test]
fn alpha_interpolates_premultiplied() {
    let from = Value::Color(Color::Rgba(Rgba::from_rgba8(255, 0, 0, 0)));
    let to = rgb(255, 0, 0);
    let Some(Value::Color(Color::Rgba(half))) = interpolate(&from, &to, 0.5) else {
        panic!()
    };
    assert_eq!(half.to_rgba8(), [255, 0, 0, 128]);
}

#[test]
fn lists_interpolate_elementwise_and_mismatches_snap() {
    let a = Value::List(vec![px(0.0), px(10.0)]);
    let b = Value::List(vec![px(10.0), px(30.0)]);
    assert_eq!(
        interpolate(&a, &b, 0.5),
        Some(Value::List(vec![px(5.0), px(20.0)]))
    );
    let k = Value::Keyword("block".to_owned());
    let n = Value::Keyword("none".to_owned());
    assert_eq!(interpolate(&k, &n, 0.49), Some(k.clone()));
    assert_eq!(interpolate(&k, &n, 0.5), Some(n));
    assert_eq!(
        interpolate(&px(1.0), &Value::Keyword("auto".to_owned()), 0.5),
        None,
        "no common shape"
    );
}

#[test]
fn transition_sample_follows_delay_duration_and_easing() {
    // Given
    let spec = TransitionSpec {
        property: TransitionProperty::Property(PropertyId::from_name("width").expect("width")),
        duration_ms: 100.0,
        timing: TimingFunction::LINEAR,
        delay_ms: 50.0,
    };
    let t = Transition::new(spec, px(0.0), px(100.0));

    // Then
    assert_eq!(t.sample(0.0), (px(0.0), false));
    assert_eq!(t.sample(50.0), (px(0.0), false));
    assert_eq!(t.sample(100.0), (px(50.0), false));
    assert_eq!(t.sample(150.0), (px(100.0), true));
    assert_eq!(t.sample(999.0), (px(100.0), true));
}

#[test]
fn transition_retargets_from_the_current_value() {
    let spec = TransitionSpec {
        property: TransitionProperty::All,
        duration_ms: 100.0,
        timing: TimingFunction::LINEAR,
        delay_ms: 0.0,
    };
    let t = Transition::new(spec.clone(), px(0.0), px(100.0));
    let (current, _) = t.sample(40.0);
    let retargeted = Transition::new(spec, current, px(0.0));
    assert_eq!(retargeted.sample(20.0), (px(32.0), false));
}

fn pulse_animation(src: &str, scheme: ColorScheme) -> Animation {
    let (tree, leaf) = chain(&["div.pulse"]);
    let cascade = Cascade::new().author(sheet(src));
    let mut e = env();
    e.color_scheme = scheme;
    let styles = cascade
        .compute_tree(&tree.node(tree.root()), &e)
        .expect("computes");
    let style = &styles.iter().find(|(n, _)| n.id() == leaf).expect("leaf").1;
    let spec = animations(style).into_iter().next().expect("one animation");
    cascade
        .animation(&spec, style, &e)
        .unwrap_or_else(|err| panic!("{err}"))
}

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
fn no_animation_leaves_the_list_empty() {
    let (tree, leaf) = chain(&["div"]);
    let style = support::computed("div { color: red; }", &tree, leaf);
    assert_eq!(animations(&style), []);
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
fn animation_sample_iterations_direction_and_fill() {
    // Given: 0 to 100px over 100ms, 2 iterations, alternate, delay 50, fill both.
    let src = ".pulse { width: 7px; animation: grow 100ms linear 50ms 2 alternate both; } @keyframes grow { from { width: 0px; } to { width: 100px; } }";
    let anim = pulse_animation(src, ColorScheme::Light);
    let width = PropertyId::from_name("width").expect("width");
    let at = |ms: f64| {
        let s = anim.sample(ms);
        (
            s.values
                .into_iter()
                .find(|(id, _)| *id == width)
                .map(|(_, v)| v),
            s.finished,
        )
    };

    // Then
    assert_eq!(
        at(0.0),
        (Some(px(0.0)), false),
        "backwards fill during the delay"
    );
    assert_eq!(at(75.0), (Some(px(25.0)), false));
    assert_eq!(at(150.0), (Some(px(100.0)), false), "end of iteration 1");
    assert_eq!(
        at(200.0),
        (Some(px(50.0)), false),
        "iteration 2 runs backwards"
    );
    assert_eq!(
        at(250.0),
        (Some(px(0.0)), true),
        "forwards fill holds the last value"
    );
    assert_eq!(at(9999.0), (Some(px(0.0)), true));
}

#[test]
fn animation_without_fill_holds_nothing_outside_its_interval() {
    let src = ".pulse { animation: grow 100ms linear; } @keyframes grow { from { width: 0px; } to { width: 100px; } }";
    let anim = pulse_animation(src, ColorScheme::Light);
    assert_eq!(anim.sample(150.0).values, []);
    assert!(anim.sample(150.0).finished);
    assert_ne!(anim.sample(50.0).values, []);
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
fn keyframe_timing_applies_per_segment() {
    let src = ".pulse { animation: s 100ms steps(1, end); } @keyframes s { from { width: 0px; } 50% { width: 50px; } to { width: 100px; } }";
    let anim = pulse_animation(src, ColorScheme::Light);
    let width = PropertyId::from_name("width").expect("width");
    let at = |ms: f64| {
        anim.sample(ms)
            .values
            .into_iter()
            .find(|(id, _)| *id == width)
            .map(|(_, v)| v)
    };
    assert_eq!(at(25.0), Some(px(0.0)));
    assert_eq!(at(75.0), Some(px(50.0)));
}
