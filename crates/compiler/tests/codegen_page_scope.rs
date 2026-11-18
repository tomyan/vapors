//! What a page's generated file does with the script's `use` items and
//! where its script code came from (issue #508).

use crate::common_codegen as codegen;

use codegen::{assert_has, generated, refusal};

const PAGE: &str = "<script>\nuse crate::taps::Faces;\n\n#[prop]\nlet faces: std::rc::Rc<Faces> = Default::default();\n</script>\n<p>x</p>";

#[test]
fn a_use_is_at_the_top_level_before_the_props_struct() {
    // When
    let code = generated(PAGE);

    // Then
    let top = code.find("use crate::taps::Faces;").expect("the use");
    assert!(
        top < code.find("pub struct CounterProps").unwrap_or(0),
        "{code}"
    );
    assert_eq!(code.matches("use crate::taps::Faces;").count(), 1, "{code}");
}

#[test]
fn a_use_naming_a_local_item_is_refused() {
    // When
    let err = refusal("<script>\nenum Mode { A }\nuse Mode::*;\n</script>\n<p>x</p>");

    // Then
    assert_eq!((err.line, err.col), (3, 1), "{err}");
    assert!(
        err.message.contains("Mode") && err.message.contains("wisp-check"),
        "{err}"
    );
}

#[test]
fn a_use_of_a_name_the_header_imports_is_not_imported_twice() {
    // When
    let code = generated("<script>\nuse std::rc::Rc;\n</script>\n<p>x</p>");

    // Then
    assert_eq!(code.matches("use std::rc::Rc;").count(), 1, "{code}");
}

#[test]
fn script_code_is_preceded_by_its_wisp_line() {
    // When
    let code = generated("<script>\nlet a = 1;\n\nlet b = 2;\n</script>\n<p>x</p>");

    // Then
    assert_has(&code, "// from ");
    assert!(code.contains(":2\n") && code.contains(":4\n"), "{code}");
    assert!(!code.contains("const _: &str"), "{code}");
}

#[test]
fn generated_code_names_dom_through_elements() {
    // When
    let code = generated("<p>x</p>");

    // Then
    assert!(!code.contains(" vapors_dom::"), "{code}");
    assert_has(&code, "vapors_elements::__wisp::dom::");
}

#[test]
fn a_component_that_uses_itself_is_not_imported_into_its_own_module() {
    // When
    let code =
        generated("<script>\nuse crate::counter::{Counter, CounterProps};\n</script>\n<p>x</p>");

    // Then
    assert!(!code.contains("use crate::counter"), "{code}");
}
