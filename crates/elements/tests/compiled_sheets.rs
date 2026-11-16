//! Check 1 of compiled winners (build-time-styles.md, section 2): a
//! sheet a key's winners index must hash as the compiler hashed it; the
//! user-agent sheet and the key's own, scoped or global, must be in the
//! cascade.

use vapors_css::sheet_hash::sheet_hash;
use vapors_dom::{Dom, KeyWinners, SheetRef, StyleKey, Winners};
use vapors_elements::{Document, SheetHash};
use vapors_text::FontSystem;

// with the newlines a `<style>` section's text has
const CARD: &str = "\n.card { color: red; }\n";
const OWN: &str = "cards/card.wisp";

const fn key(winners: &'static KeyWinners) -> StyleKey {
    StyleKey {
        site: "cards/card.wisp:3:1",
        scope: Some(OWN),
        tag: "div",
        classes: &["card"],
        toggles: &[],
        attributes: &[],
        style: None,
        winners: Winners::Compiled(winners),
    }
}

const fn winners(sheets: &'static [(SheetRef, u64)]) -> KeyWinners {
    KeyWinners {
        sheets,
        base: &[],
        conditional: &[],
        blocked_by: &[],
        // the first sheet is the key's own in every test here
        own: Some(0),
    }
}

fn document_with_card() -> Document {
    let mut doc = Document::new(FontSystem::bundle_only());
    doc.add_style(OWN, CARD);
    doc
}

#[test]
fn the_cascade_tells_a_registered_sheet_from_an_absent_one_of_either_kind() {
    // When
    let doc = document_with_card();

    // Then
    let registered = |text| SheetHash::Registered(sheet_hash(text));
    assert_eq!(
        doc.sheet_hash(SheetRef::UserAgent),
        registered(vapors_css::user_agent::TEXT)
    );
    assert_eq!(doc.sheet_hash(SheetRef::Scope(OWN)), registered(CARD));
    assert_eq!(
        doc.sheet_hash(SheetRef::Scope("cards/other.wisp")),
        SheetHash::Absent
    );
    assert_eq!(doc.sheet_hash(SheetRef::Global(OWN)), SheetHash::Absent);
}

#[test]
fn a_scope_whose_sheet_was_refused_is_absent_from_the_cascade() {
    // Given
    let mut doc = Document::new(FontSystem::bundle_only());

    // When: the sheet does not parse
    doc.add_style(OWN, ".card { color: red;");

    // Then
    assert_eq!(doc.sheet_hash(SheetRef::Scope(OWN)), SheetHash::Absent);
}

#[test]
fn a_key_whose_sheets_hash_as_registered_or_are_another_absent_scope_passes() {
    // Given: CARD's hash is not a constant, so the key is leaked
    let sheets = vec![
        (SheetRef::Scope(OWN), sheet_hash(CARD)),
        (SheetRef::Scope("cards/other.wisp"), 7),
    ];
    let winners: &'static KeyWinners = Box::leak(Box::new(winners(sheets.leak())));
    let key: &'static StyleKey = Box::leak(Box::new(key(winners)));
    let doc = document_with_card();

    // Then
    assert_eq!(doc.check_compiled_sheets(key), Ok(()));
}

#[test]
fn a_key_compiled_against_other_text_is_refused_naming_both_hashes_as_written() {
    // Given winners read from an older `.card` rule
    static WINNERS: KeyWinners = winners(&[(SheetRef::Scope(OWN), 1)]);
    static KEY: StyleKey = key(&WINNERS);
    let doc = document_with_card();

    // When
    let refusal = doc.check_compiled_sheets(&KEY).expect_err("refused");

    // Then: in the generated code's hex
    let hex = format!("{:016x}", sheet_hash(CARD));
    assert!(refusal.starts_with("cards/card.wisp:3:1: "), "{refusal}");
    assert!(
        refusal.contains("with hash 0x0000_0000_0000_0001"),
        "{refusal}"
    );
    assert!(
        refusal.contains(&format!("hashes 0x{}_", &hex[..4])),
        "{refusal}"
    );
    assert!(refusal.contains("fix: rebuild the crate"), "{refusal}");
}

#[test]
fn a_key_whose_own_scope_is_absent_is_refused_as_two_names_for_one_sheet() {
    // Given: the compiler named the own sheet otherwise than the mount
    static WINNERS: KeyWinners = winners(&[(SheetRef::Scope(OWN), 1)]);
    static KEY: StyleKey = key(&WINNERS);
    let mut doc = Document::new(FontSystem::bundle_only());
    doc.add_style("cards//card.wisp", CARD);

    // When
    let refusal = doc.check_compiled_sheets(&KEY).expect_err("refused");

    // Then
    assert!(refusal.contains("its own sheet"), "{refusal}");
    assert!(refusal.contains("fix: give both one name"), "{refusal}");
}

#[test]
fn a_global_key_whose_own_global_sheet_is_absent_is_refused() {
    // Given: a key of a global component (no scope); its sheet never added
    static WINNERS: KeyWinners = winners(&[(SheetRef::Global("corpus/fixtures/x.wisp"), 1)]);
    static KEY: StyleKey = StyleKey {
        scope: None,
        ..key(&WINNERS)
    };
    let doc = document_with_card();

    // When
    let refusal = doc.check_compiled_sheets(&KEY).expect_err("refused");

    // Then
    assert!(refusal.contains("its own sheet"), "{refusal}");
}

#[test]
fn a_global_key_passes_when_another_components_global_sheet_is_absent() {
    // Given: its own sheet added by its mount; a sibling fixture's not
    let own = "corpus/fixtures/x.wisp";
    let sheets = vec![
        (SheetRef::Global(own), sheet_hash(CARD)),
        (SheetRef::Global("corpus/fixtures/y.wisp"), 7),
    ];
    let winners: &'static KeyWinners = Box::leak(Box::new(winners(sheets.leak())));
    let key: &'static StyleKey = Box::leak(Box::new(StyleKey {
        scope: None,
        ..key(winners)
    }));
    let mut doc = Document::new(FontSystem::bundle_only());
    doc.add_global_style(own, CARD);

    // Then
    assert_eq!(doc.check_compiled_sheets(key), Ok(()));
}

#[test]
fn a_scope_and_a_global_sheet_of_one_name_are_held_apart() {
    // Given: one name, two kinds, two texts
    let mut doc = Document::new(FontSystem::bundle_only());
    doc.add_style(OWN, CARD);
    doc.add_global_style(OWN, ".card { color: blue; }");

    // Then
    let registered = |text| SheetHash::Registered(sheet_hash(text));
    assert_eq!(doc.sheet_hash(SheetRef::Scope(OWN)), registered(CARD));
    assert_eq!(
        doc.sheet_hash(SheetRef::Global(OWN)),
        registered(".card { color: blue; }")
    );
}
