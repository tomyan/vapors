//! Compiles the test pages, `tests/pages/**/*.wisp`, into `OUT_DIR`;
//! the tests `include!` them, so the components are mounted through the
//! compiler's component-tag path.

fn main() {
    vapors_compiler::build::compile_dir("tests/pages");
}
