//! `build::compile_dir_into` checks a component tag's props against the
//! component it names, when that component is in the same directory
//! (1-R1 review, finding 3): an attribute, children, or a snippet in the
//! tag that the component does not declare is a `.wisp` error at the
//! tag, naming the tag, the prop and the component's file.

#[path = "common/build.rs"]
mod build;

use build::{dirs, write};
use vapors_compiler::build::compile_dir_into;

const PLAIN: &str =
    "<script>\n#[prop]\nlet title: String = String::new();\n</script>\n\n<h2>{title}</h2>";

/// The refusals of a directory holding `plain.wisp` and a parent whose
/// template is `parent`.
fn refusals(name: &str, parent: &str) -> Vec<String> {
    let (src, out) = dirs(name);
    write(&src, "plain.wisp", PLAIN);
    let script = "<script>\nuse crate::plain::{Plain, PlainProps};\n</script>\n\n";
    write(&src, "page.wisp", &format!("{script}{parent}"));
    match compile_dir_into(&src, &out) {
        Ok(_) => Vec::new(),
        Err(errors) => errors.iter().map(ToString::to_string).collect(),
    }
}

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
fn a_name_two_components_share_is_not_checked() {
    // Given: two `Plain`s, in two subdirectories; the tag cannot say which
    let (src, out) = dirs("shared_name");
    write(&src, "a/plain.wisp", PLAIN);
    write(&src, "b/plain.wisp", PLAIN);
    write(&src, "page.wisp", "<Plain titel=\"a\" />");

    // When
    let compiled = compile_dir_into(&src, &out);

    // Then: left to rustc, as for a component outside the directory
    assert!(compiled.is_ok(), "{:?}", compiled.err());
}
