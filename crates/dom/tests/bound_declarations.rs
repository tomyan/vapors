//! The declaration a bound value makes, each property's domain and the
//! key shape `bound` must have (decision 2026-11-20-style-bindings).

use vapors_dom::{BoundProperty, BoundValue, Rgba, StyleKey, Winners};

fn declared(property: BoundProperty, px: f32) -> Result<String, String> {
    property.declaration(BoundValue::Length(px))
}

#[test]
fn only_width_height_and_padding_refuse_a_negative() {
    let lengths = BoundProperty::ALL
        .iter()
        .filter(|p| declared(**p, 1.0).is_ok());
    for property in lengths {
        let css = property.css_name();
        let refused = ["width", "height", "padding-"]
            .iter()
            .any(|n| css.starts_with(n));
        match declared(*property, -4.5) {
            Err(why) => assert!(refused && why.contains("(0 or more)"), "{css}: {why}"),
            Ok(text) => assert!(!refused && text == format!("{css}: -4.5px"), "{css}"),
        }
    }
}

#[test]
fn every_length_refuses_nan_and_infinity() {
    for property in BoundProperty::ALL
        .iter()
        .filter(|p| declared(**p, 1.0).is_ok())
    {
        for bad in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            let why = declared(*property, bad).expect_err("refused");
            assert!(why.contains("outside its domain"), "{why}");
        }
    }
}

#[test]
fn a_negative_zero_prints_as_zero_and_a_colour_as_hex_with_alpha() {
    assert_eq!(declared(BoundProperty::Top, -0.0), Ok("top: 0px".into()));
    let color = BoundValue::Color(Rgba {
        r: 255,
        g: 0,
        b: 16,
        a: 128,
    });
    let css = BoundProperty::BackgroundColor.declaration(color);
    assert_eq!(css, Ok("background-color: #ff001080".to_owned()));
}

static TWICE: StyleKey = StyleKey {
    site: "crates/x/bars.wisp:9:3",
    scope: None,
    tag: "div",
    classes: &[],
    toggles: &[],
    attributes: &[],
    style: None,
    bound: &[BoundProperty::Height, BoundProperty::Height],
    winners: Winners::Unanalysed,
};

#[test]
fn a_key_listing_a_property_twice_is_refused_naming_the_site() {
    let why = TWICE.shape_refusal().expect("refused");
    assert!(why.contains("crates/x/bars.wisp:9:3"), "{why}");
    assert!(why.contains("lists `style:height` twice"), "{why}");
    let once = StyleKey {
        bound: &[BoundProperty::Height, BoundProperty::Width],
        ..TWICE
    };
    assert_eq!(once.shape_refusal(), None);
}
