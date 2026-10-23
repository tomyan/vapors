//! The settings parser: tables, keys (bare or quoted), values, duplicates.

use super::*;

#[test]
fn reads_a_value_by_table_and_key() {
    // Given
    let text = "# settings\n[screenshot]\npackage = \"vapors-platform\"\n";

    // When
    let config = Config::parse(text).expect("valid");

    // Then
    assert_eq!(config.require("screenshot.package"), Ok("vapors-platform"));
}

#[test]
fn entries_under_a_prefix_are_keyed_by_the_rest() {
    // Given
    let text = "[files.allow]\ncrates/a.rs = \"a table\"\ncrates/b.rs = \"a list\"\n[files]\nmax_lines = \"220\"\n";
    let config = Config::parse(text).expect("valid");

    // When
    let allow = config.entries_under("files.allow.");

    // Then
    assert_eq!(allow.len(), 2);
    assert_eq!(
        allow.get("crates/a.rs").map(String::as_str),
        Some("a table")
    );
    assert!(config.entries_under("files.generated.").is_empty());
    assert_eq!(config.get("files.max_lines"), Some("220"));
}

#[test]
fn a_quoted_key_is_unquoted_and_a_bare_key_kept() {
    // Given
    let text = "[files.allow]\n\"crates/a b.rs\" = \"a table\"\ncrates/c.rs = \"a list\"\n";

    // When
    let config = Config::parse(text).expect("valid");

    // Then
    assert_eq!(config.get("files.allow.crates/a b.rs"), Some("a table"));
    assert_eq!(config.get("files.allow.crates/c.rs"), Some("a list"));
}

#[test]
fn a_key_with_an_escape_or_a_stray_quote_is_refused_naming_the_line() {
    // Given
    let escaped = "[t]\n\"a\\\"b\" = \"x\"\n";
    let stray = "[t]\nabc\" = \"x\"\n";
    let spaced = "[t]\na b = \"x\"\n";

    // When / Then
    for text in [escaped, stray, spaced] {
        let error = Config::parse(text).expect_err(text);
        assert_eq!(error.line, 2, "{text:?}");
        assert!(
            error.reason.contains("key must be bare"),
            "{text:?}: {}",
            error.reason
        );
    }
}

#[test]
fn a_duplicate_key_is_refused_naming_both_lines() {
    // Given
    let text = "[t]\na = \"1\"\nb = \"2\"\na = \"3\"\n";

    // When
    let error = Config::parse(text).expect_err("duplicate");

    // Then
    assert_eq!(error.line, 4);
    assert_eq!(error.reason, "duplicate key `t.a`; first on line 2");
}

#[test]
fn a_missing_key_says_how_to_add_it() {
    // Given
    let config = Config::parse("[screenshot]\n").expect("valid");

    // When
    let error = config.require("screenshot.package").expect_err("missing");

    // Then
    assert!(error.reason.contains("[screenshot]"));
    assert!(error.reason.contains("package = "));
}

#[test]
fn refuses_an_entry_before_a_header_naming_the_line() {
    // Given
    let text = "package = \"x\"\n";

    // When
    let error = Config::parse(text).expect_err("no header");

    // Then
    assert_eq!(error.line, 1);
}

#[test]
fn a_quoted_key_may_hold_an_equals_sign() {
    // Given: a gate setup line with `=` in it
    let text = "[gate.setup]\n\"fc-list :lang=zh | head -1\" = \"checks the CJK font\"\n";

    // When
    let config = Config::parse(text).expect("valid");

    // Then
    assert_eq!(
        config.get("gate.setup.fc-list :lang=zh | head -1"),
        Some("checks the CJK font")
    );
}
