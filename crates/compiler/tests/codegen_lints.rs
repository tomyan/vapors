//! The generated file is included into the user's crate, where clippy
//! pedantic runs on it. What the compiler writes itself is clean; the
//! lints that depend on types it cannot see are allowed on the one
//! function they apply to, with the reason in the attribute. The
//! compiled examples crate is the check that the whole file passes.

#[path = "common/codegen.rs"]
mod codegen;

use codegen::{assert_has, assert_lacks, generated, generated_with};

const PROP: &str = "#[prop]\nlet label: String = \"Count\".to_string();";

#[test]
fn an_empty_props_struct_derives_default() {
    // When
    let code = generated("<div></div>");

    // Then
    assert_has(&code, "#[derive(Default)]\npub struct CounterProps {}");
    assert_lacks(&code, "impl Default for CounterProps");
}

#[test]
fn props_defaults_are_written_out_with_the_reason_they_are_not_derived() {
    // When
    let code = generated_with(PROP, "<span>{label}</span>");

    // Then
    assert_has(
        &code,
        "#[allow(clippy::derivable_impls, reason = \"the defaults are the #[prop] initialisers as written in the .wisp\")]\nimpl Default for CounterProps",
    );
}

#[test]
fn mount_before_allows_cloning_a_binding_that_may_be_copy() {
    // When
    let code = generated("<div></div>");

    // Then
    assert_has(&code, "clippy::clone_on_copy");
    assert_has(
        &code,
        "reason = \"a closure gets a clone of each binding it captures; the compiler cannot see which are Copy\"",
    );
}

#[test]
fn empty_props_are_taken_by_value_with_the_reason() {
    // When
    let code = generated("<div></div>");

    // Then
    assert_has(&code, "clippy::needless_pass_by_value");
    assert_has(
        &code,
        "every component takes its props struct by value, empty or not, so parents mount them alike",
    );
}

#[test]
fn props_with_fields_are_consumed_and_need_no_allowance() {
    // When
    let code = generated_with(PROP, "<span>{label}</span>");

    // Then
    assert_lacks(&code, "needless_pass_by_value");
}

#[test]
fn roots_is_must_use() {
    // When
    let code = generated("<div></div>");

    // Then
    assert_has(&code, "#[must_use]\npub fn roots(&self) -> &[NodeId]");
}
