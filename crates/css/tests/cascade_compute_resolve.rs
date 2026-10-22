//! Value resolution (`src/cascade/compute_resolve.rs`): calc at compute time.

mod support;

use support::{computed, css};

#[test]
fn calc_without_percent_resolves_to_px() {
    let (tree, leaf) = support::chain(&["div.x"]);
    let style = computed(
        ".x { width: calc(10px + 5px); row-gap: min(3px, 2px); }",
        &tree,
        leaf,
    );
    assert_eq!(css(&style, "width"), "15px");
    assert_eq!(css(&style, "row-gap"), "2px");
}

#[test]
fn calc_with_percent_defers_to_layout() {
    // Given: calc against the containing block stays a calc.
    let (tree, leaf) = support::chain(&["div.x"]);
    let style = computed(".x { width: calc(100% - 10px); }", &tree, leaf);
    assert_eq!(css(&style, "width"), "calc(100% - 10px)");
}

#[test]
fn relative_units_inside_calc_resolve_at_compute_leaving_percent() {
    let (tree, leaf) = support::chain(&["div.x"]);
    let style = computed(
        ".x { font-size: 20px; width: calc(1em + 10%); height: calc(10vw - 1rem); }",
        &tree,
        leaf,
    );
    assert_eq!(css(&style, "width"), "calc(20px + 10%)");
    assert_eq!(css(&style, "height"), "64px");
}
