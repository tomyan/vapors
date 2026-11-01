//! `bind:this` written any way but a signal of the shape where it is:
//! refused, naming the fix.

#[path = "common/codegen.rs"]
mod codegen;

use codegen::refusal;

const SCRIPT: &str = "let names: Signal<Vec<String>> = Signal::new(Vec::new());\nlet one: Signal<Option<NodeId>> = Signal::new(None);\nlet rows: Signal<BTreeMap<String, NodeId>> = Signal::new(BTreeMap::new());";

fn refused(template: &str) -> String {
    refusal(&format!("<script>\n{SCRIPT}\n</script>\n{template}")).to_string()
}

#[test]
fn bind_this_without_a_signal_is_refused() {
    let why = refused("<input bind:this=\"one\" />");
    assert!(why.contains("bind:this={signal}"), "{why}");
}

#[test]
fn an_option_signal_in_a_for_row_is_refused() {
    let why = refused(
        "<ul>{for name in names.get() key=name.clone()}<li bind:this={one}>{name}</li>{/for}</ul>",
    );
    assert!(why.contains("Signal<BTreeMap<K, NodeId>>"), "{why}");
}

#[test]
fn a_map_signal_outside_a_for_is_refused() {
    let why = refused("<p bind:this={rows}>x</p>");
    assert!(why.contains("Signal<Option<NodeId>>"), "{why}");
}

#[test]
fn a_signal_of_another_type_is_refused() {
    let why = refused("<p bind:this={names}>x</p>");
    assert!(why.contains("Signal<Option<NodeId>>"), "{why}");
}

#[test]
fn bind_this_in_a_for_without_a_key_is_refused() {
    let why = refused("<ul>{for name in names.get()}<li bind:this={rows}>{name}</li>{/for}</ul>");
    assert!(why.contains("key="), "{why}");
}

#[test]
fn bind_this_in_a_nested_for_is_refused() {
    let why = refused(
        "<ul>{for a in names.get() key=a.clone()}{for b in names.get() key=b.clone()}<li bind:this={rows}>{b}</li>{/for}{/for}</ul>",
    );
    assert!(why.contains("nested"), "{why}");
}

#[test]
fn bind_this_on_a_component_tag_is_refused() {
    let why = refusal(
        "<script>\nuse crate::item::{Item, ItemProps};\nlet one: Signal<Option<NodeId>> = Signal::new(None);\n</script>\n<Item bind:this={one} />",
    )
    .to_string();
    assert!(why.contains("`bind:this` binds an element"), "{why}");
}
