use taffy::{AlignContent, AlignItems, Display, FlexDirection, FlexWrap, Style};

use super::centre_run;

/// An author's inner layout on an input: a wrapping reversed row of a
/// grid's alignment, which Chromium 157 ignores (the probe's author
/// cases, `tests/field_text_centre.rs`).
fn authored(display: Display) -> Style {
    Style {
        display,
        flex_direction: FlexDirection::RowReverse,
        flex_wrap: FlexWrap::Wrap,
        justify_content: AlignContent::END,
        align_items: AlignItems::FLEX_START,
        ..Style::default()
    }
}

#[test]
fn a_fields_own_column_replaces_every_authored_inner_layout() {
    for display in [Display::Block, Display::Flex, Display::Grid] {
        // Given
        let mut style = authored(display);

        // When
        centre_run(&mut style);

        // Then: a one-line column, its run centred and as wide as it
        assert_eq!(style.display, Display::Flex, "{display:?}");
        assert_eq!(style.flex_direction, FlexDirection::Column, "{display:?}");
        assert_eq!(style.flex_wrap, FlexWrap::NoWrap, "{display:?}");
        assert_eq!(style.justify_content, AlignContent::CENTER, "{display:?}");
        assert_eq!(style.align_items, AlignItems::STRETCH, "{display:?}");
    }
}

#[test]
fn a_field_with_display_none_keeps_it() {
    // Given
    let mut style = authored(Display::None);

    // When
    centre_run(&mut style);

    // Then
    assert_eq!(style, authored(Display::None));
}
