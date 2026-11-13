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

    // Then: the tag is at line 4, column 1; its key's tag is the one
    // the component's props type declares
    assert_has(
        &code,
        "static KEY_MESH0: vapors_dom::StyleKey = { vapors_dom::StyleKey { site: concat!(env!(\"CARGO_PKG_NAME\"), \"/\", \"counter.wisp:4:1\"), scope: None, tag: MeshProps::ANCHOR_TAG, classes: &[\"body\"], toggles: &[\"braking\"], style: Some(\"color: red\") } };",
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
        "scope: Some(SCOPE), tag: MeshProps::ANCHOR_TAG, classes: &[\"body\"]",
    );
}

#[test]
fn the_keys_tag_comes_from_the_props_type_the_tag_names() {
    // When: a path, and an alias (`use ..::{Group as Section, GroupProps
    // as SectionProps}`) whose spelling is not the component's tag
    let path = generated("<scene3d::Mesh class=\"body\" />");
    let alias = generated("<Section class=\"craft\" />");

    // Then
    assert_has(
        &path,
        "tag: scene3d::MeshProps::ANCHOR_TAG, classes: &[\"body\"]",
    );
    assert_has(&path, "anchor: Some(&KEY_MESH0),");
    assert_has(
        &alias,
        "tag: SectionProps::ANCHOR_TAG, classes: &[\"craft\"]",
    );
}

#[test]
fn a_components_key_brings_in_the_scripts_use_items_to_name_its_props_type() {
    // When: the key is a file-level static, outside mount where the
    // script's `use` items are
    let code = generated_with(
        "use vapors_scene3d_components::{Mesh, MeshProps};",
        "<Mesh class=\"body\" />",
    );

    // Then
    assert_has(
        &code,
        "static KEY_MESH0: vapors_dom::StyleKey = { #[allow(unused_imports)] use vapors_scene3d_components::{Mesh, MeshProps}; vapors_dom::StyleKey {",
    );
}

#[test]
fn a_component_inside_an_element_toggles_through_an_unprefixed_binding() {
    // When: not at the root, where the binding would otherwise be `_mesh1`
    let code = generated_with(SCRIPT, "<div><Mesh class:braking={on.get()} /></div>");

    // Then
    assert_has(&code, "let mesh1 = Mesh::mount_before(");
    assert_has(&code, "let mesh1_anchor = mesh1.anchor_element();");
    assert_lacks(&code, "_mesh1");
}

#[test]
fn a_style_from_an_expression_on_a_component_tag_is_refused() {
    // When
    let err = refusal(&format!(
        "<script>\n{SCRIPT}\n</script>\n<Mesh class=\"body\" style={{css}} />"
    ))
    .to_string();

    // Then
    assert_eq!(
        err,
        "counter.wisp:4:20: `style={..}` is refused: the compiler must see every declaration an element can have; write the declarations under a class in the <style> section and toggle it with class:name={condition}"
    );
}

#[test]
fn an_anchor_attribute_on_a_component_tag_is_refused() {
    // When
    let err = refusal("<Mesh class=\"body\" anchor={key} />").to_string();

    // Then
    assert_eq!(
        err,
        "counter.wisp:1:20: `anchor` is generated from the tag's `class`, `class:` and `style`: remove `anchor={..}` and style the component with those"
    );
}

#[test]
fn a_component_tag_with_more_directives_than_fit_is_refused_naming_its_component() {
    // Given
    let each: Vec<String> = (0..65)
        .map(|i| format!("class:d{i}={{on.get()}}"))
        .collect();
    let text = format!("<script>\n{SCRIPT}\n</script>\n<Mesh {} />", each.join(" "));

    // When
    let err = refusal(&text).to_string();

    // Then
    assert!(
        err.starts_with("counter.wisp:4:1: <Mesh> has 65 `class:` directives"),
        "{err}"
    );
    assert!(
        err.contains("move some to an element inside <Mesh>"),
        "{err}"
    );
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
