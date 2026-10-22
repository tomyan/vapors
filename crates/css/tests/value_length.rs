//! Lengths (`src/value/length.rs`). vapors-specific; sumi had only the cell.

mod support;

use support::value_length::context;
use vapors_css::value::{Length, LengthUnit, parse_length_text};

#[test]
fn every_unit_resolves_against_the_context() {
    for (text, want) in [
        ("10px", 10.0),
        ("2em", 40.0),
        ("2rem", 32.0),
        ("10vw", 100.0),
        ("10vh", 50.0),
        ("2ch", 18.0),
        ("50%", 100.0),
        ("0", 0.0),
    ] {
        let length = parse_length_text(text).unwrap_or_else(|e| panic!("{text}: {e}"));
        assert_eq!(length.to_px(&context()).expect(text), want, "{text}");
    }
}

#[test]
fn parsed_lengths_keep_value_and_unit() {
    assert_eq!(
        parse_length_text("1.5em").expect("parses"),
        Length {
            value: 1.5,
            unit: LengthUnit::Em
        }
    );
}

#[test]
fn unitless_nonzero_and_unknown_units_are_refused() {
    for text in ["10", "10pt", "10cell", "10cm", "auto", "-"] {
        let err = parse_length_text(text).expect_err(text);
        assert!(err.message.contains("px"), "{text}: {err}");
    }
}

#[test]
fn percentage_without_a_base_is_unresolvable() {
    // Given
    let mut ctx = context();
    ctx.percent_base = None;

    // When
    let err = parse_length_text("50%")
        .expect("parses")
        .to_px(&ctx)
        .unwrap_err();

    // Then
    assert!(err.to_string().contains('%'), "{err}");
}
