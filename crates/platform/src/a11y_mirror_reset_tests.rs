//! Chromium resets a scroll container's offset when its element is
//! appended again and ignores a scroll set on a detached element
//! (review-v243 H2): the reconcile moves only the elements out of
//! place, and offsets are written once elements are in place, so a
//! viewport change keeps every mirror scroller where it was.

use vapors_a11y::AxUpdate;

use super::a11y_mirror_dom::Mirror;
use super::a11y_mirror_fake::FakeDom;
use super::a11y_mirror_scroll_tests::{ROW_HEIGHT, body_and_rows, body_at, scrolling_list};
use super::a11y_mirror_tests::node;

#[test]
fn a_reconcile_keeps_a_scrolled_containers_offset() {
    // Given: scrolled by one row
    let (mut dom, mut mirror) = scrolling_list();
    let scrolled = AxUpdate {
        nodes: vec![body_at(ROW_HEIGHT)],
        ..AxUpdate::default()
    };
    mirror.apply(&mut dom, &scrolled).expect("applies");
    let mut nodes = body_and_rows();
    nodes[0] = body_at(ROW_HEIGHT);

    // When: every node again, as a viewport change takes it
    let update = AxUpdate {
        every: true,
        nodes,
        ..AxUpdate::default()
    };
    mirror.apply(&mut dom, &update).expect("applies");

    // Then
    assert_eq!(dom.elements[1].scroll, [0.0, ROW_HEIGHT]);
}

#[test]
fn a_new_scroll_container_is_scrolled_once_it_is_in_place() {
    // Given: a mirror with nothing yet
    let mut dom = FakeDom::with_root();
    let mut mirror = Mirror::new(0);
    let mut nodes = body_and_rows();
    nodes[0] = body_at(ROW_HEIGHT);

    // When: the first take holds the body scrolled already
    let update = AxUpdate {
        every: true,
        nodes,
        ..AxUpdate::default()
    };
    mirror.apply(&mut dom, &update).expect("applies");

    // Then: the offset was set after the element was attached (a
    // scroll set on a detached element is lost)
    assert_eq!(dom.elements[1].scroll, [0.0, ROW_HEIGHT]);
}

#[test]
fn a_scroll_container_moved_by_a_reconcile_is_scrolled_again() {
    // Given: scrolled by one row
    let (mut dom, mut mirror) = scrolling_list();
    let scrolled = AxUpdate {
        nodes: vec![body_at(ROW_HEIGHT)],
        ..AxUpdate::default()
    };
    mirror.apply(&mut dom, &scrolled).expect("applies");

    // When: a region arrives holding the body, so the body's element
    // moves under it (its page scroll resets)
    let mut nodes = vec![node(9000, None, "region", "Page")];
    nodes.extend(body_and_rows());
    nodes[1] = body_at(ROW_HEIGHT);
    nodes[1].parent = Some(vapors_a11y::NodeId(9000));
    let update = AxUpdate {
        every: true,
        nodes,
        ..AxUpdate::default()
    };
    mirror.apply(&mut dom, &update).expect("applies");

    // Then: the body is under the region, scrolled again
    let region = dom.elements.len() - 1;
    assert_eq!(dom.elements[region].children, [1]);
    assert_eq!(dom.elements[1].scroll, [0.0, ROW_HEIGHT]);
}

#[test]
fn a_scroll_container_under_a_moved_node_is_scrolled_again() {
    // Given: a toolbar holding a scrolling list, scrolled, with a
    // status after the toolbar
    let mut dom = FakeDom::with_root();
    let mut mirror = Mirror::new(0);
    let mut list = body_at(ROW_HEIGHT);
    list.node = vapors_a11y::NodeId(2);
    list.parent = Some(vapors_a11y::NodeId(1));
    let first = AxUpdate {
        every: true,
        nodes: vec![
            node(1, None, "toolbar", "Tools"),
            list.clone(),
            node(3, None, "status", "Ready"),
        ],
        ..AxUpdate::default()
    };
    mirror.apply(&mut dom, &first).expect("applies");
    assert_eq!(dom.elements[2].scroll, [0.0, ROW_HEIGHT]);

    // When: a region arrives holding the toolbar
    let mut toolbar = node(1, None, "toolbar", "Tools");
    toolbar.parent = Some(vapors_a11y::NodeId(4));
    let update = AxUpdate {
        every: true,
        nodes: vec![
            node(4, None, "region", "Page"),
            toolbar,
            list,
            node(3, None, "status", "Ready"),
        ],
        ..AxUpdate::default()
    };
    mirror.apply(&mut dom, &update).expect("applies");

    // Then: the toolbar moved under the region, and the list in it is
    // scrolled again
    assert_eq!(dom.elements[4].children, [1]);
    assert_eq!(dom.elements[2].scroll, [0.0, ROW_HEIGHT]);
}
