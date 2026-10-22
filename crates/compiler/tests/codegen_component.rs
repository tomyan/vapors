//! Translated from sumi's `codegen_component_test.go`,
//! `codegen_static_test.go` and `codegen_style_test.go` where the
//! expectations transfer: the component type, its signal fields, the
//! props struct, child mounts, and the stylesheet passed through. Not
//! translated: sumi's layout, units and standard-props tests describe
//! terminal layout attributes folded into a layout tree; here styling
//! is the css crate's at run time, so the compiler only carries the
//! `<style>` text.

#[path = "common/codegen.rs"]
mod codegen;

use codegen::{assert_has, assert_lacks, generated, generated_with};

#[test]
fn generate_component_with_signal_field_and_effect() {
    // Given
    let script = "let count: Signal<i32> = Signal::new(0);";

    // When
    let code = generated_with(script, "<span>Count: {count}</span>");

    // Then
    assert_has(&code, "pub struct Counter {");
    assert_has(&code, "pub count: Signal<i32>,");
    assert_has(&code, "Signal::named(\"count\", 0)");
    assert_has(&code, "count.get()");
    assert_has(&code, "Effect::new(");
    assert_has(&code, "pub fn mount<D: Dom + 'static>(");
    assert_has(&code, "pub fn mount_before<D: Dom + 'static>(");
    assert_has(&code, "pub fn unmount<D: Dom>(self, dom: &Rc<RefCell<D>>)");
}

#[test]
fn memo_becomes_a_field() {
    // Given
    let script = "let count: Signal<i32> = Signal::new(1);\nlet doubled: Memo<i32> = Memo::new(move || count.get() * 2);";

    // When
    let code = generated_with(script, "<span>{doubled}</span>");

    // Then
    assert_has(&code, "pub doubled: Memo<i32>,");
    assert_has(&code, "Memo::named(\"doubled\", move || count.get() * 2)");
    assert_has(&code, "doubled.get()");
}

#[test]
fn generated_code_is_valid_rust() {
    // When
    let code = generated("<div class=\"a\"><span>{1 + 1}</span></div>");

    // Then
    syn::parse_file(&code).unwrap_or_else(|err| panic!("{err}\n{code}"));
}

#[test]
fn child_component_is_mounted_with_props() {
    // When
    let code = generated_with(
        "",
        "<div><span>Parent</span><Greeting name=\"Sumi\" /></div>",
    );

    // Then
    assert_has(&code, "let greeting3_props = GreetingProps {");
    assert_has(
        &code,
        "Greeting::mount_before(dom, div0, None, greeting3_props);",
    );
    assert_has(&code, "name: \"Sumi\".into(),");
    assert_has(&code, "..Default::default()");
}

#[test]
fn top_level_child_component_contributes_its_roots() {
    // When
    let code = generated_with("", "<Greeting />");

    // Then
    assert_has(
        &code,
        "Greeting::mount_before(dom, parent, before, greeting0_props);",
    );
    assert_has(
        &code,
        "roots.extend(greeting0.root_entries().iter().cloned());",
    );
}

#[test]
fn bind_prop_passes_the_signal_itself() {
    // Given
    let script = "let name: Signal<String> = Signal::new(String::new());";

    // When
    let code = generated_with(script, "<Field bind:value={name} />");

    // Then
    assert_has(&code, "let field0_value = name;");
    assert_has(&code, "value: field0_value,");
    assert_lacks(&code, "name.get()");
}

#[test]
fn props_become_a_struct_with_defaults() {
    // Given
    let script = "#[prop]\nlet label: String = String::from(\"Count\");\nlet count: Signal<i32> = Signal::new(0);";

    // When
    let code = generated_with(script, "<span>{label}: {count}</span>");

    // Then
    assert_has(&code, "pub struct CounterProps {");
    assert_has(&code, "pub label: String,");
    assert_has(&code, "impl Default for CounterProps {");
    assert_has(&code, "label: String::from(\"Count\"),");
    assert_has(&code, "props: CounterProps,");
    assert_has(&code, "let CounterProps { label } = props;");
    assert_has(&code, "let shown = &(label);");
    assert_lacks(&code, "label.get()");
}

#[test]
fn signal_typed_prop_is_read_like_a_signal() {
    // Given
    let script = "#[prop]\nlet count: Signal<i32> = Signal::new(0);";

    // When
    let code = generated_with(script, "<span>{count}</span>");

    // Then
    assert_has(&code, "pub count: Signal<i32>,");
    assert_has(&code, "count.get()");
}

#[test]
fn style_is_passed_through_untouched() {
    // When
    let code = generated("<style>\n.a { color: red; }\n</style>\n<div></div>");

    // Then
    assert_has(
        &code,
        "pub const STYLE: &str = \"\\n.a { color: red; }\\n\";",
    );
}

#[test]
fn a_component_without_style_has_an_empty_stylesheet() {
    // When
    let code = generated("<div></div>");

    // Then
    assert_has(&code, "pub const STYLE: &str = \"\";");
}

#[test]
fn script_statements_are_kept_in_order_before_the_template() {
    // Given
    let script =
        "let total = 2;\nlet count: Signal<i32> = Signal::new(total);\nuse std::fmt::Write as _;";

    // When
    let code = generated_with(script, "<span>{count}</span>");

    // Then
    let total = codegen::at(&code, "let total = 2;");
    let count = codegen::at(&code, "Signal::named(\"count\", total)");
    let element = codegen::at(&code, "create_element(\"span\")");
    assert!(total < count && count < element, "{code}");
    assert_has(&code, "use std::fmt::Write as _;");
}

#[test]
fn top_level_elements_become_roots() {
    // When
    let code = generated("<div></div><span></span>");

    // Then
    assert_has(&code, "insert_before(parent, div0, before);");
    assert_has(&code, "roots.push(Root::Node(div0));");
    assert_has(&code, "roots.push(Root::Node(span1));");
    assert_has(&code, "Counter { owner, roots }");
}

#[test]
fn the_file_names_its_source_and_warns_against_editing() {
    // When
    let code = generated("<div></div>");

    // Then: a plain comment, since `include!` refuses inner attributes
    assert!(
        code.starts_with(
            "// Generated from `counter.wisp` by vapors-compiler; edit the .wisp, not this file.\n"
        ),
        "{code}"
    );
    assert_lacks(&code, "//!");
}
