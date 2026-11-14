//! A lane lays each of its children out again once per ask however many
//! of the child's descendants changed (`layout/lane_layout.rs`,
//! `fill_stale`): the child is cleared once per changed descendant, and
//! the lane reads each cleared slot once.

mod support;

use support::{document, element, text, viewport};
use vapors_dom::{Dom, NodeId};

/// Rows of block cells; a `.tall` cell is taller, so its row's answer
/// differs and the row is cleared.
const CSS: &str = ".row { padding: 1px; } .cell { height: 10px; } .tall { height: 30px; }";

/// Layout calls of an update in which the cells named by `[row, cell]`
/// in `tall` grow, on a page of four rows of three cells.
fn calls_with_tall(tall: &[[usize; 2]]) -> usize {
    let mut doc = document();
    let body = doc.body();
    let cells: Vec<Vec<NodeId>> = (0..4)
        .map(|_| {
            let row = element(&mut doc, body, "div", "row");
            (0..3)
                .map(|_| {
                    let cell = element(&mut doc, row, "div", "cell");
                    text(&mut doc, cell, "cell");
                    cell
                })
                .collect()
        })
        .collect();
    doc.add_stylesheet("rows.css", CSS).expect("parses");
    doc.update(viewport(400.0, 300.0)).expect("updates");
    for &[row, cell] in tall {
        doc.set_attribute(cells[row][cell], "class", "cell tall");
    }
    doc.update(viewport(400.0, 300.0)).expect("updates");
    doc.update_counts().layout_calls
}

#[test]
fn a_row_with_two_changed_cells_is_laid_out_again_once_per_ask() {
    // Given: a cell of row 3 grows in every case, so body and html are
    // laid out again in each

    // When: cells 0 and 1 of row 0 grow, together and apart
    let both = calls_with_tall(&[[3, 0], [0, 0], [0, 1]]);
    let first = calls_with_tall(&[[3, 0], [0, 0]]);
    let second = calls_with_tall(&[[3, 0], [0, 1]]);
    let neither = calls_with_tall(&[[3, 0]]);

    // Then: each cell adds its own calls and its row's; with both,
    // row 0's one call is shared
    let shared = first + second - neither - both;
    assert_eq!(shared, 1, "row 0 laid out again per changed cell");
}
