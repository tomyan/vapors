//! Which files may name a crate, and which do.

use std::collections::BTreeMap;

use super::*;

fn allow(paths: &[&str]) -> BTreeMap<String, String> {
    paths
        .iter()
        .map(|p| ((*p).to_owned(), "a reason".to_owned()))
        .collect()
}

#[test]
fn a_name_in_code_is_found_on_its_line() {
    let text = "fn a() {}\nlet w = dom_api::window();\n";
    assert_eq!(first_use(text, "dom_api"), Some(2));
}

#[test]
fn a_name_in_a_comment_is_not_a_use() {
    let text = "// dom_api is the browser API\n    /// see dom_api\n";
    assert_eq!(first_use(text, "dom_api"), None);
}

#[test]
fn a_name_inside_another_word_is_not_a_use() {
    let text = "let x = my_dom_api_thing;\nlet y = dom_apis;\n";
    assert_eq!(first_use(text, "dom_api"), None);
}

#[test]
fn a_directory_entry_allows_everything_under_it_and_a_file_entry_only_itself() {
    let rule = allow(&["crates/web/", "crates/runtime/src/key_web.rs"]);
    assert!(allowed("crates/web/src/page.rs", &rule));
    assert!(allowed("crates/runtime/src/key_web.rs", &rule));
    assert!(!allowed("crates/runtime/src/key_web_extra.rs", &rule));
    assert!(!allowed("crates/webby/src/lib.rs", &rule));
}

#[test]
fn a_use_outside_the_allowed_paths_is_named_with_its_line_and_the_allowed_paths() {
    // Given
    let files = [
        (
            "crates/web/src/page.rs".to_owned(),
            "dom_api::window();".to_owned(),
        ),
        (
            "crates/engine/src/lib.rs".to_owned(),
            "\nuse dom_api::Element;".to_owned(),
        ),
    ];
    let rules = BTreeMap::from([("dom_api".to_owned(), allow(&["crates/web/"]))]);

    // When
    let found = violations(&files, &rules);

    // Then
    assert_eq!(found.len(), 1);
    assert!(
        found[0].starts_with("crates/engine/src/lib.rs:2 names dom_api"),
        "{}",
        found[0]
    );
    assert!(found[0].contains("crates/web/"), "{}", found[0]);
}
