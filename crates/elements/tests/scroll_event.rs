//! A scroll container's `scroll` event (virtual list V1,
//! `docs/plans/virtual-list.md`): `Document::settle_scroll` fires one at
//! each container whose offset changed since its last `scroll`, carrying
//! the offset, by every route that moves it, and none otherwise.

use crate::support;

use std::cell::RefCell;
use std::rc::Rc;

use support::scroll_list::{at, list};
use vapors_dom::{Dom, Event, NodeId};
use vapors_elements::Document;

/// Each `scroll` heard: who heard it (`list` or `body`) and its offset.
type Heard = Vec<(&'static str, Option<[f32; 2]>)>;

/// The `.list` of `support::scroll_list` (110 px to scroll), updated
/// once, with each `scroll` its container and the body hear.
struct Page {
    doc: Rc<RefCell<Document>>,
    container: NodeId,
    rows: Vec<NodeId>,
    heard: Rc<RefCell<Heard>>,
}

fn page() -> Page {
    let mut list = list("overflow: auto;");
    let heard = Rc::new(RefCell::new(Vec::new()));
    let body = list.doc.body();
    for (node, who) in [(list.container, "list"), (body, "body")] {
        let into = Rc::clone(&heard);
        list.doc.listen(
            node,
            "scroll",
            Box::new(move |event: &mut Event| {
                into.borrow_mut().push((who, event.scroll_offset()));
            }),
        );
    }
    list.doc.update(at(1.0)).expect("updates");
    list.doc.take_changed();
    Page {
        doc: Rc::new(RefCell::new(list.doc)),
        container: list.container,
        rows: list.rows,
        heard,
    }
}

impl Page {
    /// The `scroll`s one settle fires.
    fn settle(&self) -> Heard {
        self.heard.borrow_mut().clear();
        Document::settle_scroll(&self.doc);
        self.heard.borrow().clone()
    }

    fn wheel(&self, dy: f32) -> bool {
        self.doc.borrow_mut().scroll_by([10.0, 10.0], [0.0, dy])
    }

    fn scroll_to(&self, y: f32) -> bool {
        let container = self.container;
        self.doc.borrow_mut().scroll_to(container, [0.0, y])
    }

    fn update(&self) {
        self.doc.borrow_mut().update(at(1.0)).expect("updates");
    }
}

#[test]
fn a_wheel_scroll_fires_one_scroll_at_the_container_with_its_offset() {
    // Given
    let page = page();

    // When
    page.wheel(30.0);
    let heard = page.settle();

    // Then: the container's alone; an element's scroll does not bubble
    assert_eq!(heard, [("list", Some([0.0, 30.0]))]);
}

#[test]
fn a_touch_scroll_of_one_container_fires_a_scroll() {
    // Given
    let page = page();

    // When: the route touch and fling take
    let container = page.container;
    page.doc.borrow_mut().scroll_node_by(container, [0.0, 25.0]);
    let heard = page.settle();

    // Then
    assert_eq!(heard, [("list", Some([0.0, 25.0]))]);
}

#[test]
fn a_programmatic_scroll_fires_a_scroll() {
    // Given
    let page = page();

    // When
    page.scroll_to(60.0);
    let heard = page.settle();

    // Then
    assert_eq!(heard, [("list", Some([0.0, 60.0]))]);
}

#[test]
fn scrolls_between_two_settles_fire_one_scroll_with_the_last_offset() {
    // Given
    let page = page();

    // When
    page.wheel(10.0);
    page.wheel(15.0);
    page.scroll_to(70.0);
    let heard = page.settle();

    // Then
    assert_eq!(heard, [("list", Some([0.0, 70.0]))]);
}

#[test]
fn a_settle_with_no_scroll_since_the_last_fires_none() {
    // Given: a scroll already reported
    let page = page();
    page.wheel(30.0);
    page.settle();

    // When
    let heard = page.settle();

    // Then
    assert_eq!(heard, []);
}

#[test]
fn a_scroll_to_the_offset_it_has_fires_none() {
    // Given
    let page = page();
    page.scroll_to(40.0);
    page.settle();

    // When
    let moved = page.scroll_to(40.0);
    let heard = page.settle();

    // Then
    assert_eq!((moved, heard), (false, vec![]));
}

#[test]
fn a_scroll_and_its_reverse_between_settles_fire_none() {
    // Given
    let page = page();

    // When
    page.wheel(30.0);
    page.wheel(-30.0);
    let heard = page.settle();

    // Then: the offset is the one last reported (0, 0)
    assert_eq!(heard, []);
}

#[test]
fn a_scroll_past_the_end_fires_the_clamped_offset() {
    // Given
    let page = page();

    // When
    page.scroll_to(500.0);
    let heard = page.settle();

    // Then: 160 px of rows in a 50 px list
    assert_eq!(heard, [("list", Some([0.0, 110.0]))]);
}

#[test]
fn a_wheel_at_the_end_of_the_range_fires_none() {
    // Given: at the end, reported
    let page = page();
    page.scroll_to(110.0);
    page.settle();

    // When
    let moved = page.wheel(40.0);
    let heard = page.settle();

    // Then
    assert_eq!((moved, heard), (false, vec![]));
}

#[test]
fn an_update_that_clamps_the_offset_owes_a_scroll_until_settled() {
    // Given: at the end, reported, then two of the four rows removed
    let page = page();
    page.scroll_to(110.0);
    page.settle();
    page.update();
    page.doc.borrow_mut().take_changed();
    for row in &page.rows[2..] {
        page.doc.borrow_mut().remove(*row);
    }
    page.doc.borrow_mut().take_changed();

    // When: the update clamps 110 to the 30 px left
    page.update();
    let owed = page.doc.borrow_mut().take_changed();
    let heard = page.settle();
    let still_owed = page.doc.borrow_mut().take_changed();

    // Then
    assert_eq!(
        (owed, heard, still_owed),
        (true, vec![("list", Some([0.0, 30.0]))], false)
    );
}

#[test]
fn an_idle_document_fires_no_scroll_and_reports_no_change() {
    // Given
    let page = page();

    // When
    let heard = page.settle();
    page.update();
    let changed = page.doc.borrow_mut().take_changed();

    // Then
    assert_eq!((heard, changed), (vec![], false));
}

#[test]
fn a_scroll_listener_on_an_element_is_not_refused() {
    // Given: listeners for scroll on the list and the body (`page`)
    let page = page();

    // When
    let updated = page.doc.borrow_mut().update(at(1.0)).map(|_| ());

    // Then
    assert!(updated.is_ok(), "{updated:?}");
}

#[test]
fn an_event_that_is_not_a_scroll_carries_no_offset() {
    // When
    let event = Event::new("click");

    // Then
    assert_eq!(event.scroll_offset(), None);
}
