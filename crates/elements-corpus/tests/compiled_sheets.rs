//! Check 1 of compiled winners over every keyed element of every
//! mounted fixture (review of #430, H1): a fixture's keys reference the
//! other fixtures' global sheets, which are absent while it is mounted
//! alone, and only its own sheet must be there.

use std::cell::RefCell;
use std::rc::Rc;

use vapors_dom::Dom;
use vapors_elements::Document;
use vapors_elements_corpus::all;
use vapors_text::FontSystem;

#[test]
fn every_keyed_element_of_every_fixture_passes_check_1_mounted_alone() {
    let mut checked = 0;
    for fixture in all() {
        // Given
        let doc = Rc::new(RefCell::new(Document::new(FontSystem::bundle_only())));
        let body = doc.borrow().body();
        (fixture.mount)(&doc, body);
        let doc = doc.borrow();

        // Then
        for node in doc.elements() {
            let Some(key) = doc.style_key(node) else {
                continue;
            };
            let checked_key = doc.check_compiled_sheets(key);
            assert_eq!(checked_key, Ok(()), "{}", fixture.name);
            checked += 1;
        }
    }
    assert!(checked > 100, "only {checked} keyed elements were checked");
}
