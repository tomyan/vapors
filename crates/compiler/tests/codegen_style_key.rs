//! Each template element gets a `static` `StyleKey` at file level and
//! is created with `create_keyed_element(&KEY)`: the scope attribute,
//! `class` and `style` come from the key, so no `set_attribute` is
//! emitted for them, and a `class:` directive toggles through
//! `toggle_classes`.

#[path = "common/codegen.rs"]
mod codegen;

use codegen::{assert_has, assert_lacks, at, generated, generated_with};

const SCRIPT: &str = "let on: Signal<bool> = Signal::new(false);";

const STYLED: &str = "<style>\n.card { color: red; }\n</style>\n<div class=\"card wide\" style=\"width: 8px\" title=\"t\"></div>";

#[test]
fn an_element_is_created_from_its_key_with_site_scope_tag_classes_and_style() {
    // When
    let code = generated(STYLED);

    // Then: the <div> is at line 4, column 1 of counter.wisp
    assert_has(
        &code,
        "static KEY_DIV0: vapors_dom::StyleKey = vapors_dom::StyleKey { site: \"counter.wisp:4:1\", scope: Some(SCOPE), tag: \"div\", classes: &[\"card\", \"wide\"], toggles: &[], style: Some(\"width: 8px\") };",
    );
    assert_has(
        &code,
        "let div0 = dom.borrow_mut().create_keyed_element(&KEY_DIV0);",
    );
    assert_has(&code, "set_attribute(div0, \"title\", \"t\");");
    assert_lacks(&code, "create_element(");
    for fixed in ["\"class\"", "\"style\"", "SCOPE_ATTRIBUTE, SCOPE"] {
        assert_lacks(&code, &format!("set_attribute(div0, {fixed}"));
    }
}

#[test]
fn the_keys_are_file_level_statics_before_mount() {
    // When
    let code = generated(STYLED);

    // Then
    assert!(
        at(&code, "static KEY_DIV0") < at(&code, "fn mount_before"),
        "{code}"
    );
}

#[test]
fn a_component_without_a_style_has_keys_without_scope() {
    // When
    let code = generated("<span></span>");

    // Then
    assert_has(
        &code,
        "scope: None, tag: \"span\", classes: &[], toggles: &[], style: None",
    );
    assert_lacks(&code, "Some(SCOPE)");
}

#[test]
fn class_directives_name_the_keys_toggles_and_one_effect_toggles_them() {
    // When
    let code = generated_with(
        SCRIPT,
        "<div class=\"card\" class:lit={on.get()} class:dim={!on.get()}></div>",
    );

    // Then
    assert_has(&code, "classes: &[\"card\"], toggles: &[\"lit\", \"dim\"]");
    assert_has(
        &code,
        "dom.borrow_mut().toggle_classes(div0, &[(\"lit\", on.get()), (\"dim\", !on.get())]);",
    );
    assert_lacks(&code, "class_names(");
    assert_lacks(&code, "\"class\"");
    assert_eq!(code.matches("Effect::new(").count(), 1, "{code}");
}

#[test]
fn every_element_gets_its_own_key_named_after_its_node() {
    // When
    let code = generated("<ul><li class=\"a\"></li><li class=\"b\"></li></ul>");

    // Then
    assert_has(&code, "static KEY_UL0:");
    assert_has(&code, "static KEY_LI1:");
    assert_has(&code, "static KEY_LI2:");
    assert_has(&code, "create_keyed_element(&KEY_LI2)");
}

#[test]
fn a_literal_id_stays_an_ordinary_attribute() {
    // When
    let code = generated("<div id=\"main\"></div>");

    // Then
    assert_has(&code, "set_attribute(div0, \"id\", \"main\");");
}
