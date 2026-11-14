//! Pages for the lane-frame tests (`docs/plans/splice-layout.md`,
//! section 11): a header above a list of rows, and drawing a page held
//! against the whole scene's batches.

use std::cell::RefCell;
use std::rc::Rc;

use vapors_dom::{Dom, NodeId};
use vapors_elements::{Document, UpdateCounts, Viewport};

use super::narrow_list::row;
use super::oracle::assert_matches_full;
use super::{document, element};

/// A header above a list, under body, and the list's rows.
pub struct HeaderList {
    pub doc: Rc<RefCell<Document>>,
    pub head: NodeId,
    pub list: NodeId,
    pub rows: Vec<NodeId>,
}

/// `div.head` then `div.list` of `count` rows under body, styled by
/// `css`, updated once at `view`.
pub fn header_list(count: usize, css: &str, view: Viewport) -> HeaderList {
    let mut doc = document();
    let body = doc.body();
    let head = element(&mut doc, body, "div", "head");
    let list = element(&mut doc, body, "div", "list");
    let rows = (0..count)
        .map(|index| {
            let node = row(&mut doc, &format!("row {index}"));
            doc.append(list, node);
            node
        })
        .collect();
    doc.add_stylesheet("page.css", css).expect("parses");
    doc.update(view).expect("updates");
    HeaderList {
        doc: Rc::new(RefCell::new(doc)),
        head,
        list,
        rows,
    }
}

/// Updates `doc` at `view` and asserts the draw gets the batches of the
/// whole scene in its frames, naming `case`.
pub fn assert_drawn_afresh(doc: &Rc<RefCell<Document>>, view: Viewport, case: &str) {
    let mut doc = doc.borrow_mut();
    let painted = doc.update(view).expect("updates");
    let rect = vapors_scene::Rect::new(0.0, 0.0, view.width * view.scale, view.height * view.scale);
    let kept = painted.kept_batches(rect).clone();
    let fresh = painted.scene().framed_batches(rect);
    assert_eq!((kept.batched, kept.frames), fresh, "{case}");
}

impl HeaderList {
    /// Inserts a new row before row `at`; the new row.
    pub fn insert_at(&self, at: usize) -> NodeId {
        let mut doc = self.doc.borrow_mut();
        let node = row(&mut doc, "new");
        doc.insert_before(self.list, node, Some(self.rows[at]));
        node
    }

    /// The counts of the update at `view` after `edit`, which matches a
    /// full update.
    pub fn edited(&self, view: Viewport, edit: impl FnOnce(&Self)) -> UpdateCounts {
        edit(self);
        let counts = {
            let mut doc = self.doc.borrow_mut();
            doc.update(view).expect("updates");
            doc.update_counts()
        };
        assert_matches_full(&self.doc, view, "after the edit");
        counts
    }
}
