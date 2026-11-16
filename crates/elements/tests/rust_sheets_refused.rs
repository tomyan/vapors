//! A sheet added from Rust beside an analysed element is refused both
//! ways (build-time-styles.md, decision 2 (a), R1b): the element's
//! compiled winners cannot see the sheet. A hand-written key, without
//! winners, is not analysed.

use vapors_dom::{Dom, KeyWinners, SheetRef, StyleKey};
use vapors_elements::{Document, ElementError, Viewport};
use vapors_text::FontSystem;

static WINNERS: KeyWinners = KeyWinners {
    sheets: &[(SheetRef::UserAgent, 0)],
    base: &[],
    conditional: &[],
    shareable: true,
};

const fn key(winners: Option<&'static KeyWinners>) -> StyleKey {
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

static ANALYSED: StyleKey = key(Some(&WINNERS));
static HAND_WRITTEN: StyleKey = key(None);

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
fn a_hand_written_key_beside_a_rust_sheet_is_not_refused_in_either_order() {
    // Given
    let (mut before, mut after) = (document(), document());

    // When
    before
        .add_stylesheet("page.css", "div { color: red; }")
        .expect("added");
    mounted(&mut before, &HAND_WRITTEN);
    mounted(&mut after, &HAND_WRITTEN);
    let added = after.add_stylesheet("page.css", "div { color: red; }");

    // Then
    assert!(added.is_ok(), "{added:?}");
    assert!(before.update(VIEWPORT).is_ok());
    assert!(after.update(VIEWPORT).is_ok());
}
