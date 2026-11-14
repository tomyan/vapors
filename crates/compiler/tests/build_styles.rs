//! `compile_dir` reads each `<style>` with vapors-css and refuses what
//! the runtime would refuse at the first update, naming `file:line:col`
//! in the `.wisp` file: a style that does not parse, and a `:global()`
//! where the scoping cannot honour it.

#[path = "common/build.rs"]
mod build;

use std::path::Path;

use build::{dirs, write};
use vapors_compiler::build::{Styles, compile_dir_into_styled};

/// Compiles `card.wisp` holding `text`, expecting one refusal, which
/// is returned as `file:line:col: message`.
fn one_refusal(name: &str, wisp: &str, styles: Styles) -> (String, String) {
    let (src, out) = dirs(name);
    write(&src, "card.wisp", wisp);
    let errors = compile_dir_into_styled(&src, &out, styles).expect_err("refused");
    assert_eq!(errors.len(), 1, "{errors:?}");
    (
        src.join("card.wisp").display().to_string(),
        errors[0].to_string(),
    )
}

fn compiles(name: &str, wisp: &str, styles: Styles) {
    let (src, out) = dirs(name);
    write(&src, "card.wisp", wisp);
    compile_dir_into_styled(&src, &out, styles).unwrap_or_else(|errs| panic!("{errs:?}"));
}

#[test]
fn a_style_that_does_not_parse_fails_the_build_at_its_line_in_the_file() {
    // Given: the style's third line is the file's fifth
    let text =
        "<script>\n</script>\n<style>\np { color: red; }\np { colr: red; }\n</style>\n<p>Card</p>";

    // When
    let (file, refusal) = one_refusal("style_unparsed", text, Styles::Scoped);

    // Then
    assert!(refusal.starts_with(&format!("{file}:5:5: ")), "{refusal}");
    assert!(refusal.contains("colr"), "{refusal}");
}

#[test]
fn a_refusal_on_the_styles_first_line_is_offset_by_the_open_tag() {
    // Given: `<style>` is 7 characters; the property starts at column 5 of the CSS
    let text = "<style>p { colr: red; }</style>\n<p>Card</p>";

    // When
    let (file, refusal) = one_refusal("style_first_line", text, Styles::Scoped);

    // Then
    assert!(refusal.starts_with(&format!("{file}:1:12: ")), "{refusal}");
}

#[test]
fn a_nested_global_fails_a_scoped_build_naming_the_fix() {
    // Given
    let text = "<style>\n.x:is(:global(.y)) { color: red; }\n</style>\n<p>Card</p>";

    // When
    let (file, refusal) = one_refusal("style_nested_global", text, Styles::Scoped);

    // Then
    assert!(refusal.starts_with(&format!("{file}:2:1: ")), "{refusal}");
    assert!(
        refusal.contains("leaves a top-level compound unscoped"),
        "{refusal}"
    );
}

#[test]
fn a_top_level_global_compiles_in_a_scoped_build() {
    // Given
    let text = "<style>\n:global(body) { margin: 0; }\n.x :global(p) { color: red; }\n</style>\n<p>Card</p>";

    // When / Then
    compiles("style_top_level_global", text, Styles::Scoped);
}

#[test]
fn any_global_fails_a_global_build_naming_the_fix() {
    // Given: what `add_stylesheet` refuses on the sheet the app adds
    let text = "<style>\np { color: red; }\n:global(body) { margin: 0; }\n</style>\n<p>Card</p>";

    // When
    let (file, refusal) = one_refusal("style_global_build", text, Styles::Global);

    // Then
    assert!(refusal.starts_with(&format!("{file}:3:1: ")), "{refusal}");
    assert!(refusal.contains("component's <style>"), "{refusal}");
}

#[test]
fn a_form_feed_or_lone_carriage_return_in_a_style_is_refused_at_its_place() {
    // Given: CSS would count the form feed as a line break; the file does not
    let text = "<style>\np { color: red; }\x0cp { colr: red; }\n</style>\n<p>Card</p>";

    // When
    let (file, refusal) = one_refusal("style_form_feed", text, Styles::Scoped);

    // Then
    assert!(refusal.starts_with(&format!("{file}:2:18: ")), "{refusal}");
    assert!(refusal.contains("form feed"), "{refusal}");
    let crlf = "<style>\r\np { color: red; }\r\n</style>\r\n<p>Card</p>";
    compiles("style_crlf", crlf, Styles::Scoped);
}

#[test]
fn a_lone_carriage_return_in_a_style_is_refused_at_its_place() {
    // Given: old-Mac line endings; CSS would count the CR as a line break
    let text = "<style>\np { color: red; }\rp { colr: red; }\n</style>\n<p>Card</p>";

    // When
    let (file, refusal) = one_refusal("style_lone_cr", text, Styles::Scoped);

    // Then
    assert!(refusal.starts_with(&format!("{file}:2:18: ")), "{refusal}");
    assert!(refusal.contains("lone carriage return"), "{refusal}");
}

#[test]
fn wisp_check_refuses_a_style_that_does_not_parse() {
    // Given
    let (src, _) = dirs("style_wisp_check");
    write(
        &src,
        "card.wisp",
        "<style>\np { colr: red; }\n</style>\n<p>Card</p>",
    );

    // When
    let error = vapors_compiler::parse_file(&src.join("card.wisp")).expect_err("refused");

    // Then
    assert_eq!((error.line, error.col), (2, 5), "{error}");
    assert_eq!(Path::new(&error.path), src.join("card.wisp"));
}
