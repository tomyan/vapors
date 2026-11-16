//! The writing direction cannot change a scroll container's range, as
//! `direction` and `writing-mode` are refused, by name, wherever they are
//! written (4g): a kept range (`scroll_range_kept.rs`) assumes a scroller's
//! axis never flips. When either is supported, its flip becomes a case of
//! `range_changes` there, and the kept range needs the right-to-left
//! overflow Taffy computes.

use crate::support;

use support::{document, element};
use vapors_dom::Dom;
use vapors_elements::Viewport;

const VIEW: Viewport = Viewport {
    width: 400.0,
    height: 600.0,
    scale: 1.0,
};

const PROPERTIES: [&str; 2] = ["direction", "writing-mode"];

/// The refusal of `property` as it appears in a refusal's `Debug` form,
/// whose quotes are escaped.
fn refusal_naming(property: &str) -> String {
    format!(r#"unknown property \"{property}\""#)
}

/// A value of `property` other than its initial one.
fn flipped(property: &str) -> &'static str {
    if property == "direction" {
        "rtl"
    } else {
        "vertical-rl"
    }
}

#[test]
fn a_stylesheet_writing_either_property_is_refused_naming_it() {
    for property in PROPERTIES {
        // Given
        let mut doc = document();

        // When
        let css = format!(".scroller {{ {property}: {}; }}", flipped(property));
        let added = doc.add_stylesheet("flip.css", &css);

        // Then
        let refusal = format!("{added:?}");
        let named = refusal_naming(property);
        assert!(refusal.contains(&named), "{property}: {refusal}");
    }
}

#[test]
fn a_style_attribute_writing_either_property_is_refused_naming_it() {
    for property in PROPERTIES {
        // Given
        let mut doc = document();
        let body = doc.body();
        let scroller = element(&mut doc, body, "div", "scroller");

        // When
        let style = format!("{property}: {}", flipped(property));
        doc.set_attribute(scroller, "style", &style);
        let updated = doc.update(VIEW).map(|_| ());

        // Then
        let refusal = format!("{updated:?}");
        let named = refusal_naming(property);
        assert!(refusal.contains(&named), "{property}: {refusal}");
    }
}
