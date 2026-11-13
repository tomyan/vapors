//! Where each longhand's computed value comes from
//! (`src/cascade/compute_source.rs`, `Source`): a declared value resolved, the parent's computed value
//! copied as it is, the initial value resolved, or a `var()` winner
//! substituted and resolved. The copies are what tell the paths apart:
//! a parent's computed value is already absolute, so a child takes it
//! without resolving it against its own font size.

mod support;

use support::css_at;

#[test]
fn a_declared_value_is_resolved_against_the_elements_own_font_size() {
    assert_eq!(
        css_at(".c { font-size: 10px; width: 2em; }", &["div.c"], "width"),
        "20px"
    );
}

#[test]
fn an_inherited_property_without_a_declaration_copies_the_parents_computed_value() {
    // Given: the parent's line-height computes to 15px; the child's font
    // size differs, so resolving 150% again would give 30px
    let src = ".p { font-size: 10px; line-height: 150%; } .c { font-size: 20px; }";

    // Then
    assert_eq!(css_at(src, &["div.p", "span.c"], "line-height"), "15px");
}

#[test]
fn inherit_copies_the_parents_computed_value_of_a_non_inherited_property() {
    let src = ".p { font-size: 10px; border-top-width: 2em; } .c { font-size: 20px; border-top-width: inherit; }";
    assert_eq!(
        css_at(src, &["div.p", "span.c"], "border-top-width"),
        "20px"
    );
}

#[test]
fn unset_copies_the_parent_for_an_inherited_property_and_takes_the_initial_for_another() {
    let src = ".p { font-size: 10px; letter-spacing: 1em; opacity: 0.5; } .c { font-size: 20px; letter-spacing: unset; opacity: unset; }";
    assert_eq!(
        (
            css_at(src, &["div.p", "span.c"], "letter-spacing"),
            css_at(src, &["div.p", "span.c"], "opacity")
        ),
        ("10px".to_owned(), "1".to_owned())
    );
}

#[test]
fn initial_takes_the_initial_value_resolved() {
    let src = ".p { color: red; } .c { color: initial; }";
    assert_eq!(css_at(src, &["div.p", "span.c"], "color"), "rgb(0 0 0)");
}

#[test]
fn a_var_winner_is_substituted_then_resolved() {
    let src = ".p { --w: 2em; } .c { font-size: 10px; width: var(--w); }";
    assert_eq!(css_at(src, &["div.p", "span.c"], "width"), "20px");
}

#[test]
fn a_var_fallback_of_inherit_copies_the_parent() {
    // Given: CSS Variables 1 section 2.3, a fallback used when the
    // custom property is not set; a CSS-wide keyword in a custom
    // property's own value is issue #300
    let src = ".p { font-size: 10px; border-top-width: 2em; } .c { font-size: 20px; border-top-width: var(--missing, inherit); }";
    assert_eq!(
        css_at(src, &["div.p", "span.c"], "border-top-width"),
        "20px"
    );
}

#[test]
fn a_calc_that_is_not_finite_is_refused_naming_it() {
    // Given: infinity minus infinity
    let src = ".c { line-height: calc(1e308 * 10 - 1e308 * 10); }";
    let (tree, leaf) = support::chain(&["div.c"]);

    // When
    let error = support::cascade(src)
        .compute(&tree.node(leaf), None, &support::env())
        .expect_err("refused");

    // Then
    assert!(
        error.to_string().contains("line-height") && error.to_string().contains("finite"),
        "{error}"
    );
}
