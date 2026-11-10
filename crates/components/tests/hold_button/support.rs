//! A `HoldButton` mounted on the recording DOM, its output written into a
//! consumer's signal.
use std::{cell::RefCell, rc::Rc};
use vapors_components::hold_button::{HoldButton, HoldButtonProps};
use vapors_dom::{
    Event, Listeners, NodeId, Pointer,
    recording::{Call, RecordingDom},
};
use vapors_signals::Signal;

pub struct Mounted {
    pub dom: Rc<RefCell<RecordingDom>>,
    pub button: HoldButton,
    pub node: NodeId,
    pub pressed: Signal<bool>,
    pub enabled: Signal<bool>,
    pub reset: Signal<u64>,
}

impl Mounted {
    pub fn new() -> Self {
        let dom = Rc::new(RefCell::new(RecordingDom::new()));
        Self::beside(&dom, "test")
    }

    /// Another button on `dom`, with its own consumer signal.
    pub fn beside(dom: &Rc<RefCell<RecordingDom>>, id: &str) -> Self {
        let pressed = Signal::new(false);
        Self::mount(dom, id, pressed, vapors_components::hold_into(pressed))
    }

    /// A button whose changes go to `on_change`; `pressed` is never written.
    pub fn reporting_to(on_change: Rc<dyn Fn(bool)>) -> Self {
        let dom = Rc::new(RefCell::new(RecordingDom::new()));
        Self::mount(&dom, "test", Signal::new(false), on_change)
    }

    fn mount(
        dom: &Rc<RefCell<RecordingDom>>,
        id: &str,
        pressed: Signal<bool>,
        on_change: Rc<dyn Fn(bool)>,
    ) -> Self {
        let (enabled, reset) = (Signal::new(true), Signal::new(0));
        let before = dom.borrow().calls().len();
        let root = dom.borrow().root();
        let props = HoldButtonProps {
            id: id.into(),
            label: "Hold".into(),
            on_change,
            enabled,
            reset,
        };
        let button = HoldButton::mount(dom, root, props);
        let node = button_node(&dom.borrow().calls()[before..]);
        let dom = Rc::clone(dom);
        Self {
            dom,
            button,
            node,
            pressed,
            enabled,
            reset,
        }
    }

    pub fn send(&self, mut event: Event) -> Event {
        let along =
            Listeners::listeners_along(&*self.dom.borrow(), self.node, event.name()).unwrap();
        event = event.at(self.node);
        vapors_dom::run_along(along, &mut event, |_, _| {});
        event
    }

    pub fn pointer(&self, name: &str, id: u32) -> Event {
        self.send(Event::new(name).with_pointer(Pointer {
            id,
            pressed: name == "pointerdown",
            ..Pointer::default()
        }))
    }
}

fn button_node(calls: &[Call]) -> NodeId {
    calls
        .iter()
        .find_map(|c| match c {
            Call::CreateElement { node, tag } if tag == "button" => Some(*node),
            _ => None,
        })
        .expect("HoldButton creates a button element")
}
