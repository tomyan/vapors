//! `build::compile_dir_into` checks a component tag's props against the
//! component it names, when that component is in the same directory
//! (1-R1 review, finding 3): an attribute, children, or a snippet in the
//! tag that the component does not declare is a `.wisp` error at the
//! attribute, the snippet or the tag, naming the tag, the prop and the
//! component's file, with a declaration that compiles.

#[path = "common/build.rs"]
mod build;
#[path = "common/props.rs"]
mod props;

use props::refusals;

#[test]
fn children_for_a_component_without_a_children_prop_are_refused_at_the_tag() {
    // When
    let errors = refusals(
        "children_refused",
        "<div><Plain title=\"a\"><p>body</p></Plain></div>",
    );

    // Then
    assert_eq!(errors.len(), 1, "{errors:?}");
    let error = &errors[0];
    assert!(error.contains("page.wisp:5:6"), "{error}");
    assert!(
        error.contains("<Plain>") && error.contains("children") && error.contains("plain.wisp"),
        "{error}"
    );
    assert!(
        error.contains("#[prop] let children: Snippet = Snippet::empty();"),
        "{error}"
    );
}

#[test]
fn an_attribute_the_component_does_not_declare_is_refused_naming_its_props() {
    // When
    let errors = refusals("attribute_refused", "<Plain titel=\"a\" />");

    // Then
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(
        errors[0].contains("titel") && errors[0].contains("title"),
        "{}",
        errors[0]
    );
}

#[test]
fn a_snippet_in_the_tag_needs_a_prop_of_its_name() {
    // When
    let errors = refusals(
        "snippet_refused",
        "<Plain title=\"a\">{snippet row(n: i32)}<b>{n}</b>{/snippet}</Plain>",
    );

    // Then
    assert!(
        errors
            .iter()
            .any(|e| e.contains("row") && e.contains("plain.wisp")),
        "{errors:?}"
    );
}

#[test]
fn declared_props_compile() {
    // When
    let errors = refusals("declared", "<Plain title=\"a\" />");

    // Then
    assert_eq!(errors, Vec::<String>::new());
}

#[test]
fn a_snippet_prop_is_suggested_with_the_snippets_parameter_types() {
    // When
    let errors = refusals(
        "snippet_suggested",
        "<Plain title=\"a\">{snippet row(n: i32, name: String)}<b>{n}</b>{/snippet}</Plain>",
    );

    // Then: at the snippet
    let error = errors
        .iter()
        .find(|e| e.contains("`row`"))
        .expect("row refused");
    assert!(error.contains("page.wisp:5:18"), "{error}");
    assert!(
        error.contains("#[prop] let row: Snippet<(i32, String)> = Snippet::empty();"),
        "{error}"
    );
}

#[test]
fn a_value_prop_is_suggested_with_a_type_inferred_from_a_literal() {
    // When
    let text = refusals("text_suggested", "<Plain title=\"a\" titel=\"b\" />");
    let number = refusals("number_suggested", "<Plain title=\"a\" count={3} />");
    let unknown = refusals("unknown_suggested", "<Plain title=\"a\" when={now()} />");

    // Then: the unknown type is a placeholder that compiles, marked
    assert!(
        text[0].contains("#[prop] let titel: String = String::new();"),
        "{}",
        text[0]
    );
    assert!(
        number[0].contains("#[prop] let count: i32 = 0;"),
        "{}",
        number[0]
    );
    assert!(
        unknown[0].contains("#[prop] let when: () = ();") && unknown[0].contains("replace `()`"),
        "{}",
        unknown[0]
    );
}

#[test]
fn every_undeclared_prop_is_refused_at_its_attribute() {
    // When
    let errors = refusals(
        "every_prop",
        "<Plain x=\"1\" title=\"a\" y=\"2\" />\n<Plain z=\"3\" />",
    );

    // Then
    assert_eq!(errors.len(), 3, "{errors:?}");
    assert!(
        errors[0].contains("page.wisp:5:8") && errors[0].contains("`x`"),
        "{errors:?}"
    );
    assert!(
        errors[1].contains("page.wisp:5:24") && errors[1].contains("`y`"),
        "{errors:?}"
    );
    assert!(
        errors[2].contains("page.wisp:6:8") && errors[2].contains("`z`"),
        "{errors:?}"
    );
}

#[test]
fn a_bound_prop_is_checked_by_its_name_without_bind() {
    // When
    let declared = refusals(
        "bind_declared",
        "<script>\nlet t: Signal<String> = Signal::new(String::new());\n</script>\n<Plain bind:title={t} />",
    );
    let undeclared = refusals("bind_undeclared", "<Plain bind:titel={t} />");

    // Then
    assert!(
        !declared.iter().any(|e| e.contains("does not declare")),
        "{declared:?}"
    );
    assert!(undeclared[0].contains("`titel`"), "{undeclared:?}");
}

#[test]
fn tags_inside_blocks_and_snippets_are_checked() {
    // When
    let in_if = refusals("in_if", "{if true}<Plain titel=\"a\" />{/if}");
    let in_for = refusals("in_for", "{for x in 0..2}<Plain titel=\"a\" />{/for}");
    let in_children = refusals(
        "in_children",
        "<Plain title=\"a\"><Plain titel=\"b\" /></Plain>",
    );

    // Then
    assert!(in_if.iter().any(|e| e.contains("`titel`")), "{in_if:?}");
    assert!(in_for.iter().any(|e| e.contains("`titel`")), "{in_for:?}");
    assert!(
        in_children.iter().any(|e| e.contains("`titel`")),
        "{in_children:?}"
    );
}
