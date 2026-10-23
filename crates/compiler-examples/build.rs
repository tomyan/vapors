//! Compiles `src/**/*.wisp` into `OUT_DIR`; see `src/lib.rs`.

fn main() {
    vapors_compiler::build::compile_dir("src");
}
