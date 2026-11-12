use vapors_css::cascade::ComputedStyle;
use vapors_text::FontSystem;

use super::differing_property;
use crate::document::Document;
use crate::update::Viewport;

/// The styles of two elements, one per class, under `css`.
fn styles(css: &str, classes: [&str; 2]) -> [ComputedStyle; 2] {
    use vapors_dom::Dom;
    let mut doc = Document::new(FontSystem::bundle_only());
    let body = doc.body();
    let nodes = classes.map(|class| {
        let node = doc.create_element("div");
        doc.set_attribute(node, "class", class);
        doc.append(body, node);
        node
    });
    doc.add_stylesheet("t.css", css).expect("parses");
    let viewport = Viewport {
        width: 100.0,
        height: 100.0,
        scale: 1.0,
    };
    doc.update(viewport).expect("updates");
    nodes.map(|node| doc.computed_style(node).expect("styled").clone())
}

#[test]
fn an_element_that_stopped_matching_a_rule_names_the_kept_value() {
    // Given: the kept style has the rule's `color`, the fresh one has not
    let [kept, fresh] = styles(".on { color: red; }", ["on", "off"]);

    // When
    let message = differing_property(Some(&kept), &fresh);

    // Then
    assert!(
        message.starts_with('`') && message.contains(" fresh, ") && message.ends_with(" kept"),
        "{message}"
    );
    assert!(!message.contains("no kept style"), "{message}");
}

#[test]
fn a_difference_outside_the_longhands_is_not_called_a_missing_style() {
    // Given: only a custom property differs
    let [kept, fresh] = styles(".on { --x: 1; }", ["on", "off"]);

    // When
    let message = differing_property(Some(&kept), &fresh);

    // Then
    assert!(message.starts_with("the longhands are equal"), "{message}");
}

#[test]
fn an_element_with_no_kept_style_says_so() {
    let [_, fresh] = styles("", ["a", "b"]);

    assert_eq!(differing_property(None, &fresh), "no kept style");
}
