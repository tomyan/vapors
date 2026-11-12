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

    // Then: the elements are put in order again; no attribute is written
    assert_eq!(dom.attribute_writes, 0);
}
