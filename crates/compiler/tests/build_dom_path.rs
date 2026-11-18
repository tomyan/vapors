//! How a built file names the dom crate, and the comment saying so.

use crate::common_build as build;

use build::{dirs, read, write};
use vapors_compiler::build::{DomPath, Styles, compile_dir_into_with};

fn built(test: &str, dom: DomPath) -> String {
    let (src, out) = dirs(test);
    write(
        &src,
        "page.wisp",
        "<style>\np { margin: 0; }\n</style>\n<p>x</p>",
    );
    compile_dir_into_with(&src, &out, (Styles::Scoped, dom)).unwrap_or_else(|e| panic!("{e:?}"));
    read(&out.join("wisp/page.rs"))
}

#[test]
fn by_default_the_dom_is_named_through_elements_and_the_file_says_so() {
    // When
    let code = built("dom_default", DomPath::Elements);

    // Then
    assert!(code.contains("vapors_elements::__wisp::dom::"), "{code}");
    assert!(!code.contains(" vapors_dom::"), "{code}");
    assert!(
        code.contains("// The dom is named through vapors-elements"),
        "{code}"
    );
}

#[test]
fn below_elements_the_dom_is_named_as_vapors_dom_and_the_file_says_so() {
    // When
    let code = built("dom_below", DomPath::VaporsDom);

    // Then
    assert!(code.contains("vapors_dom::StyleKey"), "{code}");
    assert!(!code.contains("__wisp"), "{code}");
    assert!(code.contains("// The dom is named as vapors_dom"), "{code}");
}
