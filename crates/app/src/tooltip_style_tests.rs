use vapors_elements::{Document, Viewport};
use vapors_scene::Color;
use vapors_text::FontSystem;

use super::TooltipStyle;
use crate::tooltip_error::TooltipError;

/// The root's style after an update with `css`.
fn style_with(css: &str) -> Result<TooltipStyle, TooltipError> {
    let mut doc = Document::new(FontSystem::bundle_only());
    doc.add_stylesheet("app.css", css).expect("parses");
    let viewport = Viewport {
        width: 100.0,
        height: 100.0,
        scale: 1.0,
    };
    doc.update(viewport).expect("updates");
    TooltipStyle::of(&doc)
}

#[test]
fn a_root_with_no_tooltip_properties_takes_the_defaults() {
    // When
    let style = style_with("").expect("read");

    // Then
    assert_eq!(style.background.to_rgba8(), [0x2b, 0x2b, 0x2b, 0xff]);
    assert_eq!(style.color.to_rgba8(), [0xf2, 0xf2, 0xf2, 0xff]);
    assert_eq!(
        (style.font_size, style.padding, style.border_width),
        (12.0, [3.0, 7.0, 4.0, 7.0], 1.0)
    );
}

#[test]
fn the_tooltip_properties_set_on_the_root_are_read() {
    // Given
    let css = ":root { --tooltip-background: #102030; --tooltip-color: rgb(1 2 3); \
        --tooltip-border-color: currentcolor; color: #00ff00; \
        --tooltip-font-size: 15px; --tooltip-padding: 2px 6px; \
        --tooltip-border-width: 0; --tooltip-border-radius: 3px; }";

    // When
    let style = style_with(css).expect("read");

    // Then
    assert_eq!(style.background, Color::from_rgba8(0x10, 0x20, 0x30, 0xff));
    assert_eq!(style.color, Color::from_rgba8(1, 2, 3, 0xff));
    assert_eq!(style.border_color, Color::from_rgba8(0, 0xff, 0, 0xff));
    assert_eq!(style.font_size, 15.0);
    assert_eq!(style.padding, [2.0, 6.0, 2.0, 6.0]);
    assert_eq!((style.border_width, style.border_radius), (0.0, 3.0));
}

#[test]
fn padding_takes_one_to_four_lengths_as_css_padding_does() {
    let cases = [
        ("5px", [5.0; 4]),
        ("1px 2px", [1.0, 2.0, 1.0, 2.0]),
        ("1px 2px 3px", [1.0, 2.0, 3.0, 2.0]),
        ("1px 2px 3px 4px", [1.0, 2.0, 3.0, 4.0]),
    ];
    for (value, padding) in cases {
        // When
        let style = style_with(&format!(":root {{ --tooltip-padding: {value}; }}"));

        // Then
        assert_eq!(style.expect("read").padding, padding, "{value}");
    }
}

#[test]
fn a_value_of_the_wrong_kind_is_refused_naming_the_property_and_value() {
    let cases = [
        ("--tooltip-background", "12px"),
        ("--tooltip-color", "nonsense"),
        ("--tooltip-font-size", "1em"),
        ("--tooltip-font-size", "-2px"),
        ("--tooltip-border-width", "1px 2px"),
        ("--tooltip-padding", "1px 2px 3px 4px 5px"),
        ("--tooltip-border-radius", "red"),
    ];
    for (property, value) in cases {
        // When
        let refused = style_with(&format!(":root {{ {property}: {value}; }}"));

        // Then
        let Err(TooltipError::Property {
            name, value: shown, ..
        }) = refused
        else {
            panic!("{property}: {value} was not refused: {refused:?}");
        };
        assert_eq!((name, shown.as_str()), (property, value));
    }
}

#[test]
fn a_document_not_yet_updated_is_refused() {
    // Given
    let doc = Document::new(FontSystem::bundle_only());

    // Then
    assert!(matches!(
        TooltipStyle::of(&doc),
        Err(TooltipError::NotUpdated)
    ));
}
