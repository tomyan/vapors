//! The `switch` attribute on an input that is not a checkbox is
//! refused, naming the type.

use crate::support::{document, element, refusals};
use vapors_dom::Dom;

#[test]
fn the_switch_attribute_on_another_input_is_refused_naming_its_type() {
    for kind in [Some("text"), Some("range"), Some("number"), None] {
        // Given
        let mut doc = document();
        let body = doc.body();
        let input = element(&mut doc, body, "input", "");
        if let Some(kind) = kind {
            doc.set_attribute(input, "type", kind);
        }
        doc.set_attribute(input, "switch", "");
        doc.add_stylesheet("a.css", "input { display: block; width: 50px }")
            .expect("parses");

        // When
        let errors = refusals(&mut doc);

        // Then
        let named = kind.unwrap_or("text");
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert!(
            errors[0].contains(&format!("type=\"{named}\"")) && errors[0].contains("switch"),
            "{errors:?}"
        );
    }
}
