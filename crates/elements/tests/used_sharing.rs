//! Keyed elements that share a computed style share one used style
//! (build-time styles U2) where their parents' used styles and places
//! are equal: the counters say what each update took from the
//! used-style table and what it derived, and a refused element's
//! refusals still name it.

mod support;

use support::{document, refusals, viewport};
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

/// A document with `css` under the rows' scope and `count` keyed rows
/// under the body, each with the id `r<i>`, which no sheet selects.
fn page(css: &str, count: usize) -> (Document, Vec<NodeId>) {
    let mut doc = document();
    doc.add_style(SCOPE, css);
    let body = doc.body();
    let rows = (0..count)
        .map(|i| {
            let row = doc.create_keyed_element(&ROW);
            doc.set_attribute(row, "id", &format!("r{i}"));
            doc.append(body, row);
            row
        })
        .collect();
    (doc, rows)
}

/// (shared, derived) used styles of the last update.
fn used(doc: &mut Document) -> (usize, usize) {
    doc.update(viewport(200.0, 100.0)).expect("updates");
    let counts: UpdateCounts = doc.update_counts();
    (counts.used_shared, counts.used_derived)
}

#[test]
fn rows_with_one_computed_style_derive_one_used_style_and_share_it() {
    // Given
    let (mut doc, _) = page(".row { color: rgb(1 2 3); }", 3);

    // When
    let counts = used(&mut doc);

    // Then: html, body and the first row derive; two rows share
    assert_eq!(counts, (2, 3));
}

#[test]
fn rows_whose_key_cannot_share_derive_each_used_style() {
    // Given
    let (mut doc, _) = page(".row + .row { color: rgb(1 2 3); }", 3);

    // When
    let counts = used(&mut doc);

    // Then
    assert_eq!(counts, (0, 5));
}

#[test]
fn a_toggled_class_derives_once_and_shares_the_kept_used_style_when_untoggled() {
    // Given
    let (mut doc, rows) = page(".row.marked { color: rgb(4 5 6); }", 3);
    used(&mut doc);
    doc.toggle_classes(rows[1], &[("marked", true)]);
    assert_eq!(used(&mut doc), (0, 1));

    // When
    doc.toggle_classes(rows[1], &[("marked", false)]);
    let counts = used(&mut doc);

    // Then: the other rows still hold the unmarked used style
    assert_eq!(counts, (1, 0));
}

#[test]
fn rows_sharing_a_refused_computed_style_are_each_refused_by_name() {
    // Given
    let (mut doc, _) = page(".row { position: sticky; top: 0; }", 2);

    // When
    let errors = refusals(&mut doc);

    // Then
    for id in ["r0", "r1"] {
        let named = format!("id=\"{id}\"");
        assert!(
            errors
                .iter()
                .any(|e| e.contains(&named) && e.contains("sticky")),
            "{id}: {errors:?}"
        );
    }
}
