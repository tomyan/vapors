//! Every `.wisp` in the workspace compiles to elements created from
//! their `StyleKey`: no `create_element(` in the output, so a test that
//! looks only for `CreateKeyedElement` calls misses no element.

use std::fs;
use std::path::{Path, PathBuf};

use vapors_compiler::build::{Styles, compile_dir_into_styled};

/// Each crate's `.wisp` directory and the styles its build.rs uses.
const DIRS: &[(&str, Styles)] = &[
    ("compiler-examples/src", Styles::Scoped),
    ("components/src", Styles::Scoped),
    ("demos/demos", Styles::Scoped),
    ("elements-corpus/fixtures", Styles::Global),
    ("perf/ui", Styles::Scoped),
    ("racer/ui", Styles::Scoped),
];

/// Every `.rs` file under `dir`.
fn generated(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for entry in fs::read_dir(dir).unwrap_or_else(|e| panic!("{}: {e}", dir.display())) {
        let path = entry.expect("a directory entry").path();
        if path.is_dir() {
            out.extend(generated(&path));
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
    out
}

#[test]
fn every_workspace_wisp_creates_its_elements_from_keys() {
    let crates = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let tmp = Path::new(env!("CARGO_TARGET_TMPDIR")).join("every_wisp_keyed");
    for (dir, styles) in DIRS {
        // Given
        let out = tmp.join(dir.replace('/', "_"));
        let _ = fs::remove_dir_all(&out);

        // When
        compile_dir_into_styled(&crates.join(dir), &out, *styles)
            .unwrap_or_else(|errors| panic!("{dir}: {errors:?}"));

        // Then
        let files = generated(&out.join("wisp"));
        assert!(!files.is_empty(), "{dir}: nothing generated");
        for file in files {
            let code = fs::read_to_string(&file).expect("generated code reads");
            assert!(
                !code.contains("create_element("),
                "{}: an unkeyed element",
                file.display()
            );
        }
    }
}
