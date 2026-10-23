//! Names the generated code uses are refused in the script and in loop
//! patterns, at the name's position, rather than shadowing generated
//! locals or failing to compile in `OUT_DIR`. Raw identifiers are refused
//! the same way.

#[path = "common/codegen.rs"]
mod codegen;

use codegen::{assert_has, assert_lacks, generated, generated_with, refusal};

#[test]
fn a_script_binding_named_like_a_generated_parameter_is_refused() {
    // When
    let err = refusal("<script>\nlet x = 1;\nlet dom = 2;\n</script>\n<p>{dom}</p>");

    // Then
    assert_eq!((err.line, err.col), (3, 1), "{err}");
    assert!(
        err.message.contains("`dom`") && err.message.contains("rename"),
        "{err}"
    );
}

#[test]
fn a_name_bound_inside_a_script_pattern_is_checked_too() {
    // When
    let err = refusal("<script>\nlet (roots, x) = (1, 2);\n</script>\n<p>{x}</p>");

    // Then
    assert_eq!(err.line, 2, "{err}");
    assert!(err.message.contains("`roots`"), "{err}");
}

#[test]
fn a_prop_named_props_is_refused() {
    // When
    let err = refusal("<script>\n#[prop]\nlet props: i32 = 0;\n</script>\n<p>{props}</p>");

    // Then
    assert_eq!(err.line, 2, "{err}");
    assert!(err.message.contains("`props`"), "{err}");
}

#[test]
fn a_script_binding_named_like_a_generated_node_is_refused() {
    // When: the <p> below is generated as `p0`
    let err = refusal("<script>\nlet p0 = 5;\n</script>\n<p>{p0}</p>");

    // Then
    assert_eq!((err.line, err.col), (2, 1), "{err}");
    assert!(
        err.message.contains("`p0`") && err.message.contains("generated"),
        "{err}"
    );
}

#[test]
fn a_loop_binding_named_like_a_builder_parameter_is_refused() {
    // Given
    let script = "<script>\nlet items: Signal<Vec<u32>> = Signal::new(Vec::new());\n</script>\n";

    // When
    let err = refusal(&format!(
        "{script}<ul>{{for (parent, x) in items.get()}}<li>{{x}}</li>{{/for}}</ul>"
    ));

    // Then
    assert_eq!(err.line, 4, "{err}");
    assert!(err.message.contains("`parent`"), "{err}");
}

#[test]
fn a_name_close_to_a_generated_one_compiles() {
    // When: only `p0` is generated
    let code = generated_with("let p1 = 5;", "<p>{p1}</p>");

    // Then
    assert_has(&code, "let p1 = 5;");
}

#[test]
fn a_raw_identifier_signal_is_refused_at_its_position() {
    // When
    let err =
        refusal("<script>\nlet r#gen: Signal<i32> = Signal::new(0);\n</script>\n<p>{r#gen}</p>");

    // Then
    assert_eq!((err.line, err.col), (2, 1), "{err}");
    assert!(
        err.message.contains("r#gen") && err.message.contains("raw identifier"),
        "{err}"
    );
}

#[test]
fn a_raw_identifier_prop_is_refused_at_its_position() {
    // When
    let err = refusal("<script>\n#[prop]\nlet r#type: i32 = 0;\n</script>\n<p>x</p>");

    // Then
    assert_eq!(err.line, 2, "{err}");
    assert!(err.message.contains("raw identifier"), "{err}");
}

#[test]
fn node_names_share_one_counter_width_past_ten() {
    // Given: eleven elements, so counters run 0 to 10
    let template = "<p></p>".repeat(11);

    // When
    let code = generated(&template);

    // Then
    assert_has(&code, "let p00 = dom");
    assert_has(&code, "let p10 = dom");
    assert_lacks(&code, "let p0 = dom");
}

#[test]
fn up_to_ten_names_keep_one_digit() {
    // When
    let code = generated(&"<p></p>".repeat(10));

    // Then
    assert_has(&code, "let p0 = dom");
    assert_has(&code, "let p9 = dom");
}
