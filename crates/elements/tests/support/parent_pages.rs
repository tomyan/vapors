//! The random test's pages for the parents a rebuilt row is replayed
//! under (M10c, review-v241 L3): the flex-rows page with its body a
//! flex column of rows with vertical margins that wrap, and with its
//! body a two-column grid. A block row answers a size query with its
//! margins, which Taffy's cache keeps no entry for, so these replays
//! go through the row's own record (`layout/asked.rs`).

use std::cell::RefCell;
use std::rc::Rc;

use vapors_elements::Document;

use super::changes::{Random, page};

/// What the column page adds to the random stylesheet.
const COLUMN_EXTRA: &str = "
body { display: flex; flex-direction: column; }
.row { margin: 4px 0; flex-wrap: wrap; }
";

/// What the grid page adds to the random stylesheet.
const GRID_EXTRA: &str = "
body { display: grid; grid-template-columns: auto auto; }
.row { margin: 3px 0; }
";

/// `changes::page` under a flex column body, its rows with vertical
/// margins, wrapping.
pub fn column_page(random: &mut Random) -> Rc<RefCell<Document>> {
    styled(random, "column.css", COLUMN_EXTRA)
}

/// `changes::page` under a two-column grid body, its rows with
/// vertical margins.
pub fn grid_page(random: &mut Random) -> Rc<RefCell<Document>> {
    styled(random, "grid.css", GRID_EXTRA)
}

fn styled(random: &mut Random, name: &str, extra: &str) -> Rc<RefCell<Document>> {
    let doc = page(random);
    doc.borrow_mut()
        .add_stylesheet(name, extra)
        .expect("parses");
    doc
}
