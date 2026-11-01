//! The combobox mounted on the recording Dom with three options, and
//! its keys pressed as the field's keydown listeners hear them.

#![allow(dead_code)] // each test file uses a subset

use std::cell::RefCell;
use std::rc::Rc;

use vapors_components::combobox::{Combobox, ComboboxProps};
use vapors_dom::recording::{Call, RecordingDom};
use vapors_dom::{Dom, Event, NodeId};
use vapors_signals::Signal;

pub struct Mounted {
    pub dom: Rc<RefCell<RecordingDom>>,
    pub combobox: Combobox,
    pub value: Signal<String>,
    pub expanded: Signal<bool>,
    pub active: Signal<Option<usize>>,
}

pub fn mounted() -> Mounted {
    let dom = Rc::new(RefCell::new(RecordingDom::new()));
    let root = dom.borrow().root();
    let (value, expanded, active) = (
        Signal::new(String::new()),
        Signal::new(false),
        Signal::new(None),
    );
    let options = Signal::new(vec![
        "Alabama".to_owned(),
        "Alaska".to_owned(),
        "Arizona".to_owned(),
    ]);
    let props = ComboboxProps {
        options,
        value,
        expanded,
        active,
        id: "cb".into(),
        label: "State".into(),
        list_label: "States".into(),
    };
    let combobox = Combobox::mount(&dom, root, props);
    Mounted {
        dom,
        combobox,
        value,
        expanded,
        active,
    }
}

impl Mounted {
    pub fn input(&self) -> NodeId {
        self.created("input")[0]
    }

    pub fn created(&self, tag: &str) -> Vec<NodeId> {
        self.dom
            .borrow()
            .calls()
            .iter()
            .filter_map(|call| match call {
                Call::CreateElement { node, tag: made } if made == tag => Some(*node),
                _ => None,
            })
            .collect()
    }

    pub fn key(&self, key: &str) -> bool {
        let mut event = Event::new("keydown").with_key(key);
        let input = self.input();
        let along = vapors_dom::Listeners::listeners_along(&*self.dom.borrow(), input, "keydown")
            .expect("the input is in the tree");
        event = event.at(input);
        vapors_dom::run_along(along, &mut event, |_, _| {});
        event.default_prevented()
    }

    pub fn cancel(&self, name: &str) {
        let root = self.dom.borrow().root();
        self.dom
            .borrow_mut()
            .listen(root, name, Box::new(|e: &mut Event| e.prevent_default()));
    }

    pub fn attribute(&self, name: &str) -> Option<String> {
        self.dom.borrow().attribute(self.input(), name)
    }
}
