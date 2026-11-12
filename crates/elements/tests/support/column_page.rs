//! The page the restyle-count tests share: html > body > column > 10
//! rows of 5 cells (63 elements), updated once under a sheet, and the
//! elements an update restyles after a change.

use vapors_dom::NodeId;
use vapors_elements::Document;

use super::basics::{document, element, text, viewport};

/// Every element of the page.
pub const ALL: usize = 63;

pub struct Page {
    pub doc: Document,
    pub rows: Vec<NodeId>,
    pub cells: Vec<NodeId>,
    pub texts: Vec<NodeId>,
}

/// The page under `css`, updated once.
pub fn page(css: &str) -> Page {
    let mut doc = document();
    let body = doc.body();
    let column = element(&mut doc, body, "div", "column");
    let (mut rows, mut cells, mut texts) = (Vec::new(), Vec::new(), Vec::new());
    for r in 0..10 {
        let row = element(&mut doc, column, "div", "row");
        rows.push(row);
        for c in 0..5 {
            let cell = element(&mut doc, row, "div", "cell");
            texts.push(text(&mut doc, cell, &format!("{r}.{c}")));
            cells.push(cell);
        }
    }
    doc.add_stylesheet("app.css", css).expect("parses");
    doc.update(viewport(400.0, 600.0)).expect("updates");
    Page {
        doc,
        rows,
        cells,
        texts,
    }
}

/// Updates after `change` and counts the elements restyled.
pub fn restyled_after(page: &mut Page, change: impl FnOnce(&mut Page)) -> usize {
    change(page);
    page.doc.update(viewport(400.0, 600.0)).expect("updates");
    page.doc.elements_restyled()
}
