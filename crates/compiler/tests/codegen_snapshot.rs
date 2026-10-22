//! The generated code for the worked counter component, pinned with
//! insta so a change to the output is reviewed as a diff. Read the
//! snapshot: it is what a person steps through in the debugger.

use vapors_compiler::compile_str;

#[test]
fn counter_component_generates_readable_rust() {
    // Given
    let wisp = include_str!("../../compiler-examples/src/counter.wisp");

    // When
    let code = compile_str("src/counter.wisp", wisp).unwrap_or_else(|err| panic!("{err}"));

    // Then
    insta::assert_snapshot!(code);
}
