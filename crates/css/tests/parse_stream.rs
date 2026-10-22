//! The token stream (`src/parse/stream.rs`): positions of refusals.

mod support;

#[test]
fn a_refusal_at_the_end_of_input_points_past_the_last_token() {
    // Given: review item 17; the brace is missing at the end of input.
    let err = vapors_css::parse(".ab\n").unwrap_err();
    assert_eq!((err.position.line, err.position.column), (2, 1));
}
