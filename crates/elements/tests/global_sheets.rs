//! A `Styles::Global` component's sheet, registered by its mount
//! (build-time-styles.md, R1a): unscoped, added once per document
//! however often it is mounted, refused like a scoped sheet.

use vapors_css::sheet_hash::sheet_hash;
use vapors_dom::{Dom, SheetRef};
use vapors_elements::{Document, Viewport};
use vapors_text::FontSystem;

const SHEET: &str = "corpus/fixtures/card.wisp";
const CARD: &str = "\np { color: rgb(255, 0, 0); }\n";
const VIEWPORT: Viewport = Viewport {
    width: 200.0,
    height: 100.0,
    scale: 1.0,
};

fn document_with_paragraph() -> Document {
    let mut doc = Document::new(FontSystem::bundle_only());
    let body = doc.body();
    let p = doc.create_element("p");
    doc.insert_before(body, p, None);
    doc
}

#[test]
fn a_global_sheet_styles_unscoped_elements_and_hashes_as_registered() {
    // Given
    let mut doc = document_with_paragraph();

    // When
    doc.add_global_style(SHEET, CARD);
    doc.update(VIEWPORT).expect("updates");

    // Then
    assert_eq!(
        doc.sheet_hash(SheetRef::Global(SHEET)),
        Some(sheet_hash(CARD))
    );
    let p = doc.children(doc.body())[0];
    let style = doc.computed_style(p).expect("styled");
    assert_eq!(
        style.get_by_name("color").expect("a longhand").to_css(),
        "rgb(255 0 0)"
    );
}

#[test]
fn a_second_mounts_registration_of_the_same_sheet_restyles_nothing() {
    // Given
    let mut doc = document_with_paragraph();
    doc.add_global_style(SHEET, CARD);
    doc.update(VIEWPORT).expect("updates");

    // When
    doc.add_global_style(SHEET, CARD);
    doc.update(VIEWPORT).expect("updates");

    // Then
    assert_eq!(doc.update_counts().elements_restyled, 0);
}

#[test]
fn another_text_under_the_same_name_is_refused_naming_the_fix() {
    // Given
    let mut doc = document_with_paragraph();
    doc.add_global_style(SHEET, CARD);

    // When
    doc.add_global_style(SHEET, "\np { color: blue; }\n");
    let refused = format!("{:?}", doc.update(VIEWPORT).err().expect("refused"));

    // Then
    assert!(
        refused.contains("already added with another text"),
        "{refused}"
    );
    assert!(
        refused.contains("give their files different paths"),
        "{refused}"
    );
}

#[test]
fn a_global_sheet_holding_global_selectors_is_refused() {
    // Given
    let mut doc = document_with_paragraph();

    // When
    doc.add_global_style(SHEET, ":global(p) { color: blue; }");
    let refused = format!("{:?}", doc.update(VIEWPORT).err().expect("refused"));

    // Then
    assert!(refused.contains(SHEET), "{refused}");
}
