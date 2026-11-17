//! Generated code names its `.wisp` by the path under the compiled
//! directory's parent, after the crate's name, never by the build
//! machine's path: every generated file (each component's and the
//! `wisp.rs` that includes them) is the same on every machine and
//! worktree, which sccache and reproducible builds rely on.

use std::path::Path;

use crate::common_build as build;

use build::{dirs, read, write};
use vapors_compiler::build::{Styles, compile_dir_into_styled};

const PAGE: &str = "<script>\nlet rows: Signal<Vec<u32>> = Signal::new(vec![]);\n</script>\n<style>\nul { margin: 0; }\n</style>\n<ul>{for row in rows}<li>{row}</li>{/for}</ul>";
const ITEM: &str = "<p>item</p>";

/// Every generated file under `out`, with its path.
fn every_file(out: &Path) -> Vec<(String, String)> {
    let mut found = Vec::new();
    for entry in std::fs::read_dir(out).expect("out") {
        let path = entry.expect("an entry").path();
        if path.is_dir() {
            found.extend(every_file(&path));
        } else {
            found.push((path.display().to_string(), read(&path)));
        }
    }
    found
}

/// The generated files of a page and a component in a subdirectory, and
/// the root that holds both the source and the output.
fn generated(test: &str, styles: Styles) -> (Vec<(String, String)>, String) {
    let (src, out) = dirs(test);
    write(&src, "page.wisp", PAGE);
    write(&src, "parts/item.wisp", ITEM);
    compile_dir_into_styled(&src, &out, styles).unwrap_or_else(|errs| panic!("{errs:?}"));
    let root = src.parent().expect("a parent").display().to_string();
    (every_file(&out), root)
}

#[test]
fn no_generated_file_holds_a_path_of_the_build_machine() {
    for (test, styles) in [
        ("sites_scoped", Styles::Scoped),
        ("sites_global", Styles::Global),
    ] {
        // When
        let (files, root) = generated(test, styles);

        // Then
        assert!(files.len() >= 3, "{files:?}");
        for (path, code) in &files {
            assert!(!code.contains(&root), "{root} in {path}:\n{code}");
        }
        let page = files
            .iter()
            .find(|(path, _)| path.ends_with("wisp/page.rs"));
        assert!(page.expect("page.rs").1.contains("\"src/page.wisp:"));
    }
}
