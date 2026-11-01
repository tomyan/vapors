//! The tree the dispatcher tests dispatch in.

use std::cell::RefCell;
use std::rc::Rc;

use vapors_dom::recording::RecordingDom;
use vapors_dom::{Dom, Event};

/// The tree, the list, the option in it, and the log of who heard.
pub type Tree = (
    Rc<RefCell<RecordingDom>>,
    vapors_dom::NodeId,
    vapors_dom::NodeId,
    Rc<RefCell<Vec<String>>>,
);

/// A list holding one option, and a log of who heard what.
pub fn tree() -> Tree {
    let dom = Rc::new(RefCell::new(RecordingDom::new()));
    let log = Rc::new(RefCell::new(Vec::new()));
    let (list, option) = {
        let mut d = dom.borrow_mut();
        let list = d.create_element("div");
        let option = d.create_element("div");
        d.append(list, option);
        (list, option)
    };
    for (node, who) in [(option, "option"), (list, "list")] {
        let into = Rc::clone(&log);
        dom.borrow_mut().listen(
            node,
            "optionchoose",
            Box::new(move |_: &mut Event| into.borrow_mut().push(who.to_owned())),
        );
    }
    (dom, list, option, log)
}
