//! Which longhands a declaration reached (`src/cascade/declared.rs`):
//! declared on the element, or on an ancestor for an inherited
//! property. A longhand no declaration reached is at its initial value,
//! which lets a reader skip comparing it with the initial.

mod support;

use support::{chain, computed};
use vapors_css::property::PropertyId;

fn id(name: &str) -> PropertyId {
    PropertyId::from_name(name).expect("a longhand")
}

/// Whether `property` is declared at the leaf of `descs` under `src`.
fn declared_at(src: &str, descs: &[&str], property: &str) -> bool {
    let (tree, leaf) = chain(descs);
    computed(src, &tree, leaf).declared(id(property))
}

#[test]
fn a_declared_longhand_is_declared_and_an_undeclared_one_is_not() {
    let src = ".c { width: 10px; }";
    assert_eq!(
        (
            declared_at(src, &["div.c"], "width"),
            declared_at(src, &["div.c"], "height")
        ),
        (true, false)
    );
}

#[test]
fn an_inherited_property_declared_on_the_parent_is_declared_on_the_child() {
    let src = ".p { color: red; }";
    assert!(declared_at(src, &["div.p", "span.c"], "color"));
}

#[test]
fn a_non_inherited_property_declared_on_the_parent_is_not_declared_on_the_child() {
    let src = ".p { width: 10px; }";
    assert!(!declared_at(src, &["div.p", "span.c"], "width"));
}

#[test]
fn inherit_on_a_non_inherited_property_declares_it() {
    let src = ".p { width: 10px; } .c { width: inherit; }";
    assert!(declared_at(src, &["div.p", "span.c"], "width"));
}

#[test]
fn a_var_winner_declares_the_longhand() {
    let src = ".p { --w: 10px; } .c { width: var(--w); }";
    assert!(declared_at(src, &["div.p", "span.c"], "width"));
}

#[test]
fn a_declaration_three_ancestors_up_reaches_an_inherited_property() {
    let src = ".a { letter-spacing: 1px; }";
    assert!(declared_at(
        src,
        &["div.a", "div.b", "div.c", "span.d"],
        "letter-spacing"
    ));
}
