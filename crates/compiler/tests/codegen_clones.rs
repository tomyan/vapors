//! A builder closure gets a clone of each non-Copy binding it reads, and
//! of each that a component tag inside it reads in its children (1-R1
//! review): the closure is `move`, so without the clone a later use of
//! the binding finds it moved.

#[path = "common/codegen.rs"]
mod codegen;

use codegen::{assert_has, generated_with};

const CHILDREN: &str = "#[prop]\nlet children: Snippet = Snippet::empty();";

#[test]
fn children_read_by_a_tag_inside_an_if_are_cloned_before_its_builder() {
    // When: children forwarded into a Card inside an {if}, then rendered again
    let code = generated_with(
        CHILDREN,
        "{if true}<Card title=\"a\">{render children()}</Card>{/if}{render children()}",
    );

    // Then: within the {if}'s call, the clone precedes its builder closure
    let call = code
        .find("vapors_dom::blocks::conditional(")
        .expect("the if");
    let clone = call
        + code[call..]
            .find("let children = children.clone();")
            .expect("cloned");
    let builder = call
        + code[call..]
            .find("dom: &Rc<RefCell<D>>")
            .expect("the if's builder");
    assert!(clone < builder, "{code}");
}

#[test]
fn a_builder_that_needs_clones_is_valid_rust() {
    // When: a plain binding read in an {if} branch wraps its builder in
    // a block holding the clone
    let code = generated_with(
        "#[prop]\nlet label: String = String::new();",
        "{if true}<p>{label}</p>{/if}<p>{label}</p>",
    );

    // Then
    assert_has(&code, "let label = label.clone();");
    syn::parse_file(&code).unwrap_or_else(|err| panic!("{err}\n{code}"));
}
