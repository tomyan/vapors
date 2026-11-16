//! A sheet added from Rust beside an analysed element is refused both
//! ways (build-time-styles.md, decision 2 (a), R1b): the element's
//! compiled winners cannot see the sheet. A hand-written key, without
//! winners, is not analysed.

use vapors_dom::{Dom, KeyWinners, StyleKey, Winners};
use vapors_elements::{Document, ElementError, Viewport};
use vapors_text::FontSystem;

// referencing no sheet, every rule is ranked at run time, so the debug
// winners oracle (style::winners_oracle) has no base winner to hold
static WINNERS: KeyWinners = KeyWinners {
    sheets: &[],
    base: &[],
    conditional: &[],
    shareable: true,
    own: None,
};

const fn key(winners: Winners) -> StyleKey {
    StyleKey {
        site: "cards/card.wisp:3:1",
        scope: None,
        tag: "div",
        classes: &[],
        toggles: &[],
        attributes: &[],
        style: None,
        winners,
    }
}

static ANALYSED: StyleKey = key(Winners::Compiled(&WINNERS));
static HAND_WRITTEN: StyleKey = key(Winners::Unanalysed);
static COMPONENT_TAG: StyleKey = key(Winners::ComponentTag);

const VIEWPORT: Viewport = Viewport {
    width: 200.0,
    height: 100.0,
    scale: 1.0,
};

fn document() -> Document {
    Document::new(FontSystem::bundle_only())
}

fn mounted(doc: &mut Document, key: &'static StyleKey) {
    let body = doc.body();
    let node = doc.create_keyed_element(key);
    doc.insert_before(body, node, None);
}

#[test]
fn a_rust_sheet_after_an_analysed_element_is_refused_naming_the_element_and_the_fix() {
    // Given
    let mut doc = document();
    mounted(&mut doc, &ANALYSED);

    // When
    let refused = doc.add_stylesheet("page.css", "div { color: red; }");

    // Then
    let Err(ElementError::Refused(refusal)) = refused else {
        panic!("not refused: {refused:?}");
    };
    assert!(
        refusal.contains("add_stylesheet(\"page.css\")"),
        "{refusal}"
    );
    assert!(refusal.contains("cards/card.wisp:3:1"), "{refusal}");
    assert!(
        refusal.contains("fix: put the rules in a component's <style>"),
        "{refusal}"
    );
}

#[test]
fn an_analysed_element_in_a_document_holding_a_rust_sheet_is_refused_naming_the_sheet() {
    // Given
    let mut doc = document();
    doc.add_stylesheet("page.css", "div { color: red; }")
        .expect("added");

    // When
    mounted(&mut doc, &ANALYSED);
    let refused = format!("{:?}", doc.update(VIEWPORT).err().expect("refused"));

    // Then
    assert!(refused.contains("cards/card.wisp:3:1"), "{refused}");
    assert!(
        refused.contains("\\\"page.css\\\", a sheet added from Rust"),
        "{refused}"
    );
    assert!(
        refused.contains("fix: put that sheet's rules in a component's <style>"),
        "{refused}"
    );
}

#[test]
fn a_key_without_compiled_winners_beside_a_rust_sheet_is_not_refused_in_either_order() {
    // a hand-written key, and a component tag's, whose tag its props type
    // decides: neither is analysed
    for key in [&HAND_WRITTEN, &COMPONENT_TAG] {
        // Given
        let (mut before, mut after) = (document(), document());

        // When
        before
            .add_stylesheet("page.css", "div { color: red; }")
            .expect("added");
        mounted(&mut before, key);
        mounted(&mut after, key);
        let added = after.add_stylesheet("page.css", "div { color: red; }");

        // Then
        assert!(added.is_ok(), "{:?}: {added:?}", key.winners);
        assert!(before.update(VIEWPORT).is_ok(), "{:?}", key.winners);
        assert!(after.update(VIEWPORT).is_ok(), "{:?}", key.winners);
    }
}

#[test]
fn a_refused_rust_sheet_changes_nothing() {
    // Given an analysed element, updated
    let mut doc = document();
    mounted(&mut doc, &ANALYSED);
    if let Some(errors) = doc.update(VIEWPORT).err() {
        panic!("{errors:?}");
    }
    doc.take_changed();

    // When
    let refused = doc.add_stylesheet("page.css", "div { color: red; }");

    // Then: nothing to draw again
    assert!(refused.is_err());
    assert!(!doc.take_changed());
}
