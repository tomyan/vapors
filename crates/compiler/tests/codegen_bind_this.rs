//! `bind:this={signal}` on an element: the node into a
//! `Signal<Option<NodeId>>` once the element is built, children
//! included, and out again when it unmounts (unless the signal holds
//! another node by then), the expression evaluated once. Refusals:
//! `codegen_bind_this_refusals.rs`; inside `{for}`:
//! `codegen_bind_this_rows.rs`.

#[path = "common/codegen.rs"]
mod codegen;

use codegen::{assert_has, assert_lacks, at, generated_with};

const SCRIPT: &str = "let field: Signal<Option<NodeId>> = Signal::new(None);";

#[test]
fn bind_this_sets_the_node_and_clears_it_on_cleanup() {
    // When
    let code = generated_with(SCRIPT, "<input bind:this={field} />");

    // Then
    assert_has(&code, "this.set(Some(input0));");
    assert_has(&code, "vapors_signals::on_cleanup(");
    assert_has(&code, "if this.try_get_untracked() == Ok(Some(input0))");
}

#[test]
fn bind_this_evaluates_its_expression_once() {
    // When
    let code = generated_with(SCRIPT, "<input bind:this={field} />");

    // Then: set and cleanup both act on the one value
    assert_has(&code, "let this = field;");
    assert_lacks(&code, "field.set(");
}

#[test]
fn bind_this_is_set_after_the_elements_children_and_later_attributes() {
    // When
    let code = generated_with(SCRIPT, "<p bind:this={field} title={field}>text</p>");

    // Then
    let set = at(&code, "this.set(Some(p0));");
    assert!(at(&code, "create_text(") < set, "{code}");
    assert!(
        at(&code, "set_attribute_if_changed(p0, \"title\"") < set,
        "{code}"
    );
}
