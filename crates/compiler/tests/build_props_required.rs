//! `build::compile_dir_into` refuses a component tag that omits a
//! required prop of a component in the same directory, naming the
//! tag's file and line, the component and the prop; a tag that passes it
//! wraps it in `Required::Given`; a tag it cannot check is not wrapped.

use crate::common_build::{dirs, read, write};
use crate::common_props::shown;
use vapors_compiler::build::compile_dir_into;

const CARD: &str = "<script>\n#[prop]\nlet title: String;\n#[prop]\nlet size: usize = 1;\n</script>\n\n<h2>{title}</h2>";
const USE: &str = "<script>\nuse crate::card::{Card, CardProps};\n</script>\n\n";

fn compile(name: &str, template: &str) -> (Vec<String>, String) {
    let (src, out) = dirs(name);
    write(&src, "card.wisp", CARD);
    write(&src, "page.wisp", &format!("{USE}{template}"));
    let errors = shown(compile_dir_into(&src, &out));
    let page = out.join("wisp/page.rs");
    let code = if page.exists() {
        read(&page)
    } else {
        String::new()
    };
    (errors, code)
}

#[test]
fn an_omitted_required_prop_is_refused_naming_file_line_component_and_prop() {
    // When
    let (errors, _) = compile("required_omitted", "<div><Card size={2} /></div>");

    // Then
    assert_eq!(errors.len(), 1, "{errors:?}");
    let error = &errors[0];
    assert!(error.contains("page.wisp:5:6"), "{error}");
    assert!(
        error.contains("<Card>") && error.contains("`title`"),
        "{error}"
    );
    assert!(error.contains("card.wisp:2"), "{error}");
    assert!(error.contains("cargo run -p vapors-compiler"), "{error}");
}

#[test]
fn every_omitted_required_prop_is_refused_once() {
    // Given: a second component, with two required props
    let (src, out) = dirs("required_two");
    write(
        &src,
        "pair.wisp",
        "<script>\n#[prop]\nlet a: i32;\n#[prop]\nlet b: i32;\n</script>\n<i></i>",
    );
    let uses = "<script>\nuse crate::pair::{Pair, PairProps};\n</script>\n\n<Pair />";
    write(&src, "page.wisp", uses);

    // When
    let errors = shown(compile_dir_into(&src, &out));

    // Then
    assert_eq!(errors.len(), 2, "{errors:?}");
    assert!(
        errors[0].contains("`a`") && errors[1].contains("`b`"),
        "{errors:?}"
    );
}

#[test]
fn a_passed_required_prop_compiles_wrapped() {
    // When
    let (errors, code) = compile("required_given", "<Card title=\"x\" />");

    // Then
    assert!(errors.is_empty(), "{errors:?}");
    assert!(code.contains("Required::Given("), "{code}");
    assert!(!code.contains("size: Required"), "{code}");
}

#[test]
fn a_tag_the_compiler_cannot_check_is_not_wrapped_and_not_refused() {
    // When
    let (errors, code) = compile("required_unchecked", "<other::Card title=\"x\" />");

    // Then
    assert!(errors.is_empty(), "{errors:?}");
    assert!(!code.contains("Required::Given"), "{code}");
}
