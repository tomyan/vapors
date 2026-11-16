//! Check 1 of compiled winners (build-time-styles.md, section 2): a
//! sheet a key's winners index, when registered, must hash as the
//! compiler hashed it.

use vapors_css::sheet_hash::sheet_hash;
use vapors_dom::{Dom, KeyWinners, SheetRef, StyleKey};
use vapors_elements::Document;
use vapors_text::FontSystem;

// with the newlines a `<style>` section's text has
const CARD: &str = "\n.card { color: red; }\n";

const fn key(winners: &'static KeyWinners) -> StyleKey {
    StyleKey {
        site: "cards/card.wisp:3:1",
        scope: Some("cards/card.wisp"),
        tag: "div",
        classes: &["card"],
        toggles: &[],
        attributes: &[],
        style: None,
        winners: Some(winners),
    }
}

fn document_with_card() -> Document {
    let mut doc = Document::new(FontSystem::bundle_only());
    doc.add_style("cards/card.wisp", CARD);
    doc
}

#[test]
fn the_runtime_hashes_each_registered_sheet_as_the_compiler_does() {
    // When
    let doc = document_with_card();

    // Then
    assert_eq!(
        doc.sheet_hash(SheetRef::UserAgent),
        Some(sheet_hash(vapors_css::user_agent::TEXT))
    );
    assert_eq!(
        doc.sheet_hash(SheetRef::Scope("cards/card.wisp")),
        Some(sheet_hash(CARD))
    );
    assert_eq!(doc.sheet_hash(SheetRef::Scope("cards/other.wisp")), None);
}

#[test]
fn a_key_whose_sheets_hash_as_registered_or_are_not_registered_passes() {
    // Given
    static WINNERS: KeyWinners = KeyWinners {
        sheets: &[(SheetRef::Scope("cards/other.wisp"), 7)],
        base: &[],
        conditional: &[],
        shareable: true,
    };
    static KEY: StyleKey = key(&WINNERS);
    let doc = document_with_card();

    // Then
    assert_eq!(doc.check_compiled_sheets(&KEY), Ok(()));
}

#[test]
fn a_key_compiled_against_other_text_is_refused_naming_its_site_the_sheet_and_the_fix() {
    // Given winners read from an older `.card` rule
    static WINNERS: KeyWinners = KeyWinners {
        sheets: &[(SheetRef::Scope("cards/card.wisp"), 1)],
        base: &[],
        conditional: &[],
        shareable: true,
    };
    static KEY: StyleKey = key(&WINNERS);
    let doc = document_with_card();

    // When
    let refusal = doc.check_compiled_sheets(&KEY).expect_err("refused");

    // Then
    assert!(refusal.starts_with("cards/card.wisp:3:1: "), "{refusal}");
    assert!(
        refusal.contains("Scope(\"cards/card.wisp\") with hash 1"),
        "{refusal}"
    );
    assert!(
        refusal.contains(&format!("hashes {}", sheet_hash(CARD))),
        "{refusal}"
    );
    assert!(refusal.contains("fix: rebuild the crate"), "{refusal}");
}
