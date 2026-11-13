//! A component tag carrying `class`, `class:` or `style` gets a
//! `StyleKey` as an element does, passed to the component as its
//! `anchor` prop, and its `class:` directives toggle the element the
//! component returns from `anchor_element` (docs/plans/css-for-3d.md,
//! D1(b), slice C1). The attributes are not props.

#[path = "common/codegen.rs"]
mod codegen;

use codegen::{assert_has, assert_lacks, generated, generated_with, refusal};

const SCRIPT: &str = "let on: Signal<bool> = Signal::new(false);";

#[test]
fn a_component_tag_with_classes_passes_its_key_as_the_anchor_prop() {
    // When
    let code = generated_with(
        SCRIPT,
        "<Mesh class=\"body\" class:braking={on.get()} style=\"color: red\" />",
    );

    // Then: the tag is at line 4, column 1; its key's tag is the
    // component's name in lower case
    assert_has(
        &code,
        "static KEY_MESH0: vapors_dom::StyleKey = vapors_dom::StyleKey { site: concat!(env!(\"CARGO_PKG_NAME\"), \"/\", \"counter.wisp:4:1\"), scope: None, tag: \"mesh\", classes: &[\"body\"], toggles: &[\"braking\"], style: Some(\"color: red\") };",
    );
    assert_has(&code, "anchor: Some(&KEY_MESH0),");
    assert_lacks(&code, "braking:");
    assert_lacks(&code, "class: \"body\".into()");
    assert_lacks(&code, "style: \"color: red\".into()");
}

#[test]
fn a_components_class_directives_toggle_its_anchor_element_in_one_effect() {
    // When
    let code = generated_with(
        SCRIPT,
        "<Mesh class:braking={on.get()} class:lit={!on.get()} />",
    );

    // Then
    assert_has(&code, "let mesh0_anchor = mesh0.anchor_element();");
    assert_has(
        &code,
        "dom.borrow_mut().toggle_classes(mesh0_anchor, &[(\"braking\", on.get()), (\"lit\", !on.get())]);",
    );
    assert_eq!(code.matches("Effect::new(").count(), 1, "{code}");
}

#[test]
fn a_component_tag_in_a_scoped_component_carries_the_scope() {
    // When
    let code = generated("<style>\n.body { color: red; }\n</style>\n<Mesh class=\"body\" />");

    // Then
    assert_has(
        &code,
        "scope: Some(SCOPE), tag: \"mesh\", classes: &[\"body\"]",
    );
}

#[test]
fn a_path_tag_is_keyed_by_its_last_segment() {
    // When
    let code = generated("<scene3d::Mesh class=\"body\" />");

    // Then
    assert_has(&code, "tag: \"mesh\", classes: &[\"body\"]");
    assert_has(&code, "anchor: Some(&KEY_MESH0),");
}

#[test]
fn a_component_tag_without_classes_or_style_has_no_key_and_no_anchor_prop() {
    // When
    let code = generated("<Mesh title=\"a\" />");

    // Then
    assert_lacks(&code, "KEY_MESH0");
    assert_lacks(&code, "anchor:");
    assert_lacks(&code, "anchor_element");
}

#[test]
fn a_class_from_an_expression_on_a_component_tag_is_refused() {
    // When
    let err = refusal(&format!(
        "<script>\n{SCRIPT}\n</script>\n<Mesh class={{name}} />"
    ))
    .to_string();

    // Then
    assert!(err.starts_with("counter.wisp:4:7:"), "{err}");
    assert!(err.contains("class:name={condition}"), "{err}");
}

#[test]
fn a_directive_naming_a_literal_class_of_the_tag_is_refused() {
    // When
    let err = refusal(&format!(
        "<script>\n{SCRIPT}\n</script>\n<Mesh class=\"body\" class:body={{on.get()}} />"
    ))
    .to_string();

    // Then
    assert!(err.starts_with("counter.wisp:4:20:"), "{err}");
    assert!(err.contains("remove `body` from `class=\"...\"`"), "{err}");
}
