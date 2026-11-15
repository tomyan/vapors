//! `<rect>` and `<circle>` attributes the build can see are refused at
//! build time, naming the file, line and column (one-scene S4b): an
//! attribute the shape does not take, a literal geometry value the
//! runtime would refuse, and a required one that is absent. A bound
//! value is left to the update.

#[path = "common/codegen.rs"]
mod codegen;

use codegen::refusal;

const SVG: &str = "<svg viewBox=\"0 0 16 16\" width=\"16\" height=\"16\" aria-hidden=\"true\">";

/// The refusal of `shape` in an svg, under a script with a string
/// signal `width`; the template starts on line 4.
fn refused(shape: &str) -> String {
    refusal(&format!(
        "<script>\nlet width: Signal<String> = Signal::new(String::new());\n</script>\n{SVG}{shape}</svg>"
    ))
    .to_string()
}

#[test]
fn a_literal_geometry_value_the_runtime_refuses_fails_the_build_naming_its_column() {
    // Given: a unit, a percentage, a size of 0, a negative radius
    let cases = [
        (
            "<rect width=\"12px\" height=\"8\"/>",
            "counter.wisp:4:74: <rect>: its width \"12px\" is not a number above 0 with no unit",
        ),
        (
            "<rect x=\"10%\" width=\"12\" height=\"8\"/>",
            "counter.wisp:4:74: <rect>: its x \"10%\" is not a number with no unit",
        ),
        (
            "<circle r=\"0\"/>",
            "counter.wisp:4:76: <circle>: its r \"0\" is not a number above 0",
        ),
        (
            "<rect width=\"4\" height=\"4\" rx=\"-1\"/>",
            "counter.wisp:4:95: <rect>: its rx \"-1\" is not a number of 0 or more",
        ),
    ];

    for (shape, expected) in cases {
        // When
        let err = refused(shape);

        // Then
        assert!(err.starts_with(expected), "{shape}: {err}");
    }
}

#[test]
fn a_required_size_absent_fails_the_build_at_the_element() {
    // When
    let err = refused("<circle cx=\"8\"/>");

    // Then
    assert!(
        err.starts_with("counter.wisp:4:68: <circle>: its r attribute is required"),
        "{err}"
    );
}

#[test]
fn an_attribute_a_shape_does_not_take_fails_the_build() {
    for (shape, name) in [
        (
            "<rect width=\"4\" height=\"4\" transform=\"scale(2)\"/>",
            "transform",
        ),
        ("<circle r=\"2\" class=\"dot\"/>", "class"),
    ] {
        // When
        let err = refused(shape);

        // Then
        assert!(
            err.contains(&format!("`{name}` is not an attribute of <")),
            "{shape}: {err}"
        );
    }
}

#[test]
fn a_bound_geometry_value_is_left_to_the_update() {
    // When
    let code = vapors_compiler::compile_str(
        "counter.wisp",
        &format!(
            "<script>\nlet width: Signal<String> = Signal::new(String::new());\n</script>\n{SVG}<rect width={{width.get()}} height=\"4\" fill=\"red\"/></svg>"
        ),
    );

    // Then
    assert!(code.is_ok(), "{code:?}");
}
