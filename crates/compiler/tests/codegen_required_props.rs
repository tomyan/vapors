//! A `#[prop]` declared without a default is required: the props struct
//! holds it as `Required<T>`, `Missing` by default, and the mount takes
//! it out, refusing a missing one naming the component and the prop. A
//! tag that passes it wraps the value in `Required::Given`.

use crate::common_codegen::{assert_has, assert_lacks, generated_with, refusal};

const REQUIRED: &str = "#[prop]\nlet label: String;";

#[test]
fn a_prop_without_a_default_is_a_required_field() {
    // When
    let code = generated_with(REQUIRED, "<span>{label}</span>");

    // Then
    assert_has(&code, "pub label: Required<String>,");
    assert_has(&code, "label: Required::Missing,");
}

#[test]
fn the_mount_refuses_a_missing_one_naming_component_and_prop() {
    // When
    let code = generated_with(REQUIRED, "<span>{label}</span>");

    // Then
    assert_has(&code, "let label = label.take(\"Counter\", \"label\", ");
    assert_lacks(&code, "label: String::new()");
}

#[test]
fn a_prop_with_a_default_is_not_wrapped() {
    // When
    let code = generated_with(
        "#[prop]\nlet label: String = String::new();",
        "<b>{label}</b>",
    );

    // Then
    assert_has(&code, "pub label: String,");
    assert_lacks(&code, "Required<");
    assert_lacks(&code, ".take(");
}

#[test]
fn a_prop_without_a_type_is_still_refused() {
    // When
    let error = refusal("<script>\n#[prop]\nlet label;\n</script>\n<b></b>");

    // Then
    assert!(error.message.contains("needs a type"), "{error}");
}

#[test]
fn mount_before_consumes_its_props_whole_so_no_allowance_is_written() {
    // When
    let code = generated_with("#[prop]\nlet n: usize = 0;", "<b>{n}</b>");

    // Then
    assert_has(&code, "let (CounterProps { n },) = (props,);");
    assert_lacks(&code, "needless_pass_by_value");
}
