//! A component tag inside a snippet: the snippet's body is built against
//! `dyn Dom`, so a generated component of the same directory is mounted
//! with `::<dyn Dom>` (rustc would take the enclosing mount's `D` from
//! its where clause). A tag the compiler cannot resolve keeps the plain
//! call, which a hand-written mount (`&impl DynDom`, the scene tags)
//! needs. Outside a snippet both keep the plain call.

use crate::common_build::read;
use crate::common_codegen::{assert_has, assert_lacks};
use crate::common_props::{USE_PLAIN, shown, with_plain};
use vapors_compiler::build::compile_dir_into;

const LIST: &str = "<script>\n#[prop]\nlet row: Snippet<(i32,)> = Snippet::empty();\n</script>\n\n<ul>{render row(1)}</ul>";

/// The generated page whose template is `template`, beside `plain.wisp`
/// and `list.wisp` (a `row` snippet prop).
fn page(test: &str, template: &str) -> String {
    let (src, out) = with_plain(test);
    crate::common_build::write(&src, "list.wisp", LIST);
    let script = USE_PLAIN.replace(
        "</script>",
        "use crate::list::{List, ListProps};\n</script>",
    );
    crate::common_build::write(&src, "page.wisp", &format!("{script}{template}"));
    let errors = shown(compile_dir_into(&src, &out));
    assert_eq!(errors, Vec::<String>::new());
    read(&out.join("wisp/page.rs"))
}

#[test]
fn a_component_of_the_directory_in_a_snippet_is_mounted_on_dyn_dom() {
    // When
    let code = page(
        "snippet_mount_generated",
        "<List>{snippet row(n: i32)}<Plain title=\"a\" />{/snippet}</List>",
    );

    // Then
    assert_has(&code, "Plain::mount_before::<dyn Dom");
}

#[test]
fn a_tag_the_compiler_cannot_resolve_in_a_snippet_keeps_the_plain_call() {
    // When
    let code = page(
        "snippet_mount_unresolved",
        "<List>{snippet row(n: i32)}<Mesh />{/snippet}</List>",
    );

    // Then
    assert_has(&code, "Mesh::mount_before(dom,");
}

#[test]
fn a_component_outside_a_snippet_keeps_the_plain_call() {
    // When
    let code = page("snippet_mount_outside", "<Plain title=\"a\" />");

    // Then
    assert_has(&code, "Plain::mount_before(dom,");
    assert_lacks(&code, "::<dyn Dom>");
}
