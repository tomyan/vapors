//! What the compiler refuses or records in a `StyleKey` beyond what
//! the element says: the site carries the package and the column, so
//! two crates' files and two elements on one line are told apart,
//! and an element with more `class:` directives than a tree keeps as
//! bits is refused, naming it.

#[path = "common/codegen.rs"]
mod codegen;

use codegen::{assert_has, generated, refusal};

#[test]
fn the_site_names_the_line_and_column_of_each_element() {
    // When
    let code = generated("<p><b>a</b><i>b</i></p>");

    // Then
    assert_has(
        &code,
        "site: concat!(env!(\"CARGO_PKG_NAME\"), \"/\", \"counter.wisp:1:4\"), scope: None, tag: \"b\"",
    );
    assert_has(
        &code,
        "site: concat!(env!(\"CARGO_PKG_NAME\"), \"/\", \"counter.wisp:1:12\"), scope: None, tag: \"i\"",
    );
}

/// A `<div>` with `count` directives, each on the signal `on`.
fn directives(count: usize) -> String {
    let each: Vec<String> = (0..count)
        .map(|index| format!("class:d{index}={{on.get()}}"))
        .collect();
    format!(
        "<script>\nlet on: Signal<bool> = Signal::new(false);\n</script>\n<div {}></div>",
        each.join(" ")
    )
}

#[test]
fn an_element_with_more_directives_than_fit_is_refused_naming_it() {
    // When
    let err = refusal(&directives(vapors_compiler::MAX_CLASS_DIRECTIVES + 1));

    // Then
    assert_eq!(
        err.to_string(),
        "counter.wisp:4:1: <div> has 65 `class:` directives; at most 64 fit one element, so move some to a child element"
    );
}

#[test]
fn an_element_with_as_many_directives_as_fit_compiles() {
    // When
    let code = generated(&directives(vapors_compiler::MAX_CLASS_DIRECTIVES));

    // Then
    assert_has(&code, "\"d63\"");
}
