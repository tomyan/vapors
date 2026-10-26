//! Which component a tag names, for the props check (review-v67, 2–3):
//! the tag's own path, or the script's `use` and its alias, resolved to
//! a module under the directory (whose `wisp.rs` is included at the
//! crate root). A tag that resolves to no component in the directory
//! is not checked, and is listed as unchecked with its file and line.

#[path = "common/build.rs"]
mod build;
#[path = "common/props.rs"]
mod props;

use build::write;
use props::{USE_PLAIN, shown, with_plain};
use vapors_compiler::build::compile_dir_into;

#[test]
fn a_tag_naming_another_crates_component_is_listed_unchecked() {
    // Given: a local `plain.wisp`, and a tag naming another crate's Plain
    let (src, out) = with_plain("other_crate");
    write(&src, "page.wisp", "<other_crate::Plain big=\"x\" />");

    // When
    let compiled = compile_dir_into(&src, &out).expect("compiles");

    // Then
    assert_eq!(compiled.unchecked.len(), 1, "{:?}", compiled.unchecked);
    let note = &compiled.unchecked[0];
    assert!(
        note.contains("page.wisp:1:1") && note.contains("other_crate::Plain"),
        "{note}"
    );
}

#[test]
fn a_tag_without_a_use_is_listed_unchecked() {
    // Given
    let (src, out) = with_plain("no_use");
    write(&src, "page.wisp", "<Plain big=\"x\" />");

    // When
    let compiled = compile_dir_into(&src, &out).expect("compiles");

    // Then
    assert!(
        compiled.unchecked[0].contains("`<Plain>`"),
        "{:?}",
        compiled.unchecked
    );
}

#[test]
fn a_tag_under_an_alias_is_checked_against_the_component_the_use_names() {
    // Given: `Card` is plain.wisp's `Plain`; card.wisp declares no `title`
    let (src, out) = with_plain("alias");
    write(
        &src,
        "card.wisp",
        "<script>\n#[prop]\nlet heading: String = String::new();\n</script>\n<h1>{heading}</h1>",
    );
    let script =
        "<script>\nuse crate::plain::{Plain as Card, PlainProps as CardProps};\n</script>\n";

    // When
    write(&src, "page.wisp", &format!("{script}<Card title=\"a\" />"));
    let ok = shown(compile_dir_into(&src, &out));
    write(
        &src,
        "page.wisp",
        &format!("{script}<Card heading=\"a\" />"),
    );
    let refused = shown(compile_dir_into(&src, &out));

    // Then
    assert_eq!(ok, Vec::<String>::new());
    assert!(
        refused[0].contains("`heading`") && refused[0].contains("plain.wisp"),
        "{refused:?}"
    );
}

#[test]
fn a_name_two_components_share_is_checked_against_the_one_the_path_names() {
    // Given: two `Plain`s; `a`'s declares `title`, `b`'s declares `label`
    let (src, out) = build::dirs("shared_name");
    write(&src, "a/plain.wisp", props::PLAIN);
    write(
        &src,
        "b/plain.wisp",
        "<script>\n#[prop]\nlet label: String = String::new();\n</script>\n<p>{label}</p>",
    );
    write(
        &src,
        "page.wisp",
        "<script>\nuse crate::b::plain::{Plain, PlainProps};\n</script>\n<Plain title=\"a\" />",
    );

    // When
    let errors = shown(compile_dir_into(&src, &out));

    // Then
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(
        errors[0].contains("`title`") && errors[0].contains("b/plain.wisp"),
        "{errors:?}"
    );
}

#[test]
fn self_and_super_paths_resolve_from_the_tags_own_module() {
    // Given: pages in a subdirectory naming plain.wisp up one module
    let (src, out) = with_plain("super_path");
    write(
        &src,
        "sub/page.wisp",
        "<super::super::plain::Plain titel=\"a\" />",
    );
    write(
        &src,
        "sub/other.wisp",
        "<script>\nuse super::super::plain::{Plain, PlainProps};\n</script>\n<Plain titel=\"b\" />",
    );

    // When
    let errors = shown(compile_dir_into(&src, &out));

    // Then: `sub/page.wisp` is `crate::sub::page`, so `super::super` is the crate
    assert_eq!(errors.len(), 2, "{errors:?}");
    assert!(errors.iter().all(|e| e.contains("`titel`")), "{errors:?}");
}

#[test]
fn a_crate_path_to_a_checked_component_is_checked() {
    // When
    let errors = shown({
        let (src, out) = with_plain("crate_path");
        write(
            &src,
            "page.wisp",
            &format!("{USE_PLAIN}<crate::plain::Plain titel=\"a\" />"),
        );
        compile_dir_into(&src, &out)
    });

    // Then
    assert!(errors[0].contains("`titel`"), "{errors:?}");
}
