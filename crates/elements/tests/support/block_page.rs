//! The random test's page with block rows (review v234): no ancestor of
//! an edited row is laid out again there, so the narrowed placement
//! walk (`placement_marks.rs`) is the only route to a changed slot; the
//! flex rows of `changes::page` lay their ancestors out again.

use std::cell::RefCell;
use std::rc::Rc;

use vapors_elements::Document;

use super::changes::Random;
use super::random_css::CSS;
use super::{document, element, text};

/// What the block page adds to the random stylesheet, its flex row rule
/// removed.
const BLOCK_EXTRA: &str = "
.row { display: block; }
.holder { position: relative; }
.deep { padding: 2px; }
";

/// body > 6 block rows, each with up to 3 boxes nested two deep, with a
/// relative holder in two rows and a scroller in the last.
pub fn block_page(random: &mut Random) -> Rc<RefCell<Document>> {
    let mut doc = document();
    let body = doc.body();
    for class in [
        "row",
        "row holder",
        "row",
        "row",
        "row holder",
        "row scroller",
    ] {
        let row = element(&mut doc, body, "div", class);
        for index in 0..random.below(4) {
            let cell = element(&mut doc, row, "div", "box");
            let inner = element(&mut doc, cell, "div", "deep");
            text(&mut doc, inner, &format!("cell {index}"));
        }
    }
    let flex_rows = ".row { display: flex; gap: 4px; }";
    assert!(
        CSS.contains(flex_rows),
        "random_css.rs no longer has {flex_rows}"
    );
    let css = CSS.replace(flex_rows, "") + BLOCK_EXTRA;
    doc.add_stylesheet("random.css", &css).expect("parses");
    Rc::new(RefCell::new(doc))
}
