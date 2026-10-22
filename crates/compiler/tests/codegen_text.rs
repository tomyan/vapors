//! Translated from sumi's `codegen_mixed_test.go` and the text half of
//! `codegen_static_test.go`: literal text is created once, each
//! `{expr}` hole is its own text node with its own effect, and mixed
//! content keeps source order.

#[path = "common/codegen.rs"]
mod codegen;

use codegen::{assert_has, assert_lacks, at, generated, generated_with};

#[test]
fn mixed_content_children_keep_source_order() {
    // When
    let code = generated("<p>hello <strong>bold</strong> tail</p>");

    // Then
    let hello = at(&code, "create_text(\"hello \")");
    let bold = at(&code, "create_text(\"bold\")");
    let tail = at(&code, "create_text(\" tail\")");
    assert!(hello < bold && bold < tail, "{code}");
}

#[test]
fn an_expression_hole_gets_its_own_text_node_and_effect() {
    // Given
    let script = "let count: Signal<i32> = Signal::new(0);";

    // When
    let code = generated_with(script, "<p>count: {count} <em>items</em></p>");

    // Then
    assert_has(&code, "create_text(\"count: \")");
    assert_has(
        &code,
        "let count_text2 = dom.borrow_mut().create_text(\"\");",
    );
    assert_has(&code, "let value = (count.get()).to_string();");
    assert_has(&code, "set_text(count_text2, &value);");
    assert_has(&code, "create_text(\" \")");
    assert_eq!(code.matches("Effect::new(").count(), 1, "{code}");
}

#[test]
fn literal_text_has_no_effect() {
    // When
    let code = generated("<span>Hello</span>");

    // Then
    assert_has(&code, "create_text(\"Hello\")");
    assert_lacks(&code, "Effect::new(");
}

#[test]
fn a_hole_effect_captures_a_clone_of_the_dom() {
    // Given
    let script = "let count: Signal<i32> = Signal::new(0);";

    // When
    let code = generated_with(script, "<span>{count}</span>");

    // Then
    let effect = at(&code, "Effect::new(");
    let clone = at(&code, "let dom = Rc::clone(dom);");
    assert!(
        clone < effect,
        "the clone is made before the effect: {code}"
    );
}

#[test]
fn a_complex_expression_hole_is_named_after_its_position() {
    // Given
    let script = "let count: Signal<i32> = Signal::new(0);";

    // When
    let code = generated_with(script, "<span>{count * 2}</span>");

    // Then
    assert_has(&code, "let text1 = dom.borrow_mut().create_text(\"\");");
    assert_has(&code, "let value = (count.get() * 2).to_string();");
}

#[test]
fn text_literals_are_escaped() {
    // When
    let code = generated("<span>say \"hi\" \\ now</span>");

    // Then
    assert_has(&code, "create_text(\"say \\\"hi\\\" \\\\ now\")");
}
