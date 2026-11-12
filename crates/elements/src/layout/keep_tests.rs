use taffy::{Dimension, Size, Style};

use super::*;

const HIDDEN_ASKED: crate::layout::isolated::Asked = crate::layout::isolated::Asked {
    input: taffy::LayoutInput::HIDDEN,
    in_block: false,
};

/// One element box whose width is a calc handle at `address`.
fn with_calc(address: usize) -> Vec<LayoutBox> {
    let style = Style {
        size: Size {
            width: Dimension::calc(address as *const ()),
            height: Dimension::auto(),
        },
        ..Style::default()
    };
    vec![LayoutBox::new(Content::Element(NodeId(7)), style)]
}

#[test]
fn lending_to_an_update_that_built_no_box_lends_nothing() {
    // Given: kept boxes; html became display: none, so no box
    let mut kept = KeptBoxes::default();
    kept.keep(crate::layout::slots::tests::churned(), 1.0);

    // When
    let lent = kept.lend(&mut Placed::default(), 1.0, &|_| true);

    // Then
    assert_eq!(lent, 0);
}

#[test]
fn the_kept_box_of_a_node_is_its_element_box_and_a_removed_node_has_none() {
    // Given: boxes after three updates, with text runs; nodes 10, 11
    // and 13 were removed
    let mut kept = KeptBoxes::default();

    // When
    kept.keep(crate::layout::slots::tests::churned(), 1.0);

    // Then
    for node in [1, 12, 14].map(NodeId) {
        let content = kept.element(node).map(|b| &b.content);
        assert!(content == Some(&Content::Element(node)), "{node:?}");
    }
    for node in [10, 11, 13].map(NodeId) {
        assert!(kept.element(node).is_none(), "{node:?}");
    }
}

#[test]
fn a_restyled_box_holding_a_calc_is_lent_nothing_even_when_the_address_matches() {
    // Given: the last update's box and this one's share a handle
    // address, as a freed and reused allocation would make them
    let mut kept = KeptBoxes::default();
    kept.keep(Slots::default().place(with_calc(0x1000)), 1.0);
    let mut placed = kept.place(with_calc(0x1000));

    // When: the element was restyled, its calc may have changed
    let lent = kept.lend(&mut placed, 1.0, &|_| false);

    // Then
    assert_eq!(lent, 0);
}

#[test]
fn an_unrestyled_box_holding_a_calc_is_lent_its_cache() {
    // Given
    let mut kept = KeptBoxes::default();
    kept.keep(Slots::default().place(with_calc(0x1000)), 1.0);
    let mut placed = kept.place(with_calc(0x1000));

    // When
    let lent = kept.lend(&mut placed, 1.0, &|_| true);

    // Then
    assert_eq!(lent, 1);
}

#[test]
fn a_lent_box_takes_the_inputs_the_kept_box_was_asked_with_its_cache() {
    // Given: the kept box was asked once
    let mut kept = KeptBoxes::default();
    let mut boxes = with_calc(0x1000);
    boxes[0].asked.push(HIDDEN_ASKED);
    kept.keep(Slots::default().place(boxes), 1.0);
    let mut placed = kept.place(with_calc(0x1000));

    // When
    kept.lend(&mut placed, 1.0, &|_| true);

    // Then
    assert_eq!(placed.boxes[0].asked, [HIDDEN_ASKED]);
}
