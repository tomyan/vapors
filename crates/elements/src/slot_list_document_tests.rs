//! On a page, every per-slot list an update keeps grows by a quarter,
//! not double, when one element is added after the first build (#333's
//! box list, and the lists beside it, `slot_list.rs`): its slack stays
//! within a quarter of its length.

use vapors_dom::{Dom, NodeId};
use vapors_text::FontSystem;

use crate::document::Document;
use crate::update::Viewport;

const VIEWPORT: Viewport = Viewport {
    width: 200.0,
    height: 4000.0,
    scale: 1.0,
};

/// `count` divs under the body, updated once, then one more added in
/// the middle and updated again.
fn page_with_one_added(count: usize) -> Document {
    let mut doc = Document::new(FontSystem::bundle_only());
    let body = doc.body();
    let made: Vec<NodeId> = (0..count)
        .map(|_| {
            let div = doc.create_element("div");
            doc.append(body, div);
            div
        })
        .collect();
    doc.update(VIEWPORT).expect("updates");
    let added = doc.create_element("div");
    doc.insert_before(body, added, Some(made[count / 2]));
    doc.update(VIEWPORT).expect("updates");
    doc
}

/// Every per-slot list the document keeps outside layout (the box list
/// and its keys: `layout/rebuild_capacity_tests.rs`), as its owners
/// name them: name, length, capacity.
fn lists(doc: &Document) -> Vec<(&'static str, usize, usize)> {
    let mut lists = doc.painted.as_ref().expect("painted").slot_lists();
    lists.extend(doc.kept.as_ref().expect("styled").styles.slot_lists());
    lists
}

#[test]
fn every_per_slot_list_grows_by_a_quarter_when_an_element_is_added() {
    // When: 200 divs, then one more
    let doc = page_with_one_added(200);

    // Then: no list holds over a quarter more slots than entries
    let over: Vec<_> = lists(&doc)
        .into_iter()
        .filter(|&(_, len, capacity)| capacity * 4 > len * 5 + 4)
        .collect();
    assert!(
        over.is_empty(),
        "lists with over a quarter's slack: {over:?}"
    );
}

#[test]
fn the_kept_hits_grow_by_a_quarter_when_a_patch_adds_one() {
    // Given: 200 divs, one hidden (no hit), the hits gathered
    let mut doc = Document::new(FontSystem::bundle_only());
    doc.add_stylesheet("t.css", ".gone { visibility: hidden; }")
        .expect("parses");
    let body = doc.body();
    let made: Vec<NodeId> = (0..200)
        .map(|_| {
            let div = doc.create_element("div");
            doc.append(body, div);
            div
        })
        .collect();
    doc.set_attribute(made[100], "class", "gone");
    doc.update(VIEWPORT).expect("updates");
    doc.painted.as_ref().expect("painted").kept_hits();

    // When: shown again, a patch that adds its hit in place
    doc.set_attribute(made[100], "class", "");
    doc.update(VIEWPORT).expect("updates");

    // Then
    let painted = doc.painted.as_ref().expect("painted");
    let hits = painted.kept_hits().slot_lists()[0];
    assert!(hits.2 * 4 <= hits.1 * 5 + 4, "{hits:?}");
}
