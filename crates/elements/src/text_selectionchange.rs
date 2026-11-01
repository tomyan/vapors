//! A text field's `selectionchange` (E6a): fired once per settle for
//! each field whose selection moved since the last one, as Chromium
//! queues one per task.

use std::cell::RefCell;
use std::rc::Rc;

use vapors_dom::{Event, NodeId};

use crate::document::Document;
use crate::events::tree::dispatch_bubbling;

impl Document {
    /// Fires the `selectionchange` each text field owes whose selection
    /// moved since the last one, as Chromium fires one per task: the
    /// app calls this before each update, the vapors player after each event.
    pub fn settle_selection(document: &Rc<RefCell<Document>>) {
        let owed: Vec<NodeId> = document
            .borrow_mut()
            .editors
            .iter_mut()
            .filter_map(|(node, editor)| std::mem::take(&mut editor.pending).then_some(*node))
            .collect();
        for node in owed {
            if document.borrow_mut().selection_moved(node) {
                dispatch_bubbling(document, node, Event::new("selectionchange"));
            }
        }
    }
}

impl Document {
    /// Whether `node`'s selection moved since the last `selectionchange`,
    /// noting it as reported: not for a field a listener removed, or one
    /// that is no longer a text field.
    fn selection_moved(&mut self, node: NodeId) -> bool {
        if !self.contains(node) {
            return false;
        }
        let Some(now) = self.selection(node) else {
            return false;
        };
        let Some(editor) = self.editors.get_mut(&node) else {
            return false;
        };
        let moved = editor.reported != Some(now);
        editor.reported = Some(now);
        moved
    }
}
