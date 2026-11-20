//! What a `placeholder` refuses (`docs/placeholder.md`): an input of a
//! type that takes none, naming the type.

use crate::support;

use support::{element, refusals};
use vapors_dom::Dom;

/// A document whose inputs are blocks of 100 px, so the only refusal
/// is the test's.
fn blocks() -> vapors_elements::Document {
    let mut doc = support::document();
    doc.add_stylesheet("blocks.css", "input { display: block; width: 100px; }")
        .expect("parses");
    doc
}

#[test]
fn a_placeholder_on_a_range_or_a_checkbox_is_refused_naming_the_type() {
    for kind in ["range", "checkbox", "RANGE"] {
        // Given
        let mut doc = blocks();
        let body = doc.body();
        let input = element(&mut doc, body, "input", "");
        doc.set_attribute(input, "type", kind);
        doc.set_attribute(input, "placeholder", "Search");

        // When
        let errors = refusals(&mut doc);

        // Then
        assert_eq!(errors.len(), 1, "{errors:?}");
        let want = format!("type=\"{kind}\" placeholder");
        assert!(errors[0].contains(&want), "{kind}: {errors:?}");
        assert!(errors[0].contains("remove the attribute"), "{errors:?}");
    }
}
