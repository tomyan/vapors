//! The declaration a bound value makes, each property's domain and the
//! key shape `bound` must have (decision 2026-11-20-style-bindings).

use vapors_dom::BoundProperty as P;
use vapors_dom::{BoundProperty, BoundValue, Rgba, StyleKey, Winners};

fn declared(property: BoundProperty, px: f32) -> Result<String, String> {
    property.declaration(BoundValue::Length(px))
}

/// Every length property, written out, and those refusing a negative.
const LENGTHS: [P; 12] = [
    P::Width,
    P::Height,
    P::Top,
    P::Left,
    P::MarginTop,
    P::MarginRight,
    P::MarginBottom,
    P::MarginLeft,
    P::PaddingTop,
    P::PaddingRight,
    P::PaddingBottom,
    P::PaddingLeft,
];
const NON_NEGATIVE: [P; 6] = [
    P::Width,
    P::Height,
    P::PaddingTop,
    P::PaddingRight,
    P::PaddingBottom,
    P::PaddingLeft,
];

#[test]
fn the_length_and_colour_lists_cover_every_bindable_property() {
    let mut all: Vec<P> = LENGTHS.to_vec();
    all.extend([P::BackgroundColor, P::Color]);
    all.sort();
    assert_eq!(all, BoundProperty::ALL);
}

#[test]
fn only_width_height_and_padding_refuse_a_negative() {
    for property in LENGTHS {
        let css = property.css_name();
        match declared(property, -4.5) {
            Err(why) => assert!(
                NON_NEGATIVE.contains(&property) && why.contains("(0 or more)"),
                "{css}: {why}"
            ),
            Ok(text) => assert!(
                !NON_NEGATIVE.contains(&property) && text == format!("{css}: -4.5px"),
                "{css}"
            ),
        }
    }
}

#[test]
fn every_length_refuses_nan_and_infinity() {
    for property in LENGTHS {
        for bad in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            let why = declared(property, bad).expect_err("refused");
            assert!(why.contains("outside its domain"), "{why}");
        }
    }
}

#[test]
fn a_wrong_kind_names_what_the_property_takes_and_what_it_got() {
    let why = P::Height.declaration(BoundValue::Color(Rgba::rgb(1, 2, 3)));
    let why = why.expect_err("refused");
    assert!(why.contains("takes a px f32, got an Rgba"), "{why}");
    let why = P::Color.declaration(BoundValue::Length(1.0));
    assert!(
        why.expect_err("refused")
            .contains("takes an Rgba, got a px f32")
    );
}

#[test]
fn a_negative_zero_prints_as_zero_and_a_colour_as_hex_with_alpha() {
    assert_eq!(declared(P::Top, -0.0), Ok("top: 0px".into()));
    let color = BoundValue::Color(Rgba {
        r: 255,
        g: 0,
        b: 16,
        a: 128,
    });
    let css = P::BackgroundColor.declaration(color);
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
    bound: &[P::Height, P::Height],
    winners: Winners::Unanalysed,
};

#[test]
fn a_key_listing_a_property_twice_is_refused_naming_the_site() {
    let why = TWICE.shape_refusal().expect("refused");
    assert!(why.contains("crates/x/bars.wisp:9:3"), "{why}");
    assert!(why.contains("lists `style:height` twice"), "{why}");
    let once = StyleKey {
        bound: &[P::Height, P::Width],
        ..TWICE
    };
    assert_eq!(once.shape_refusal(), None);
}

#[test]
fn an_unlisted_property_is_refused_naming_the_property_in_the_fix() {
    let why = TWICE
        .bound_declaration(P::Width, BoundValue::Length(1.0))
        .expect_err("refused");
    assert!(why.contains("write style:width={expr}"), "{why}");
}

#[test]
fn every_property_is_found_by_its_css_name_and_nothing_else_is() {
    for property in BoundProperty::ALL {
        assert_eq!(
            BoundProperty::from_css(property.css_name()),
            Some(*property)
        );
    }
    for other in [
        "",
        "Height",
        "border-radius",
        "margin",
        "style:height",
        "opacity",
    ] {
        assert_eq!(BoundProperty::from_css(other), None, "{other}");
    }
}

#[test]
fn the_kind_says_what_a_property_takes() {
    use vapors_dom::BoundKind;
    for property in LENGTHS {
        assert_eq!(property.kind(), BoundKind::Length);
    }
    assert_eq!(P::Color.kind(), BoundKind::Color);
    assert_eq!(P::BackgroundColor.kind(), BoundKind::Color);
}
