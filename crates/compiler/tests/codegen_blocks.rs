//! `{if}` and `{for}` compile to the block runtime in `vapors_dom`:
//! an anchor text node marks the position, and the generated builders
//! receive the dom, the parent and the node to insert before. Loop
//! bindings become signals, so holes reading them are patched in
//! place when a row's item or index changes.

#[path = "common/codegen.rs"]
mod codegen;

use codegen::{assert_has, assert_lacks, at, generated_with, refusal};

const SCRIPT: &str = "let count: Signal<i32> = Signal::new(0);\nlet items: Signal<Vec<(u32, String)>> = Signal::new(Vec::new());";

#[test]
fn if_block_uses_conditional_with_an_anchor() {
    // When
    let code = generated_with(
        SCRIPT,
        "<div>{if count.get() > 0}<p>yes</p>{else}<p>no</p>{/if}</div>",
    );

    // Then
    assert_has(
        &code,
        "let if_anchor1 = dom.borrow_mut().create_text(\"\");",
    );
    assert_has(&code, "dom.borrow_mut().append(div0, if_anchor1);");
    assert_has(&code, "vapors_dom::blocks::conditional(");
    assert_has(&code, "move || count.get() > 0,");
    assert_has(
        &code,
        "move |dom: &Rc<RefCell<D>>, parent: NodeId, before: Option<NodeId>| {",
    );
    assert_has(&code, "create_text(\"yes\")");
    assert_has(&code, "create_text(\"no\")");
    assert_has(&code, "insert_before(parent, p2, before);");
    assert_has(&code, "roots.push(Root::Node(p2));");
}

#[test]
fn if_without_else_has_an_empty_otherwise() {
    // When
    let code = generated_with(SCRIPT, "{if count.get() > 0}<p>yes</p>{/if}");

    // Then
    assert_has(&code, "|_dom, _parent, _before| Vec::new(),");
}

#[test]
fn top_level_if_inserts_before_the_components_before() {
    // When
    let code = generated_with(SCRIPT, "{if count.get() > 0}<p>yes</p>{/if}");

    // Then
    assert_has(&code, "insert_before(parent, if_anchor0, before);");
    assert_lacks(&code, "roots.push(Root::Node(if_anchor0));");
}

#[test]
fn a_top_level_block_is_a_root_through_its_handle() {
    // When
    let code = generated_with(SCRIPT, "{if count.get() > 0}<p>yes</p>{/if}");

    // Then: the handle lists the branch's nodes and the anchor as they are now
    assert_has(
        &code,
        "let conditional_block1 = vapors_dom::blocks::conditional(",
    );
    assert_has(&code, "roots.push(Root::Block(conditional_block1));");
}

#[test]
fn a_block_inside_an_element_is_not_a_root() {
    // When
    let code = generated_with(SCRIPT, "<div>{if count.get() > 0}<p>yes</p>{/if}</div>");

    // Then
    assert_lacks(&code, "Root::Block");
}

#[test]
fn for_block_uses_keyed_with_index_and_item_signals() {
    // When
    let code = generated_with(
        SCRIPT,
        "<ul>{for (i, x) in items.get() key=x.0}<li>{i}: {x.1}</li>{/for}</ul>",
    );

    // Then
    assert_has(
        &code,
        "let for_anchor1 = dom.borrow_mut().create_text(\"\");",
    );
    assert_has(&code, "vapors_dom::blocks::keyed(");
    assert_has(
        &code,
        "(items.get()).into_iter().enumerate().map(|(i, x)| ((x.0), x)).collect()",
    );
    assert_has(&code, "i: Signal<usize>, x: Signal<_>| {");
    assert_has(&code, "let shown = &(i.get());");
    assert_has(&code, "let shown = &(x.get().1);");
}

#[test]
fn for_without_key_is_keyed_by_index() {
    // When
    let code = generated_with(SCRIPT, "{for (i, x) in items.get()}<li>{x.1}</li>{/for}");

    // Then
    assert_has(
        &code,
        ".into_iter().enumerate().map(|(i, x)| (i, x)).collect()",
    );
}

#[test]
fn for_with_a_single_name_pattern() {
    // When
    let code = generated_with(
        SCRIPT,
        "{for item in items.get() key=item.0}<li>{item.1}</li>{/for}",
    );

    // Then
    assert_has(&code, ".into_iter().map(|item| ((item.0), item)).collect()");
    assert_has(&code, "_index: Signal<usize>, item: Signal<_>| {");
}

#[test]
fn for_with_an_unsupported_pattern_is_refused() {
    // When
    let err = refusal(&format!(
        "<script>\n{SCRIPT}\n</script>\n<ul>\n  {{for (a, (b, c)) in items.get()}}<li>x</li>{{/for}}\n</ul>"
    ));

    // Then
    assert_eq!((err.line, err.col), (6, 3), "{err}");
    assert!(err.message.contains("(index, item)"), "{err}");
}

#[test]
fn nested_blocks_build_inside_the_branch() {
    // When
    let code = generated_with(
        SCRIPT,
        "{if count.get() > 0}{for x in items.get() key=x.0}<li>{x.1}</li>{/for}{/if}",
    );

    // Then
    let conditional = at(&code, "vapors_dom::blocks::conditional(");
    let keyed = at(&code, "vapors_dom::blocks::keyed(");
    assert!(conditional < keyed, "{code}");
}

#[test]
fn a_loop_binding_shadows_a_signal_of_the_same_name_only_inside_the_loop() {
    // Given: `count` is a component signal and a loop binding
    let code = generated_with(
        SCRIPT,
        "<span>{count}</span>{for count in items.get()}<li>{count.1}</li>{/for}<b>{count}</b>",
    );

    // Then
    assert_has(&code, "let shown = &(count.get());");
    assert_has(&code, "let shown = &(count.get().1);");
}
