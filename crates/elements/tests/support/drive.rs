//! Shared by the a11y driving tests (R1): the roles page behind an
//! `Rc`, targets by role and name, and a log of events.

use std::cell::RefCell;
use std::rc::Rc;

use vapors_dom::{Dom, Event, NodeId};
use vapors_elements::{AxTarget, Document};

use super::viewport;

pub type Log = Rc<RefCell<Vec<String>>>;

/// `doc` behind an `Rc`, as the input entry points take it.
pub fn shared(doc: Document) -> Rc<RefCell<Document>> {
    Rc::new(RefCell::new(doc))
}

/// The node that is `role` named `name`.
pub fn target(role: &str, name: &str) -> AxTarget {
    AxTarget {
        role: role.to_owned(),
        name: name.to_owned(),
        within: None,
    }
}

/// The node `role` named `name` is, found in the last update.
pub fn node(doc: &Rc<RefCell<Document>>, role: &str, name: &str) -> NodeId {
    doc.borrow().ax_find(role, name, None).expect("found").node
}

/// Updates at the roles page's viewport.
pub fn update(doc: &Rc<RefCell<Document>>) {
    doc.borrow_mut()
        .update(viewport(640.0, 400.0))
        .expect("updates");
}

/// A log of `events` on `node`, by name.
pub fn log(doc: &Rc<RefCell<Document>>, node: NodeId, events: &[&str]) -> Log {
    let log = Log::default();
    for event in events {
        let log = Rc::clone(&log);
        doc.borrow_mut().listen(
            node,
            event,
            Box::new(move |e: &mut Event| log.borrow_mut().push(e.name().to_owned())),
        );
    }
    log
}
