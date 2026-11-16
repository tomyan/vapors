//! Generated code names its `.wisp` by the path under the compiled
//! directory's parent, after the crate's name, never by the build
//! machine's path: the same source gives the same Rust on every machine
//! and worktree, which sccache and reproducible builds rely on.

use crate::common_build as build;

use build::{dirs, read, write};
use vapors_compiler::build::{Styles, compile_dir_into_styled};

const PAGE: &str = "<script>\nlet rows: Signal<Vec<u32>> = Signal::new(vec![]);\n</script>\n<style>\nul { margin: 0; }\n</style>\n<ul>{for row in rows}<li>{row}</li>{/for}</ul>";

fn generated(test: &str, styles: Styles) -> (String, String) {
    let (src, out) = dirs(test);
    write(&src, "page.wisp", PAGE);
    compile_dir_into_styled(&src, &out, styles).unwrap_or_else(|errs| panic!("{errs:?}"));
    let root = src.parent().expect("a parent").display().to_string();
    (read(&out.join("wisp/page.rs")), root)
}

#[test]
fn generated_code_holds_no_path_of_the_build_machine() {
    for (test, styles) in [
        ("sites_scoped", Styles::Scoped),
        ("sites_global", Styles::Global),
    ] {
        // When
        let (code, root) = generated(test, styles);

        // Then
        assert!(!code.contains(&root), "{root} in {code}");
        assert!(code.contains("\"src/page.wisp:"), "{code}");
    }
}
