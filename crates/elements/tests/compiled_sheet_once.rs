//! A compiled sheet's hash is checked once per sheet, not on every
//! mount of its component (`style/compiled_add.rs`); a later mount with
//! the same text but another claimed hash is checked again, so a forged
//! claim is refused even after the genuine one was added.

use vapors_css::sheet_hash::sheet_hash;
use vapors_dom::{CompiledSheet, Dom};
use vapors_elements::{Document, Viewport};
use vapors_text::FontSystem;

const OWN: &str = "cards/card.wisp";
const CARD: &str = ".card { color: red; }";
const VIEWPORT: Viewport = Viewport {
    width: 200.0,
    height: 100.0,
    scale: 1.0,
};

#[test]
fn a_mount_claiming_another_hash_after_the_genuine_one_is_refused() {
    for global in [false, true] {
        // Given: the genuine sheet added, then the same text claiming the
        // hash of other text
        let mut doc = Document::new(FontSystem::bundle_only());
        let add = |doc: &mut Document, sheet| {
            if global {
                doc.add_global_style(sheet);
            } else {
                doc.add_style(sheet);
            }
        };
        add(
            &mut doc,
            CompiledSheet::for_tests_hashed(OWN, CARD, sheet_hash(CARD)),
        );
        let forged = CompiledSheet::for_tests_hashed(OWN, CARD, sheet_hash(".x {}"));

        // When
        add(&mut doc, forged);

        // Then
        let refused = format!("{:?}", doc.update(VIEWPORT).err().expect("refused"));
        assert!(
            refused.contains("for text that hashes otherwise"),
            "global {global}: {refused}"
        );
    }
}

#[test]
fn mounting_the_genuine_sheet_again_changes_nothing() {
    for global in [false, true] {
        // Given
        let mut doc = Document::new(FontSystem::bundle_only());
        let sheet = CompiledSheet::for_tests_hashed(OWN, CARD, sheet_hash(CARD));
        let add = |doc: &mut Document| {
            if global {
                doc.add_global_style(sheet);
            } else {
                doc.add_style(sheet);
            }
        };
        add(&mut doc);
        let first = doc.update(VIEWPORT).err().map(|e| format!("{e:?}"));

        // When
        add(&mut doc);

        // Then: no refusal, and no change to restyle
        assert_eq!(first, None, "global {global}");
        assert!(doc.update(VIEWPORT).is_ok(), "global {global}");
        assert_eq!(doc.update_counts().elements_restyled, 0, "global {global}");
    }
}

#[test]
fn a_refused_other_text_mount_records_no_claim_against_the_held_text() {
    // Given: the genuine sheet, then other text under its name (refused),
    // then the genuine text claiming the other text's hash
    let mut doc = Document::new(FontSystem::bundle_only());
    let other = ".card { color: blue; }";
    doc.add_style(CompiledSheet::for_tests_hashed(OWN, CARD, sheet_hash(CARD)));
    doc.add_style(CompiledSheet::for_tests_hashed(
        OWN,
        other,
        sheet_hash(other),
    ));

    // When
    doc.add_style(CompiledSheet::for_tests_hashed(
        OWN,
        CARD,
        sheet_hash(other),
    ));

    // Then: the forged claim is checked, and refused
    let refused = format!("{:?}", doc.update(VIEWPORT).err().expect("refused"));
    assert!(
        refused.contains("for text that hashes otherwise"),
        "{refused}"
    );
}

#[test]
fn a_repeat_mount_of_a_known_sheet_takes_no_hash() {
    for global in [false, true] {
        // Given
        let mut doc = Document::new(FontSystem::bundle_only());
        let sheet = CompiledSheet::for_tests_hashed(OWN, CARD, sheet_hash(CARD));

        // When: mounted three times
        for _ in 0..3 {
            if global {
                doc.add_global_style(sheet);
            } else {
                doc.add_style(sheet);
            }
        }

        // Then
        assert_eq!(doc.sheet_hashes_taken(), 1, "global {global}");
    }
}
