//! Keyed rows with one computed style share one used style; a row
//! whose place differs (forged here by writing the arena, as no call
//! can) derives its own; and the oracle: a used style the table holds
//! that differs from the one an element derives (forged in the table)
//! names the template site and the field.

use std::rc::Rc;

use vapors_dom::{Dom, NodeId, StyleKey};
use vapors_text::FontSystem;

use crate::document::Document;
use crate::update::Viewport;

static ROW: StyleKey = StyleKey {
    site: "crates/perf/ui/mount_row.wisp:14",
    scope: Some("vapors-perf/ui/mount_row.wisp"),
    tag: "div",
    classes: &["row"],
    toggles: &[],
    style: None,
};

fn viewport() -> Viewport {
    Viewport {
        width: 100.0,
        height: 100.0,
        scale: 1.0,
    }
}

/// Two keyed rows under the body, styled once so the tables hold their
/// styles.
fn two_rows() -> (Document, [NodeId; 2]) {
    rows_styled(".row { color: rgb(1 2 3); }")
}

/// The same, with `css` under the rows' scope.
fn rows_styled(css: &str) -> (Document, [NodeId; 2]) {
    let mut doc = Document::new(FontSystem::bundle_only());
    doc.add_style("vapors-perf/ui/mount_row.wisp", css);
    let body = doc.body();
    let rows = [(); 2].map(|()| {
        let row = doc.create_keyed_element(&ROW);
        doc.append(body, row);
        row
    });
    doc.update(viewport()).expect("updates");
    (doc, rows)
}

#[test]
fn rows_with_one_computed_style_hold_one_used_style() {
    // When
    let (doc, rows) = two_rows();

    // Then
    let used = &doc.kept.as_ref().expect("updated").used;
    assert!(Rc::ptr_eq(&used[&rows[0]], &used[&rows[1]]));
}

#[test]
fn a_row_whose_place_differs_derives_its_own_used_style() {
    // Given: padded rows, the second forged into a gpu-surface, whose
    // computed style is a div's (ua.css) and which refuses padding
    let (mut doc, rows) = rows_styled(".row { padding: 4px; }");
    let element = doc.arena.get_mut(rows[1], "forge").element_mut();
    element.expect("an element").tag = "gpu-surface".to_owned();
    doc.mark_changed(rows[1]);

    // When
    let Err(errors) = doc.update(viewport()) else {
        panic!("the surface's padding was not refused");
    };

    // Then: the surface bit of the key kept it from the row's used style
    let errors: Vec<String> = errors.iter().map(ToString::to_string).collect();
    assert!(
        errors.iter().any(|e| e.contains("pads a gpu-surface")),
        "{errors:?}"
    );
}

#[test]
#[should_panic(
    expected = "differs from its own: field `visible`; the used-style key (style/used_share.rs) misses an input of the derivation for <div> at crates/perf/ui/mount_row.wisp:14"
)]
fn a_shared_used_style_that_differs_from_the_elements_own_names_the_site_and_field() {
    // Given: the table's used style forged hidden
    let (mut doc, rows) = two_rows();
    let kept = doc.kept.as_mut().expect("updated");
    kept.shared.used.forge(|used| used.visible = false);
    doc.mark_changed(rows[0]);

    // When: the row is restyled and takes the forged used style
    doc.update(viewport()).expect("updates");
}
