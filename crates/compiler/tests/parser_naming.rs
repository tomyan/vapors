//! Translated from the applicable part of sumi's
//! `parser/template/tagname_test.go`: the file-name to component-name
//! rule, plus the refusal of an unknown lowercase tag.

mod common;

use common::parse_err;
use vapors_compiler::component_name;

// tagname_test.go: the file-name to component-name rule. sumi gives
// `split-panel.sumi` the name `Splitpanel`; vapors follows Rust type
// naming and gives `split-panel.wisp` the name `SplitPanel`.

#[test]
fn component_name_from_file_name() {
    for (path, want) in [
        ("counter.wisp", "Counter"),
        ("src/widgets/text-input.wisp", "TextInput"),
        ("split_panel.wisp", "SplitPanel"),
        ("my-widget.wisp", "MyWidget"),
    ] {
        assert_eq!(component_name(std::path::Path::new(path)).unwrap(), want);
    }
}

#[test]
fn component_name_refuses_a_name_that_is_not_an_identifier() {
    // When
    let err = component_name(std::path::Path::new("2fast.wisp")).unwrap_err();

    // Then
    assert!(err.message.contains("2fast"), "{err}");
}

#[test]
fn unknown_lowercase_tag_is_refused_with_both_fixes() {
    // When
    let err = parse_err("<div>\n  <counter />\n</div>");

    // Then
    assert_eq!((err.line, err.col), (2, 3), "{err}");
    assert!(err.message.contains("counter"), "{err}");
    assert!(
        err.message.contains("Counter"),
        "names the capitalised form: {err}"
    );
}

#[test]
fn a_file_name_that_makes_a_keyword_type_name_is_refused() {
    // When: `self.wisp` would be `Self`
    let err = component_name(std::path::Path::new("src/self.wisp")).expect_err("Self is a keyword");

    // Then
    assert_eq!(err.path, "src/self.wisp");
    assert!(err.message.contains("`Self`"), "{err}");
}
