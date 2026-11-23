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

/// Each type with a placeholder, and what its one refusal says: a type
/// that takes none is refused for the placeholder, naming the type; a
/// type not built is refused once, for its type (that check comes first).
const REFUSED: [(&str, &[&str]); 5] = [
    (
        "range",
        &["type=\"range\" placeholder", "remove the attribute"],
    ),
    (
        "checkbox",
        &["type=\"checkbox\" placeholder", "remove the attribute"],
    ),
    (
        "RANGE",
        &["type=\"RANGE\" placeholder", "remove the attribute"],
    ),
    ("hidden", &["is not implemented"]),
    ("button", &["is not implemented"]),
];

#[test]
fn a_placeholder_on_a_type_that_takes_none_is_refused_once_naming_the_type() {
    for (kind, wants) in REFUSED {
        // Given
        let mut doc = blocks();
        let body = doc.body();
        let input = element(&mut doc, body, "input", "");
        doc.set_attribute(input, "type", kind);
        doc.set_attribute(input, "placeholder", "Search");

        // When
        let errors = refusals(&mut doc);

        // Then
        assert_eq!(errors.len(), 1, "{kind}: {errors:?}");
        for want in wants {
            assert!(errors[0].contains(want), "{kind}: {errors:?}");
        }
    }
}
