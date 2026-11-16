//! `compile_dir_global`: components whose `STYLE` is a global sheet, as
//! the layout corpus needs so that its CSS means what Chromium reads.
//! Nothing is scoped: no `SCOPE`, no `add_style`, no tag; each mount
//! registers the sheet itself under `SHEET`, its crate and file.

use crate::common_build as build;

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
    for scoping in ["pub const SCOPE", "add_style(", "scope: Some(SCOPE)"] {
        assert!(!code.contains(scoping), "{scoping} in {code}");
    }
}

#[test]
fn a_global_components_winners_name_its_sheet_by_its_path_under_the_build_root() {
    // Given
    let (src, out) = dirs("global_winners");
    write(&src, "card.wisp", STYLED);

    // When
    compile_dir_into_styled(&src, &out, Styles::Global).unwrap_or_else(|errs| panic!("{errs:?}"));

    // Then: the same on every machine, as a scope's name is (the
    // elements' sites still hold the absolute path; not this test's)
    let code = read(&out.join("wisp/card.rs"));
    let root = src.display().to_string();
    let sheets = squashed(&code);
    let sheets = sheets
        .split("Global(")
        .skip(1)
        .chain(sheets.split("SHEET:&str=").skip(1));
    for named in sheets {
        assert!(
            !named.starts_with(&format!("concat!(env!(\"CARGO_PKG_NAME\"),\"/\",\"{root}")),
            "{code}"
        );
    }
    assert!(
        squashed(&code).contains(&squashed(&format!("Global({NAME})"))),
        "{code}"
    );
}

#[test]
fn a_globally_styled_component_registers_its_sheet_under_the_name_its_winners_use() {
    // Given
    let (src, out) = dirs("global_sheet");
    write(&src, "card.wisp", STYLED);

    // When
    compile_dir_into_styled(&src, &out, Styles::Global).unwrap_or_else(|errs| panic!("{errs:?}"));

    // Then: one name, built the same way, in the constant, the mount's
    // registration and the key's winners
    let code = read(&out.join("wisp/card.rs"));
    for needle in [
        format!("pub const SHEET: &str = {NAME};"),
        "dom.borrow_mut().add_global_style(SHEET, STYLE);".to_owned(),
        format!("vapors_dom::SheetRef::Global({NAME})"),
    ] {
        assert!(
            squashed(&code).contains(&squashed(&needle)),
            "{needle} missing from {code}"
        );
    }
}

/// The global card's sheet name: its path under the build root.
const NAME: &str = "concat!(env!(\"CARGO_PKG_NAME\"), \"/\", \"src/card.wisp\")";

/// Without whitespace or the trailing commas prettyplease adds when it
/// wraps a long line.
fn squashed(code: &str) -> String {
    let compact: String = code.chars().filter(|c| !c.is_whitespace()).collect();
    compact.replace(",)", ")")
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
        "scope: Some(SCOPE)",
    ] {
        assert!(code.contains(scoping), "{scoping} missing from {code}");
    }
}

#[test]
fn only_the_layout_corpus_builds_its_components_with_global_styles() {
    // Given: every crate's build script in the workspace
    let crates = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut users = Vec::new();
    for entry in std::fs::read_dir(&crates).expect("crates dir") {
        let dir = entry.expect("entry").path();
        let Ok(text) = std::fs::read_to_string(dir.join("build.rs")) else {
            continue;
        };

        // When
        if text.contains("compile_dir_global") {
            users.push(
                dir.file_name()
                    .expect("name")
                    .to_string_lossy()
                    .into_owned(),
            );
        }
    }

    // Then: an app or demo built this way would lose scoping silently
    assert_eq!(
        users,
        ["elements-corpus"],
        "compile_dir_global is for the layout corpus only, whose CSS must mean what Chromium reads; an app uses compile_dir"
    );
}

#[test]
fn a_global_file_compiled_alone_names_its_sheet_by_its_file_not_its_path() {
    // Given: an absolute path
    let (src, _) = dirs("global_file");
    write(&src, "card.wisp", STYLED);
    let path = src.join("card.wisp");

    // When
    let code = vapors_compiler::compile_file_global(&path).unwrap_or_else(|err| panic!("{err}"));

    // Then
    let named = squashed(&code);
    assert!(
        named.contains(&squashed(&format!(
            "SHEET: &str = {}",
            in_crate("card.wisp")
        ))),
        "{code}"
    );
    assert!(
        !named.contains(&squashed(&in_crate(&path.display().to_string()))),
        "{code}"
    );
}

/// `name` after the crate's name, as the generated code writes it.
fn in_crate(name: &str) -> String {
    format!("concat!(env!(\"CARGO_PKG_NAME\"), \"/\", {name:?})")
}
