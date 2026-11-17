//! Compile the racer UI, and the test pages (`tests/pages`, one-scene
//! R3a1) into `OUT_DIR/pages`, which only the tests include.
fn main() {
    vapors_compiler::build::compile_dir("ui");
    vapors_compiler::build::compile_dir_under("tests/pages", "pages");
}
