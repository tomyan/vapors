//! A `StyleKey` a tree cannot keep true is refused where an element is
//! created from it, naming the site and the fix: a class or directive
//! name that is empty or holds whitespace, a directive listed twice or
//! also a literal class, more than `MAX_TOGGLES` directives, an
//! attribute its own fields carry or one listed twice.

use vapors_dom::recording::{Call, RecordingDom};
use vapors_dom::{Dom, MAX_TOGGLES, StyleKey};

fn key(classes: &'static [&'static str], toggles: &'static [&'static str]) -> StyleKey {
    StyleKey {
        site: "row.wisp:2:3",
        scope: None,
        tag: "li",
        classes,
        toggles,
        attributes: &[],
        style: None,
    }
}

/// `count` distinct directive names, leaked for a `'static` key.
fn directives(count: usize) -> &'static [&'static str] {
    let names: Vec<&'static str> = (0..count)
        .map(|index| &*Box::leak(format!("d{index}").into_boxed_str()))
        .collect();
    Box::leak(names.into_boxed_slice())
}

#[test]
fn a_malformed_key_is_refused_naming_the_site_and_the_fix() {
    let at = "the StyleKey of <li> at row.wisp:2:3";
    for (classes, toggles, problem) in [
        (&[""][..], &[][..], "has an empty class name; drop it"),
        (&[], &[""], "has an empty class name; drop it"),
        (
            &["a b"],
            &[],
            "has the class name \"a b\", which holds whitespace; give each class its own entry",
        ),
        (
            &[],
            &["on\t"],
            "has the class name \"on\\t\", which holds whitespace; give each class its own entry",
        ),
        (
            &[],
            &["on", "off", "on"],
            "lists `class:on` twice; give each directive once",
        ),
        (
            &["row", "on"],
            &["on"],
            "has `on` both as a literal class and as `class:on`; drop one",
        ),
    ] {
        // When
        let refusal = key(classes, toggles).shape_refusal();

        // Then
        assert_eq!(
            refusal,
            Some(format!("{at} {problem}")),
            "{classes:?} {toggles:?}"
        );
    }
}

#[test]
fn an_attribute_a_field_carries_or_listed_twice_is_refused() {
    for (attributes, problem) in [
        (
            &[("Class", "x")][..],
            "lists `Class` under `attributes`, which its own field carries",
        ),
        (
            &[("style", "x")],
            "lists `style` under `attributes`, which its own field carries",
        ),
        (
            &[("title", "a"), ("Title", "b")],
            "lists attribute `Title` twice",
        ),
    ] {
        // Given
        let key = StyleKey {
            attributes,
            ..key(&[], &[])
        };

        // Then
        assert_eq!(
            key.shape_refusal(),
            Some(format!("the StyleKey of <li> at row.wisp:2:3 {problem}")),
            "{attributes:?}"
        );
    }
    assert_eq!(
        StyleKey {
            attributes: &[("title", "a"), ("type", "b")],
            ..key(&[], &[])
        }
        .shape_refusal(),
        None
    );
}

#[test]
fn more_directives_than_fit_the_mask_are_refused() {
    // Then
    assert_eq!(key(&[], directives(MAX_TOGGLES)).shape_refusal(), None);
    assert_eq!(
        key(&[], directives(MAX_TOGGLES + 1)).shape_refusal(),
        Some("the StyleKey of <li> at row.wisp:2:3 has 65 `class:` directives; at most 64 fit one element, so move some to a child element".to_owned())
    );
}

#[test]
fn a_well_formed_key_is_not_refused() {
    // Then
    assert_eq!(
        key(&["row", "even"], &["marked", "wide"]).shape_refusal(),
        None
    );
}

static TWICE: StyleKey = StyleKey {
    site: "row.wisp:2:3",
    scope: None,
    tag: "li",
    classes: &[],
    toggles: &["on", "on"],
    attributes: &[],
    style: None,
};

#[test]
fn creating_an_element_from_a_malformed_key_records_the_refusal() {
    // Given
    let mut dom = RecordingDom::new();
    dom.expect_refusals();

    // When
    let li = dom.create_keyed_element(&TWICE);

    // Then
    assert_eq!(dom.refusals().len(), 1);
    assert_eq!(
        dom.calls()[1],
        Call::Refused {
            node: li,
            refusal: "create_keyed_element: the StyleKey of <li> at row.wisp:2:3 lists `class:on` twice; give each directive once".to_owned()
        }
    );
}
