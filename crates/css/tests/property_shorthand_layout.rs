//! `flex` and grid line shorthands (`src/property/shorthand_layout.rs`).

mod support;

use support::property_shorthand::longhands;

#[test]
fn grid_row_and_column_expand_start_and_end() {
    assert_eq!(
        longhands("grid-row: 1 / 3"),
        ["grid-row-start: 1", "grid-row-end: 3"]
    );
    assert_eq!(
        longhands("grid-column: span 2"),
        ["grid-column-start: span 2", "grid-column-end: auto"]
    );
    assert_eq!(
        longhands("grid-column: 2"),
        ["grid-column-start: 2", "grid-column-end: auto"]
    );
}
