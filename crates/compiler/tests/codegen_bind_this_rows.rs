//! `bind:this` inside `{for}`: the row's node into a
//! `Signal<BTreeMap<K, NodeId>>` under the row's `key=`, and out of it
//! when the row goes, unless the key holds another node by then.

#[path = "common/codegen.rs"]
mod codegen;

use codegen::{assert_has, at, generated_with};

const SCRIPT: &str = "let names: Signal<Vec<String>> = Signal::new(Vec::new());\nlet rows: Signal<BTreeMap<String, NodeId>> = Signal::new(BTreeMap::new());";

#[test]
fn a_row_binds_its_node_under_its_key() {
    // When
    let code = generated_with(
        SCRIPT,
        "<ul>{for name in names.get() key=name.clone()}<li bind:this={rows}>{name}</li>{/for}</ul>",
    );

    // Then: the key from the row's item, then the node under it
    assert_has(
        &code,
        "let key = { let name = name.get_untracked(); name.clone() };",
    );
    assert_has(
        &code,
        "this.update(|nodes| { nodes.insert(key.clone(), li2); });",
    );
    assert_has(
        &code,
        "if this.try_with_untracked(|nodes| nodes.get(&key) == Some(&li2)) == Ok(true)",
    );
    assert_has(&code, "this.update(|nodes| { nodes.remove(&key); });");
}

#[test]
fn a_key_that_is_the_item_itself_is_read_directly() {
    // When
    let code = generated_with(
        SCRIPT,
        "<ul>{for name in names.get() key=name}<li bind:this={rows}>{name}</li>{/for}</ul>",
    );

    // Then
    assert_has(&code, "let key = name.get_untracked();");
}

#[test]
fn a_key_from_the_index_reads_the_index() {
    // When
    let code = generated_with(
        SCRIPT,
        "<ul>{for (i, name) in names.get() key=i.to_string()}<li bind:this={rows}>{name}</li>{/for}</ul>",
    );

    // Then: only the names the key mentions are read
    assert_has(
        &code,
        "let key = { let i = i.get_untracked(); i.to_string() };",
    );
}

#[test]
fn the_keys_clone_is_allowed_for_a_copy_key() {
    // When: a usize key, which clippy's clone_on_copy would refuse
    let code = generated_with(
        "let names: Signal<Vec<String>> = Signal::new(Vec::new());\nlet rows: Signal<std::collections::BTreeMap<usize, NodeId>> = Signal::new(std::collections::BTreeMap::new());",
        "<ul>{for (i, name) in names.get() key=i}<li bind:this={rows}>{name}</li>{/for}</ul>",
    );

    // Then: the allowance comes right before the binding
    assert_has(&code, "#[allow(clippy::clone_on_copy");
    assert!(at(&code, "#[allow(clippy::clone_on_copy") < at(&code, "let key = i.get_untracked();"));
}
