//! `compile_dir_global`: components whose `STYLE` the app adds as a
//! global sheet, as the layout corpus does so that its CSS means what
//! Chromium reads. Nothing is scoped: no `SCOPE`, no `add_style`, no tag.

#[path = "common/build.rs"]
mod build;

use build::{dirs, read, write};
use vapors_compiler::build::{Styles, compile_dir_into_styled};

const STYLED: &str = "<style>p { color: red; }</style>\n<p>Card</p>";

#[test]
fn a_globally_styled_component_keeps_its_style_and_scopes_nothing() {
    // Given
    let (src, out) = dirs("global_styles");
    write(&src, "card.wisp", STYLED);

    // When
    compile_dir_into_styled(&src, &out, Styles::Global).unwrap_or_else(|errs| panic!("{errs:?}"));

    // Then
    let code = read(&out.join("wisp/card.rs"));
    assert!(code.contains("pub const STYLE: &str"), "{code}");
    for scoping in ["pub const SCOPE", "add_style(", "SCOPE_ATTRIBUTE, SCOPE"] {
        assert!(!code.contains(scoping), "{scoping} in {code}");
    }
}

#[test]
fn a_scoped_build_of_the_same_component_scopes_it() {
    // Given
    let (src, out) = dirs("scoped_styles");
    write(&src, "card.wisp", STYLED);

    // When
    compile_dir_into_styled(&src, &out, Styles::Scoped).unwrap_or_else(|errs| panic!("{errs:?}"));

    // Then
    let code = read(&out.join("wisp/card.rs"));
    for scoping in [
        "pub const SCOPE",
        "add_style(SCOPE, STYLE)",
        "SCOPE_ATTRIBUTE, SCOPE",
    ] {
        assert!(code.contains(scoping), "{scoping} missing from {code}");
    }
}
