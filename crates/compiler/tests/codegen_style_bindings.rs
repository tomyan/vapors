//! `style:<property>={expr}` (decision 2026-11-20-style-bindings): the
//! properties go in the element's key as `bound`, and each binding is
//! one effect that calls `bind_style` with the typed value.

use crate::common_codegen as codegen;

use codegen::{assert_has, assert_lacks, generated_with, refusal};

const SCRIPT: &str = "let h: Signal<f32> = Signal::new(4.0);\nlet w: Signal<f32> = Signal::new(2.0);\nlet c: Signal<Rgba> = Signal::new(Rgba::rgb(1, 2, 3));";
const DOM: &str = "vapors_elements::__wisp::dom";

#[test]
fn a_length_binding_is_listed_in_the_key_and_set_by_one_effect() {
    // When
    let code = generated_with(SCRIPT, "<div class=\"bar\" style:height={h.get()}></div>");

    // Then
    assert_has(&code, &format!("bound: &[{DOM}::BoundProperty::Height]"));
    assert_has(
        &code,
        &format!("let value = {DOM}::BoundValue::Length(h.get());"),
    );
    assert_has(
        &code,
        &format!("bind_style(div0, {DOM}::BoundProperty::Height, value);"),
    );
    assert_eq!(code.matches("Effect::new(").count(), 1, "{code}");
    assert_lacks(&code, "set_attribute(div0, \"style\"");
}

#[test]
fn a_colour_binding_takes_a_colour_value() {
    // When
    let code = generated_with(SCRIPT, "<div style:background-color={c.get()}></div>");

    // Then
    assert_has(&code, &format!("{DOM}::BoundProperty::BackgroundColor]"));
    assert_has(&code, &format!("{DOM}::BoundValue::Color(c.get())"));
}

#[test]
fn each_binding_is_its_own_effect_so_a_change_reruns_only_its_own() {
    // When
    let code = generated_with(
        SCRIPT,
        "<div style=\"top: 1px\" style:height={h.get()} style:width={w.get()}></div>",
    );

    // Then: keyed in template order, one effect each, the literal kept
    assert_has(
        &code,
        &format!("bound: &[{DOM}::BoundProperty::Height, {DOM}::BoundProperty::Width]"),
    );
    assert_has(&code, "style: Some(\"top: 1px\")");
    assert_eq!(code.matches("Effect::new(").count(), 2, "{code}");
    assert_eq!(code.matches(".bind_style(").count(), 2, "{code}");
}

#[test]
fn a_value_is_read_before_the_tree_is_borrowed() {
    // When
    let code = generated_with(SCRIPT, "<div style:height={h.get()}></div>");

    // Then: the expression is evaluated first, then `dom` is borrowed
    assert!(
        codegen::at(&code, "let value =") < codegen::at(&code, "dom.borrow_mut().bind_style("),
        "{code}"
    );
}

#[test]
fn an_element_without_bindings_lists_none() {
    // When
    let code = generated_with(SCRIPT, "<div class=\"bar\"></div>");

    // Then
    assert_has(&code, "bound: &[], winners");
    assert_lacks(&code, "bind_style");
}

fn refused(template: &str) -> String {
    refusal(&format!("<script>\n{SCRIPT}\n</script>\n{template}")).to_string()
}

const CHECK: &str = "check with `cargo run -p vapors-compiler --bin wisp-check -- counter.wisp`";
const BINDABLE: &str = "width, height, top, left, margin-top, margin-right, margin-bottom, margin-left, padding-top, padding-right, padding-bottom, padding-left, background-color, color";

#[test]
fn a_property_that_is_not_bindable_is_refused_listing_those_that_are() {
    // When
    let err = refused("<div style:border-radius={h.get()}></div>");

    // Then
    assert_eq!(
        err,
        format!(
            "counter.wisp:6:6: `style:border-radius` is not a bindable property: bind one of {BINDABLE}; for any other property write it under a class in the <style> section and toggle it with class:name={{condition}}; {CHECK}"
        )
    );
}

#[test]
fn a_non_literal_property_is_refused() {
    // When
    let err = refused("<div style:{name}={h.get()}></div>");

    // Then
    assert_eq!(
        err,
        format!(
            "counter.wisp:6:6: `style:{{..}}` computes the property name: write the CSS property itself after the colon, style:height={{expr}}; a property changes only through a template binding with a literal name; {CHECK}"
        )
    );
}

#[test]
fn style_on_a_component_tag_is_refused() {
    // When
    let err = refused("<Card style:height={h.get()} />");

    // Then
    assert_eq!(
        err,
        format!(
            "counter.wisp:6:7: `style:height` is on the component tag <Card>: a style binding is for an element, so bind it on an element inside the component, or toggle a class with class:name={{condition}} on the tag; {CHECK}"
        )
    );
}

#[test]
fn a_quoted_value_is_refused_naming_the_type() {
    // When
    let err = refused("<div style:height=\"4px\"></div>");

    // Then
    assert_eq!(
        err,
        format!(
            "counter.wisp:6:6: `style:height` takes an expression of a px f32: write style:height={{expr}}, not a quoted value; {CHECK}"
        )
    );
}

#[test]
fn a_value_of_a_type_the_property_cannot_take_is_refused() {
    let cases = [
        ("height", "\"4px\"", "a string literal", "a px f32"),
        ("height", "true", "a bool literal", "a px f32"),
        ("height", "4", "an integer literal", "a px f32"),
        ("background-color", "1.5", "a float literal", "an Rgba"),
        ("color", "\"red\"", "a string literal", "an Rgba"),
    ];
    for (property, value, got, takes) in cases {
        // When
        let err = refused(&format!("<div style:{property}={{{value}}}></div>"));

        // Then
        let want = format!(
            "counter.wisp:6:6: `style:{property}` takes {takes}, not {got}: bind an expression of that type, such as a signal read or a computed value; {CHECK}"
        );
        assert_eq!(err, want);
    }
}

#[test]
fn the_same_property_bound_twice_is_refused() {
    // When
    let err = refused("<div style:height={h.get()} style:height={w.get()}></div>");

    // Then
    assert!(
        err.starts_with("counter.wisp:6:29: `style:height` is bound twice"),
        "{err}"
    );
}

#[test]
fn a_string_style_is_still_refused_beside_a_binding() {
    // When
    let err = refused("<div style={\"a\"} style:height={h.get()}></div>");

    // Then
    assert!(err.contains("`style={..}` is refused"), "{err}");
}
