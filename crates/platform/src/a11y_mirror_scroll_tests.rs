//! A scroll is one write (R6 PR 5a): a scroll container's element
//! scrolls, its rows' bounds being from it, and the mirror writes only
//! what a record changed, so a scroll of a 3000-row list writes the
//! container's offset and nothing else.

use vapors_a11y::{AxRect, AxScroll, AxUpdate};

use super::a11y_mirror_dom::Mirror;
use super::a11y_mirror_fake::FakeDom;
use super::a11y_mirror_tests::node;

const ROWS: u64 = 3000;
const ROW_HEIGHT: f32 = 24.0;

/// A body that scrolls `ROWS` rows of buttons, mirrored: the DOM and
/// the mirror, the writes of the first take not counted.
fn scrolling_list() -> (FakeDom, Mirror<FakeDom>) {
    let mut dom = FakeDom::with_root();
    let mut mirror = Mirror::new(0);
    let nodes = body_and_rows();
    let update = AxUpdate {
        every: true,
        nodes,
        ..AxUpdate::default()
    };
    mirror.apply(&mut dom, &update).expect("the first take");
    dom.attribute_writes = 0;
    dom.moves = 0;
    (dom, mirror)
}

/// The body, unscrolled, and its `ROWS` rows of buttons, each from the
/// body's padding box.
fn body_and_rows() -> Vec<vapors_a11y::AxMirrorNode> {
    let mut nodes = vec![body_at(0.0)];
    for row in 0..ROWS {
        let mut button = node(2 + row, Some(1), "button", &format!("Row {row}"));
        #[allow(clippy::cast_precision_loss)] // 3000 rows
        let y = row as f32 * ROW_HEIGHT;
        button.bounds = AxRect {
            x: 0.0,
            y,
            width: 640.0,
            height: ROW_HEIGHT,
        };
        nodes.push(button);
    }
    nodes
}

/// The body as a scroll container of the rows, scrolled to `y`.
fn body_at(y: f32) -> vapors_a11y::AxMirrorNode {
    let mut body = node(1, None, "none", "");
    body.bounds = AxRect {
        x: 0.0,
        y: 0.0,
        width: 640.0,
        height: 400.0,
    };
    #[allow(clippy::cast_precision_loss)] // 3000 rows
    let extent = [640.0, ROWS as f32 * ROW_HEIGHT];
    body.scroll = Some(AxScroll {
        offset: [0.0, y],
        extent,
    });
    body
}

#[test]
fn a_scroll_of_a_3000_row_list_writes_the_containers_offset_and_nothing_else() {
    // Given
    let (mut dom, mut mirror) = scrolling_list();

    // When: the take after a scroll by one row
    let update = AxUpdate {
        nodes: vec![body_at(ROW_HEIGHT)],
        ..AxUpdate::default()
    };
    mirror.apply(&mut dom, &update).expect("applies");

    // Then
    assert_eq!((dom.attribute_writes, dom.moves), (1, 0));
    assert_eq!(dom.elements[1].scroll, [0.0, ROW_HEIGHT]);
}

#[test]
fn a_scroll_container_scrolls_its_element_over_its_extent() {
    // Given / When
    let (dom, _) = scrolling_list();

    // Then: a scrolling element, sized to its content by the page's
    // rule for `data-scroll`
    let body = &dom.elements[1];
    assert_eq!(
        body.attributes.get("data-scroll").map(String::as_str),
        Some("")
    );
    assert_eq!(
        body.attributes.get("style").map(String::as_str),
        Some(
            "position:absolute;left:0px;top:0px;width:640px;height:400px;overflow:auto;\
             --vapors-scroll-w:640px;--vapors-scroll-h:72000px"
        )
    );
    assert_eq!(body.scroll, [0.0, 0.0]);
}

#[test]
fn a_record_writes_only_the_attributes_that_changed() {
    // Given
    let (mut dom, mut mirror) = scrolling_list();

    // When: one row renamed
    let mut renamed = node(2, Some(1), "button", "Row one");
    renamed.bounds = AxRect {
        x: 0.0,
        y: 0.0,
        width: 640.0,
        height: ROW_HEIGHT,
    };
    let update = AxUpdate {
        nodes: vec![renamed],
        ..AxUpdate::default()
    };
    mirror.apply(&mut dom, &update).expect("applies");

    // Then: aria-label alone
    assert_eq!((dom.attribute_writes, dom.moves), (1, 0));
    assert_eq!(
        dom.elements[2]
            .attributes
            .get("aria-label")
            .map(String::as_str),
        Some("Row one")
    );
}

#[test]
fn a_reconcile_of_the_same_records_writes_no_attribute() {
    // Given
    let (mut dom, mut mirror) = scrolling_list();
    let nodes = body_and_rows();

    // When: every node again, as a viewport change takes it
    let update = AxUpdate {
        every: true,
        nodes,
        ..AxUpdate::default()
    };
    mirror.apply(&mut dom, &update).expect("applies");

    // Then: nothing is written or moved (review-v243 H2: a move resets
    // the element's scroll and drops focus on the page)
    assert_eq!((dom.attribute_writes, dom.moves), (0, 0));
}

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

    // When: the toolbar goes after the status
    let update = AxUpdate {
        every: true,
        nodes: vec![
            node(3, None, "status", "Ready"),
            node(1, None, "toolbar", "Tools"),
            list,
        ],
        ..AxUpdate::default()
    };
    mirror.apply(&mut dom, &update).expect("applies");

    // Then: the toolbar moved (one move), and the list in it is
    // scrolled again
    assert_eq!(dom.elements[0].children, [3, 1]);
    assert_eq!(dom.elements[2].scroll, [0.0, ROW_HEIGHT]);
}
