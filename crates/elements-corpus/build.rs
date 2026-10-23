//! Compiles `fixtures/*.wisp` into `OUT_DIR`; see `src/lib.rs`. Their
//! styles stay global, added by the corpus as sheets, so each means what
//! Chromium reads from the same CSS: a scoped copy would be one attribute
//! more specific than the rule Chromium applies. This is the only crate
//! allowed `compile_dir_global` (vapors-compiler's `build_global` test).

fn main() {
    vapors_compiler::build::compile_dir_global("fixtures");
}
