//! Snippets (1-R1, R1a): a component tag's children become the child's
//! `children` snippet, built against `dyn Dom`, and `{render name()}`
//! builds a snippet in place.

#[path = "common/codegen.rs"]
mod codegen;

use codegen::{assert_has, assert_lacks, generated_with};

#[test]
fn children_of_a_component_tag_become_its_children_snippet() {
    // When
    let code = generated_with("", "<Card title=\"Curves\"><p>{1}</p></Card>");

    // Then
    assert_has(&code, "let card0_children = Snippet::new(");
    assert_has(&code, "dom: &Rc<RefCell<dyn Dom>>,");
    assert_has(&code, "children: card0_children,");
}

#[test]
fn whitespace_between_a_component_tags_ends_passes_no_children() {
    // When
    let code = generated_with("", "<Card title=\"Curves\">\n  </Card>");

    // Then
    assert_lacks(&code, "Snippet::new(");
}

#[test]
fn render_builds_the_snippet_in_place() {
    // When: at the top level, and inside an element
    let code = generated_with(
        "#[prop]\nlet children: Snippet = Snippet::empty();",
        "{render children()}<div>{render children()}</div>",
    );

    // Then
    assert_has(
        &code,
        "roots.extend(children.render(dom, parent, before, || ()));",
    );
    assert_has(&code, "let _ = children.render(dom, div");
}

#[test]
fn a_snippet_declares_a_typed_local_built_against_dyn_dom() {
    // When
    let code = generated_with(
        "",
        "{snippet row(name: String, at: u32)}<b>{name}{at}</b>{/snippet}",
    );

    // Then: the tuple of its parameter types, read through its arguments
    assert_has(&code, "let row: Snippet<(String, u32)> = Snippet::new(");
    assert_has(&code, "snippet_args: vapors_dom::Args<(String, u32)>");
}

#[test]
fn each_parameter_is_a_memo_of_its_argument_read_as_its_value() {
    // When
    let code = generated_with(
        "",
        "{snippet row(name: String, at: u32)}<b>{at}</b>{/snippet}",
    );

    // Then
    assert_has(&code, "let at = Memo::new({");
    assert_has(&code, "move || snippet_args().1");
    assert_has(&code, "at.get()");
}

#[test]
fn render_arguments_are_a_closure_so_the_snippet_reads_them_as_they_change() {
    // When
    let code = generated_with(
        "let tick: Signal<u32> = Signal::new(0);",
        "{snippet row(at: u32)}<b>{at}</b>{/snippet}{render row(tick)}",
    );

    // Then
    assert_has(&code, "move || (tick.get(),)");
    assert_has(
        &code,
        "roots.extend(row.render(dom, parent, before, render_args",
    );
}

#[test]
fn a_render_before_its_snippet_finds_it_declared_first() {
    // When
    let code = generated_with(
        "",
        "{render row(1)}{snippet row(n: i32)}<b>{n}</b>{/snippet}",
    );

    // Then
    let declared = code.find("let row: Snippet<(i32,)>").expect("declared");
    let rendered = code.find("row.render(").expect("rendered");
    assert!(declared < rendered, "{code}");
}

#[test]
fn a_snippet_inside_a_component_tag_becomes_its_prop() {
    // When
    let code = generated_with("", "<List>{snippet row(n: i32)}<b>{n}</b>{/snippet}</List>");

    // Then: passed as the prop of its name, and no children snippet
    assert_has(&code, "row: row.clone(),");
    assert_lacks(&code, "children:");
}
