//! The worked counter: `src/counter.wisp`, compiled by build.rs and
//! mounted on the recording Dom. A click increments the count and
//! patches exactly one text node; nothing else touches the tree.

use std::cell::RefCell;
use std::rc::Rc;

use vapors_compiler_examples::counter::{Counter, CounterProps};
use vapors_dom::NodeId;
use vapors_dom::recording::{Call, RecordingDom};

fn mounted() -> (Rc<RefCell<RecordingDom>>, Counter, NodeId) {
    let dom = Rc::new(RefCell::new(RecordingDom::new()));
    let root = dom.borrow().root();
    let counter = Counter::mount(&dom, root, CounterProps::default());
    let button = counter.roots()[0];
    dom.borrow_mut().clear_calls();
    (dom, counter, button)
}

/// The text node that shows the count: the button's second child.
fn count_text(dom: &Rc<RefCell<RecordingDom>>, button: NodeId) -> NodeId {
    dom.borrow().children(button)[1]
}

#[test]
fn mounting_renders_the_button_with_a_zero_count() {
    // When
    let (dom, _counter, _button) = mounted();

    // Then
    assert_eq!(dom.borrow().to_html(), "<button>Clicked 0 times</button>");
}

#[test]
fn a_click_increments_the_count_signal() {
    // Given
    let (dom, counter, button) = mounted();

    // When
    RecordingDom::click(&dom, button);

    // Then
    assert_eq!(counter.count.get(), 1);
    assert_eq!(dom.borrow().to_html(), "<button>Clicked 1 times</button>");
}

#[test]
fn each_click_sets_the_count_text_once_and_makes_no_other_call() {
    // Given
    let (dom, _counter, button) = mounted();
    let text = count_text(&dom, button);

    for expected in ["1", "2", "3"] {
        // When
        RecordingDom::click(&dom, button);

        // Then
        let calls = dom.borrow().calls().to_vec();
        assert_eq!(
            calls,
            vec![Call::SetText {
                node: text,
                text: expected.to_string()
            }]
        );
        dom.borrow_mut().clear_calls();
    }
}

#[test]
fn unmounting_removes_the_button_and_stops_updates() {
    // Given
    let (dom, counter, button) = mounted();
    let count = counter.count;

    // When
    counter.unmount(&dom);

    // Then
    assert_eq!(dom.borrow().to_html(), "");
    assert_eq!(dom.borrow().calls(), &[Call::Remove { node: button }]);
    assert!(
        count.try_get().is_err(),
        "the signal is disposed with the component"
    );
}
