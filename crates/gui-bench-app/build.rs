//! Compiles the components in `ui/` into `OUT_DIR/wisp.rs`.
fn main() {
    vapors_compiler::build::compile_dir("ui");
}
