//! Keyed elements share one computed style per share key (analysis PR
//! 2): the counters say what each update computed, shared, styled
//! unkeyed or could not share, and what differs in the key (a toggled
//! class, a state, an attribute a sheet selects) ends the sharing.

mod support;

use support::{document, viewport};
use vapors_dom::{Dom, NodeId, StyleKey};
use vapors_elements::{Document, UpdateCounts};

const SCOPE: &str = "vapors-perf/ui/mount_row.wisp";

static ROW: StyleKey = StyleKey {
    site: "crates/perf/ui/mount_row.wisp:14",
    scope: Some(SCOPE),
    tag: "div",
    classes: &["row"],
    toggles: &["marked"],
    attributes: &[],
    style: None,
};

static BUTTON: StyleKey = StyleKey {
    site: "crates/perf/ui/mount_row.wisp:14",
    scope: Some(SCOPE),
    tag: "button",
    classes: &[],
    toggles: &[],
    attributes: &[],
    style: None,
};

/// A document with `css` under the rows' scope and `count` keyed
/// elements of `key` under the body.
fn page(css: &str, key: &'static StyleKey, count: usize) -> (Document, Vec<NodeId>) {
    let mut doc = document();
    doc.add_style(SCOPE, css);
    let body = doc.body();
    let nodes = (0..count)
        .map(|_| {
            let node = doc.create_keyed_element(key);
            doc.append(body, node);
            node
        })
        .collect();
    (doc, nodes)
}

fn update(doc: &mut Document) -> UpdateCounts {
    doc.update(viewport(200.0, 100.0)).expect("updates");
    doc.update_counts()
}

/// (shared, computed, unkeyed, unshareable, live) of the last update.
fn styles(counts: &UpdateCounts) -> (usize, usize, usize, usize, usize) {
    (
        counts.styles_shared,
        counts.styles_computed,
        counts.styles_unkeyed,
        counts.styles_unshareable,
        counts.styles_live,
    )
}

fn color(doc: &Document, node: NodeId) -> String {
    doc.computed_style(node)
        .expect("styled")
        .get_by_name("color")
        .expect("a longhand")
        .to_css()
}

#[test]
fn rows_with_one_key_compute_one_style_and_share_it() {
    // Given
    let (mut doc, _) = page(".row { color: rgb(1 2 3); }", &ROW, 3);

    // When
    let counts = update(&mut doc);

    // Then: html and body are unkeyed; one row computes, two share
    assert_eq!(counts.elements_restyled, 5);
    assert_eq!(styles(&counts), (2, 1, 2, 0, 3));
}

#[test]
fn a_toggled_class_is_a_new_share_key_and_the_old_style_stays_for_the_others() {
    // Given
    let (mut doc, rows) = page(
        ".row { color: rgb(1 2 3); } .row.marked { color: rgb(4 5 6); }",
        &ROW,
        3,
    );
    update(&mut doc);

    // When
    doc.toggle_classes(rows[1], &[("marked", true)]);
    let counts = update(&mut doc);

    // Then
    assert_eq!(counts.elements_restyled, 1);
    assert_eq!(styles(&counts), (0, 1, 0, 0, 4));
    assert_eq!(color(&doc, rows[1]), "rgb(4 5 6)");
    assert_eq!(color(&doc, rows[0]), "rgb(1 2 3)");
}

#[test]
fn a_style_no_element_holds_is_dropped() {
    // Given: row 1 marked, then unmarked again
    let (mut doc, rows) = page(".row.marked { color: rgb(4 5 6); }", &ROW, 2);
    update(&mut doc);
    doc.toggle_classes(rows[1], &[("marked", true)]);
    assert_eq!(update(&mut doc).styles_live, 4);

    // When
    doc.toggle_classes(rows[1], &[("marked", false)]);
    let counts = update(&mut doc);

    // Then: it shares the unmarked style again and the marked one goes
    assert_eq!(styles(&counts), (1, 0, 0, 0, 3));
}

#[test]
fn a_key_whose_rules_read_siblings_does_not_share() {
    // Given
    let (mut doc, _) = page(".row + .row { color: rgb(1 2 3); }", &ROW, 3);

    // When
    let counts = update(&mut doc);

    // Then
    assert_eq!(styles(&counts), (0, 0, 2, 3, 5));
}

#[test]
fn an_attribute_a_sheet_selects_is_part_of_the_share_key() {
    // Given: ua.css has `[hidden] { display: none }`
    let (mut doc, rows) = page(".row { color: rgb(1 2 3); }", &ROW, 3);
    update(&mut doc);

    // When
    doc.set_attribute(rows[2], "hidden", "");
    let counts = update(&mut doc);

    // Then
    assert_eq!(styles(&counts), (0, 1, 0, 0, 4));
    let display = |node| {
        doc.computed_style(node)
            .expect("styled")
            .get_by_name("display")
            .expect("a longhand")
            .to_css()
    };
    assert_eq!(display(rows[2]), "none");
    assert_ne!(display(rows[1]), "none");
}

#[test]
fn an_attribute_no_sheet_selects_keeps_the_share() {
    // Given
    let (mut doc, rows) = page(".row { color: rgb(1 2 3); }", &ROW, 3);
    update(&mut doc);

    // When
    doc.set_attribute(rows[2], "title", "a row");
    let counts = update(&mut doc);

    // Then
    assert_eq!(styles(&counts), (1, 0, 0, 0, 3));
}

#[test]
fn a_state_is_part_of_the_share_key() {
    // Given: two keyed buttons, one disabled
    let (mut doc, buttons) = page(
        "button { display: block; color: rgb(1 2 3); } button:disabled { color: rgb(4 5 6); }",
        &BUTTON,
        2,
    );
    doc.set_attribute(buttons[1], "disabled", "");

    // When
    let counts = update(&mut doc);

    // Then
    assert_eq!(styles(&counts), (0, 2, 2, 0, 4));
    assert_eq!(color(&doc, buttons[0]), "rgb(1 2 3)");
    assert_eq!(color(&doc, buttons[1]), "rgb(4 5 6)");
}

#[test]
fn a_removed_elements_style_is_dropped() {
    // Given
    let (mut doc, rows) = page(".row { color: rgb(1 2 3); }", &ROW, 2);
    update(&mut doc);
    doc.toggle_classes(rows[1], &[("marked", true)]);
    assert_eq!(update(&mut doc).styles_live, 4);

    // When
    doc.remove(rows[1]);
    let counts = update(&mut doc);

    // Then
    assert_eq!(counts.styles_live, 3);
}

#[test]
fn a_failed_restyle_keeps_no_style_counts() {
    // Given: every row's colour reads a variable nothing sets
    let (mut doc, _) = page(".row { color: var(--missing); }", &ROW, 3);

    // When
    let refused = doc.update(viewport(200.0, 100.0)).is_err();

    // Then
    assert!(refused);
    assert_eq!(styles(&doc.update_counts()), (0, 0, 0, 0, 0));
}
