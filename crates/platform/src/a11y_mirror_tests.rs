//! The mirror applies an update as records, placements, removals; a
//! reconcile keeps the elements it holds; what it cannot apply as
//! given is refused naming the node.

use vapors_a11y::{AxMirrorNode, AxPlace, AxRect, AxStates, AxUpdate, NodeId};

use super::a11y_mirror_dom::Mirror;
use super::a11y_mirror_fake::FakeDom;

pub(crate) fn node(id: u64, parent: Option<u64>, role: &str, name: &str) -> AxMirrorNode {
    AxMirrorNode {
        node: NodeId(id),
        parent: parent.map(NodeId),
        role: role.to_owned(),
        name: name.to_owned(),
        value: None,
        description: None,
        states: AxStates::default(),
        range: None,
        focusable: role == "button",
        bounds: AxRect::default(),
        hoisted: false,
        scroll: None,
    }
}

fn every(nodes: Vec<AxMirrorNode>) -> AxUpdate {
    AxUpdate {
        every: true,
        nodes,
        ..AxUpdate::default()
    }
}

/// A toolbar holding two buttons, and a status after it.
fn page() -> (FakeDom, Mirror<FakeDom>) {
    let mut dom = FakeDom::with_root();
    let mut mirror = Mirror::new(0);
    let update = every(vec![
        node(1, None, "toolbar", "Tools"),
        node(2, Some(1), "button", "Open"),
        node(3, Some(1), "button", "Save"),
        node(4, None, "status", "Ready"),
    ]);
    mirror
        .apply(&mut dom, &update)
        .expect("the first take applies");
    (dom, mirror)
}

#[test]
fn the_first_take_builds_every_node_under_its_parent_in_order() {
    // Given / When
    let (dom, _) = page();

    // Then
    assert_eq!(
        dom.lines(0, 0),
        [
            " \"\"",
            "  aria-label=Tools role=toolbar \"\"",
            "    aria-label=Open role=button tabindex=-1 \"\"",
            "    aria-label=Save role=button tabindex=-1 \"\"",
            "  aria-label=Ready aria-live=off role=status \"\"",
        ]
    );
}

#[test]
fn a_record_changes_the_element_and_removes_the_state_that_went() {
    // Given: Save pressed, then released
    let (mut dom, mut mirror) = page();
    let mut pressed = node(3, Some(1), "button", "Save");
    pressed.states.pressed = Some(true);
    let update = AxUpdate {
        nodes: vec![pressed],
        ..AxUpdate::default()
    };
    mirror.apply(&mut dom, &update).expect("applies");
    assert_eq!(
        dom.lines(3, 0)[0],
        "aria-label=Save aria-pressed=true role=button tabindex=-1 \"\""
    );

    // When
    let update = AxUpdate {
        nodes: vec![node(3, Some(1), "button", "Save")],
        ..AxUpdate::default()
    };
    mirror.apply(&mut dom, &update).expect("applies");

    // Then
    assert_eq!(
        dom.lines(3, 0)[0],
        "aria-label=Save role=button tabindex=-1 \"\""
    );
}

#[test]
fn a_new_node_is_placed_before_its_sibling_and_a_removed_one_goes() {
    // Given
    let (mut dom, mut mirror) = page();

    // When: Close inserted before Save, Open removed
    let update = AxUpdate {
        removed: vec![NodeId(2)],
        nodes: vec![node(5, Some(1), "button", "Close")],
        placed: vec![AxPlace {
            node: NodeId(5),
            parent: Some(NodeId(1)),
            before: Some(NodeId(3)),
        }],
        ..AxUpdate::default()
    };
    mirror.apply(&mut dom, &update).expect("applies");

    // Then
    assert_eq!(
        dom.lines(1, 0),
        [
            "aria-label=Tools role=toolbar \"\"",
            "  aria-label=Close role=button tabindex=-1 \"\"",
            "  aria-label=Save role=button tabindex=-1 \"\"",
        ]
    );
}

#[test]
fn a_reconcile_keeps_the_elements_it_holds_reorders_and_drops_the_gone() {
    // Given: the status gone, the buttons swapped
    let (mut dom, mut mirror) = page();
    let update = every(vec![
        node(1, None, "toolbar", "Tools"),
        node(3, Some(1), "button", "Save"),
        node(2, Some(1), "button", "Open"),
    ]);

    // When
    mirror.apply(&mut dom, &update).expect("applies");

    // Then: elements 1 to 3 are the same indices, 4 is detached
    assert_eq!(dom.elements[0].children, [1]);
    assert_eq!(dom.elements[1].children, [3, 2]);
    assert_eq!(dom.elements[4].parent, None);
    assert_eq!(dom.elements.len(), 5, "no element was made");
}

#[test]
fn a_record_the_update_never_places_is_refused_naming_it() {
    // Given
    let (mut dom, mut mirror) = page();
    let update = AxUpdate {
        nodes: vec![node(9, Some(1), "button", "Lost")],
        ..AxUpdate::default()
    };

    // When
    let refused = mirror.apply(&mut dom, &update).expect_err("refused");

    // Then
    assert!(refused.contains("node(s) 9 without placing"), "{refused}");
}

#[test]
fn a_placement_under_a_node_the_mirror_lacks_is_refused_naming_both() {
    // Given
    let (mut dom, mut mirror) = page();
    let update = AxUpdate {
        nodes: vec![node(9, Some(7), "button", "Orphan")],
        placed: vec![AxPlace {
            node: NodeId(9),
            parent: Some(NodeId(7)),
            before: None,
        }],
        ..AxUpdate::default()
    };

    // When
    let refused = mirror.apply(&mut dom, &update).expect_err("refused");

    // Then
    assert!(refused.contains("node 9 under node 7"), "{refused}");
}

#[test]
fn a_removal_of_a_node_the_mirror_lacks_is_refused_naming_it() {
    // Given
    let (mut dom, mut mirror) = page();
    let update = AxUpdate {
        removed: vec![NodeId(8)],
        ..AxUpdate::default()
    };

    // When
    let refused = mirror.apply(&mut dom, &update).expect_err("refused");

    // Then
    assert!(refused.contains("removes node 8"), "{refused}");
}
