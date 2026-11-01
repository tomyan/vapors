//! `bind:this={signal}` on an element: the element's node into a
//! `Signal<Option<NodeId>>` when it mounts, and `None` when it unmounts
//! (unless the signal holds another node by then); refused on a
//! component tag and without a signal.

#[path = "common/codegen.rs"]
mod codegen;

use codegen::{assert_has, generated_with, refusal};

const SCRIPT: &str = "let field: Signal<Option<NodeId>> = Signal::new(None);";

#[test]
fn bind_this_sets_the_node_and_clears_it_on_cleanup() {
    // When
    let code = generated_with(SCRIPT, "<input bind:this={field} />");

    // Then
    assert_has(&code, "field.set(Some(input0));");
    assert_has(&code, "vapors_signals::on_cleanup(");
    assert_has(&code, "if this.try_get_untracked() == Ok(Some(input0))");
}

#[test]
fn bind_this_without_a_signal_is_refused() {
    let why = refusal(&format!(
        "<script>\n{SCRIPT}\n</script>\n<input bind:this=\"field\" />"
    ))
    .to_string();
    assert!(why.contains("bind:this={signal}"), "{why}");
}
