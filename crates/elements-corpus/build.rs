//! Compiles `fixtures/*.wisp` into `OUT_DIR`; see `src/lib.rs`. Their
//! styles stay global, added by the corpus as sheets, so each means what
//! Chromium reads from the same CSS.

fn main() {
    vapors_compiler::build::compile_dir_global("fixtures");
}
