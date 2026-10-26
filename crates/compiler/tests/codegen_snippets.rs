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
    assert_has(&code, "let card0_children = vapors_dom::Snippet::new(");
    assert_has(
        &code,
        "move |dom: &Rc<RefCell<dyn Dom>>, parent: NodeId, before: Option<NodeId>| {",
    );
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
    assert_has(&code, "roots.extend(children.render(dom, parent, before));");
    assert_has(&code, "let _ = children.render(dom, div");
}
