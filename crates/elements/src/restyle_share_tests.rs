//! The oracle: a keyed element whose own data disagrees with its key
//! (forged here by writing the arena directly, as no call can) takes
//! a shared style its own style differs from, and the debug check
//! names the template site.

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
    attributes: &[],
    style: None,
};

fn viewport() -> Viewport {
    Viewport {
        width: 100.0,
        height: 100.0,
        scale: 1.0,
    }
}

/// Two keyed rows under the body, styled once so the share table holds
/// their style.
fn two_rows() -> (Document, [NodeId; 2]) {
    let mut doc = Document::new(FontSystem::bundle_only());
    doc.add_style(
        "vapors-perf/ui/mount_row.wisp",
        ".row { color: rgb(1 2 3); } .other { color: rgb(4 5 6); }",
    );
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
#[should_panic(
    expected = "the share key (restyle_share.rs) misses an input of a selector that reaches <div> at crates/perf/ui/mount_row.wisp:14"
)]
fn a_shared_style_that_differs_from_the_elements_own_names_the_site() {
    // Given: the second row's class forged against its key
    let (mut doc, rows) = two_rows();
    let element = doc
        .arena
        .get_mut(rows[1], "forge")
        .element_mut()
        .expect("an element");
    element
        .attributes
        .insert("class".to_owned(), "other".to_owned());
    doc.mark_changed(rows[1]);

    // When: it is restyled and takes the first row's style
    doc.update(viewport()).expect("updates");
}
