//! Press-and-hold behavior on the recording DOM; browser tests cover capture routing.
use std::{cell::RefCell, rc::Rc};
use vapors_components::hold_button::{HoldButton, HoldButtonProps};
use vapors_dom::{
    Capture, Event, Listeners, NodeId, Pointer,
    recording::{Call, RecordingDom},
};
use vapors_signals::Signal;
struct Mounted {
    dom: Rc<RefCell<RecordingDom>>,
    button: HoldButton,
    node: NodeId,
    pressed: Signal<bool>,
    enabled: Signal<bool>,
    reset: Signal<u64>,
}
impl Mounted {
    fn new() -> Self {
        let dom = Rc::new(RefCell::new(RecordingDom::new()));
        let root = dom.borrow().root();
        let pressed = Signal::new(false);
        let enabled = Signal::new(true);
        let reset = Signal::new(0);
        let button = HoldButton::mount(
            &dom,
            root,
            HoldButtonProps {
                id: "test".into(),
                label: "Hold".into(),
                pressed,
                enabled,
                reset,
            },
        );
        let node = dom
            .borrow()
            .calls()
            .iter()
            .find_map(|c| match c {
                Call::CreateElement { node, tag } if tag == "button" => Some(*node),
                _ => None,
            })
            .unwrap();
        Self {
            dom,
            button,
            node,
            pressed,
            enabled,
            reset,
        }
    }
    fn send(&self, mut event: Event) -> Event {
        let along =
            Listeners::listeners_along(&*self.dom.borrow(), self.node, event.name()).unwrap();
        event = event.at(self.node);
        vapors_dom::run_along(along, &mut event, |_, _| {});
        event
    }
    fn pointer(&self, name: &str, id: u32) -> Event {
        self.send(Event::new(name).with_pointer(Pointer {
            id,
            pressed: name == "pointerdown",
            ..Pointer::default()
        }))
    }
}
#[test]
fn pointers_remain_held_until_each_has_ended() {
    // Given
    let m = Mounted::new();
    // When
    let mut down = m.pointer("pointerdown", 10);
    m.pointer("pointerdown", 11);
    // Then
    assert_eq!(down.take_capture(), Some(Capture::Set));
    assert!(m.pressed.get());
    m.pointer("pointerup", 10);
    assert!(m.pressed.get());
    m.pointer("pointerup", 11);
    assert!(!m.pressed.get());
}
#[test]
fn cancellation_and_capture_loss_release_the_button() {
    // Given / When / Then
    for ending in ["pointercancel", "lostpointercapture"] {
        let m = Mounted::new();
        m.pointer("pointerdown", 10);
        assert!(m.pressed.get());
        m.pointer(ending, 10);
        assert!(!m.pressed.get());
    }
}
#[test]
fn reset_and_disable_clear_holds_without_reviving_them() {
    // Given
    let m = Mounted::new();
    m.pointer("pointerdown", 10);
    // When / Then
    m.reset.set(1);
    assert!(!m.pressed.get());
    m.pointer("pointerdown", 11);
    assert!(m.pressed.get());
    m.enabled.set(false);
    assert!(!m.pressed.get());
    m.pointer("pointerdown", 12);
    assert!(!m.pressed.get());
    m.enabled.set(true);
    assert!(!m.pressed.get());
}
#[test]
fn keyboard_activation_and_blur_release_the_button() {
    // Given
    let m = Mounted::new();
    // When / Then
    for key in [" ", "Enter"] {
        m.send(Event::new("keydown").with_key(key));
        assert!(m.pressed.get());
        m.send(Event::new("keyup").with_key(key));
        assert!(!m.pressed.get());
    }
    m.send(Event::new("keydown").with_key(" "));
    m.send(Event::new("blur"));
    assert!(!m.pressed.get());
}
#[test]
fn unmount_releases_an_external_pressed_signal() {
    // Given
    let m = Mounted::new();
    m.pointer("pointerdown", 10);
    assert!(m.pressed.get());
    // When
    m.button.unmount(&m.dom);
    // Then
    assert!(!m.pressed.get());
}

#[test]
fn unchanged_enabled_and_reset_values_do_not_release_a_hold() {
    // Given
    let m = Mounted::new();
    m.pointer("pointerdown", 10);
    // When
    m.enabled.set(true);
    m.reset.set(0);
    // Then
    assert!(m.pressed.get());
}
