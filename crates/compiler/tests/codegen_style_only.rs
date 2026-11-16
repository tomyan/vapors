//! A component with a `<style>` and no markup: its mount registers its
//! sheet and builds nothing, how an app adds a compiled sheet where one
//! from Rust beside analysed elements is refused (R1b). A file with
//! neither markup nor a style is still refused.

use crate::common_codegen as codegen;

use codegen::{assert_has, assert_lacks, generated, refusal};

#[test]
fn a_style_only_component_registers_its_sheet_and_builds_nothing() {
    // When
    let code = generated("<style>\n:global(body) { margin: 0; }\n</style>\n");

    // Then
    assert_has(
        &code,
        "dom.borrow_mut().add_style(vapors_dom::CompiledSheet::__compiled(SCOPE, STYLE,",
    );
    assert_lacks(&code, "create_keyed_element");
    assert_lacks(&code, "create_element");
}

#[test]
fn a_file_with_no_markup_and_no_style_is_refused() {
    // When
    let refused = refusal("<script>\nlet a: Signal<i32> = Signal::new(0);\n</script>\n");

    // Then
    assert!(
        refused
            .to_string()
            .contains("needs a template or a <style>"),
        "{refused}"
    );
}
