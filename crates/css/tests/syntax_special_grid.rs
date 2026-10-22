//! Grid track lists (`src/syntax/special_grid.rs`).

mod support;

use support::property_values::accepts;
use support::refusal;

#[test]
fn grid_properties_for_taffy() {
    for decl in [
        "grid-template-columns: 1fr 2fr",
        "grid-template-columns: repeat(3, 1fr)",
        "grid-template-columns: minmax(100px, 1fr) auto",
        "grid-template-rows: none",
        "grid-auto-rows: min-content",
        "grid-auto-columns: 40px",
        "grid-row-start: 2",
        "grid-column-end: span 2",
        "grid-row-start: auto",
    ] {
        accepts(decl);
    }
    assert!(
        refusal("a { grid-template-columns: [a] 1fr; }")
            .message
            .contains("line names")
    );
}
