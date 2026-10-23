//! Helpers for the build tests: a fresh source and output directory
//! per test under cargo's integration-test scratch directory.

#![allow(dead_code)] // each test binary uses a subset

use std::fs;
use std::path::{Path, PathBuf};

pub const COUNTER: &str =
    "<script>\nlet count: Signal<i32> = Signal::new(0);\n</script>\n<span>{count}</span>";

/// An empty `src` and `out` pair named after the test.
pub fn dirs(test: &str) -> (PathBuf, PathBuf) {
    let root = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("wisp-build")
        .join(test);
    let _ = fs::remove_dir_all(&root);
    let (src, out) = (root.join("src"), root.join("out"));
    fs::create_dir_all(&src).expect("create src");
    fs::create_dir_all(&out).expect("create out");
    (src, out)
}

/// Writes `text` at `relative` under `dir`, creating parents.
pub fn write(dir: &Path, relative: &str, text: &str) {
    let path = dir.join(relative);
    fs::create_dir_all(path.parent().expect("has a parent")).expect("create parent");
    fs::write(path, text).expect("write file");
}

pub fn read(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_else(|err| panic!("{}: {err}", path.display()))
}
