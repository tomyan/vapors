//! A page of rows for the in-crate tests of what an edit costs.

use vapors_dom::{Dom, NodeId};
use vapors_text::FontSystem;

use crate::document::Document;
use crate::update::Viewport;

/// Enough rows that a set sized for all of them is far larger than one
/// sized for an edit.
pub(crate) const ROWS: usize = 1000;

pub(crate) fn viewport() -> Viewport {
    Viewport {
        width: 100.0,
        height: 100.0,
        scale: 1.0,
    }
}

/// `ROWS` rows under the body, each with a click listener when
/// `listened`, mounted by one update.
pub(crate) fn mounted(listened: bool) -> (Document, Vec<NodeId>) {
    let mut doc = Document::new(FontSystem::bundle_only());
    doc.add_style("rows", ".on { color: rgb(1 2 3); }");
    let body = doc.body();
    let rows = (0..ROWS).map(|_| row(&mut doc, body, listened)).collect();
    doc.update(viewport()).expect("mounts");
    (doc, rows)
}

fn row(doc: &mut Document, body: NodeId, listened: bool) -> NodeId {
    let row = doc.create_element("div");
    if listened {
        doc.listen(row, "click", Box::new(|_| {}));
    }
    doc.append(body, row);
    row
}
