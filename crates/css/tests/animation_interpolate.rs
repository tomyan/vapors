//! Interpolation (`src/animation/interpolate.rs`). vapors-specific: Oklab, premultiplied alpha.

mod support;

use support::animation_sample::{px, rgb};
use vapors_css::animation::interpolate;
use vapors_css::value::{Color, Value};
use vapors_scene::Color as Rgba;

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
