//! A page whose rows are hidden and shown: a `div.row` (`display: flex`)
//! holding a 13px `span.key`, which a flex row blockifies and a block
//! leaves inline, where its font-size differs from its block's and is
//! refused.

use vapors_dom::NodeId;
use vapors_elements::Document;

use super::{document, element, refusals_at, text, viewport};

const CSS: &str = "body { font-size: 16px; } .row { display: flex; } .hidden { display: none; } \
    .key { font-size: 13px; } .pad { padding: 2px; } .round { overflow: hidden; border-radius: 3px; } .odd { --pointer: cell; cursor: var(--pointer); } \
    @media (max-width: 50px) { .narrow { display: none; } }";

/// A document with `css` after the page's sheet.
pub fn page(css: &str) -> Document {
    let mut doc = document();
    doc.add_stylesheet("app.css", &format!("{CSS} {css}"))
        .expect("parses");
    doc
}

/// A `div` of class `class` under `parent` holding a `span.key`.
pub fn row_in(doc: &mut Document, parent: NodeId, class: &str) -> NodeId {
    let row = element(doc, parent, "div", class);
    let key = element(doc, row, "span", "key");
    text(doc, key, "K");
    row
}

/// A page with one row of class `class` in its body.
pub fn row(class: &str) -> (Document, NodeId) {
    let mut doc = page("");
    let body = doc.body();
    let row = row_in(&mut doc, body, class);
    (doc, row)
}

/// Asserts `doc` updates at `width` by 100 px.
pub fn assert_accepted_at(doc: &mut Document, width: f32) {
    let updated = doc.update(viewport(width, 100.0));
    assert!(updated.is_ok(), "{:?}", updated.err());
}

pub fn assert_accepted(doc: &mut Document) {
    assert_accepted_at(doc, 100.0);
}

/// Asserts `doc` is refused at 100 by 100 px with a message holding `says`.
pub fn assert_refused(doc: &mut Document, says: &str) {
    let found = refusals_at(doc, viewport(100.0, 100.0));
    assert!(
        found.iter().any(|r| r.contains(says)),
        "{says} in {found:?}"
    );
}
