use vapors_dom::{Dom, NodeId};
use vapors_text::FontSystem;

use super::*;
use crate::document::Document;
use crate::update::Viewport;

const VIEWPORT: Viewport = Viewport {
    width: 200.0,
    height: 100.0,
    scale: 1.0,
};

#[test]
#[should_panic(expected = "slot 0 is vacant, and no box's child")]
fn a_vacant_slots_stacking_is_refused() {
    // Given: one vacant slot
    let stackings = Stackings::of(&[LayoutBox::vacant()], &crate::style::UsedMap::default());

    // When: its stacking is asked for
    let _ = stackings.get(0);

    // Then: it panics, naming the slot
}

/// A document with two `div`s in the body, `.raised` given a z-index,
/// updated once; the first `div`.
fn two_divs() -> (Document, NodeId) {
    let mut doc = Document::new(FontSystem::bundle_only());
    doc.add_stylesheet("app.css", ".raised { position: relative; z-index: 2; }")
        .expect("parses");
    let body = doc.body();
    let first = doc.create_element("div");
    doc.append(body, first);
    let second = doc.create_element("div");
    doc.append(body, second);
    doc.update(VIEWPORT).expect("updates");
    (doc, first)
}

/// Each slot's kept stacking after the last update.
fn kept(doc: &Document) -> Vec<Option<Stacking>> {
    doc.painted().expect("painted").stackings.0.clone()
}

/// The slots whose stacking differs between `before` and `after`, with
/// each one's stacking after.
fn differing(
    before: &[Option<Stacking>],
    after: &[Option<Stacking>],
) -> Vec<(usize, Option<Stacking>)> {
    let at = |stackings: &[Option<Stacking>], slot: usize| stackings.get(slot).copied().flatten();
    (0..before.len().max(after.len()))
        .filter(|&slot| at(before, slot) != at(after, slot))
        .map(|slot| (slot, at(after, slot)))
        .collect()
}

#[test]
fn a_restyle_that_changes_z_index_updates_its_slots_stacking() {
    // Given
    let (mut doc, first) = two_divs();
    let before = kept(&doc);

    // When
    doc.set_attribute(first, "class", "raised");
    doc.update(VIEWPORT).expect("updates");

    // Then: one slot changed, to a positioned box with z-index 2
    let changed = differing(&before, &kept(&doc));
    assert_eq!(changed.len(), 1, "{changed:?}");
    let stacking = changed[0].1.expect("an element's slot holds a stacking");
    assert_eq!((stacking.positioned, stacking.z), (true, Some(2)));
}

#[test]
fn a_freed_slots_stacking_is_gone() {
    // Given
    let (mut doc, first) = two_divs();
    let before = kept(&doc);

    // When
    doc.remove(first);
    doc.update(VIEWPORT).expect("updates");

    // Then: the removed div's slot alone changed, and holds no stacking
    let changed = differing(&before, &kept(&doc));
    assert_eq!(changed.len(), 1, "{changed:?}");
    assert_eq!(changed[0].1, None);
}

#[test]
#[should_panic(expected = "slot 1's kept stacking")]
fn kept_stackings_that_differ_from_fresh_ones_are_refused_naming_the_slot() {
    // Given: kept and fresh stackings that differ at slot 1
    let raised = Stacking {
        z: Some(1),
        ..Stacking::default()
    };
    let kept = Stackings(vec![Some(Stacking::default()), Some(raised)]);
    let fresh = Stackings(vec![Some(Stacking::default()), Some(Stacking::default())]);

    // When
    kept.assert_matches(&fresh);

    // Then: it panics, naming slot 1
}

#[test]
fn a_selector_on_another_element_updates_the_stacking_it_raises() {
    // Given: `a` and `b` in the body; only an element after `.x` is raised
    let mut doc = Document::new(FontSystem::bundle_only());
    doc.add_stylesheet("app.css", ".x + div { position: relative; z-index: 2; }")
        .expect("parses");
    let body = doc.body();
    let a = doc.create_element("div");
    doc.append(body, a);
    let b = doc.create_element("div");
    doc.append(body, b);
    doc.update(VIEWPORT).expect("updates");
    let before = kept(&doc);

    // When: `a` gets the class, which restyles `b` through the sibling selector
    doc.set_attribute(a, "class", "x");
    doc.update(VIEWPORT).expect("updates");

    // Then: one slot changed, to a positioned box with z-index 2
    let changed = differing(&before, &kept(&doc));
    assert_eq!(changed.len(), 1, "{changed:?}");
    let stacking = changed[0].1.expect("an element's slot holds a stacking");
    assert_eq!((stacking.positioned, stacking.z), (true, Some(2)));
}
