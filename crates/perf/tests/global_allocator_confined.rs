//! The counting allocator stays in the memory binary and its test: a
//! global allocator set anywhere else in the workspace's crates would
//! slow every timed run of `vapors-perf` (review of #321, L1). A line of
//! code naming it in any form counts.

use std::path::{Path, PathBuf};

/// The files allowed to set the global allocator, from `crates/`.
/// This file names it too, to look for it.
const ALLOWED: [&str; 3] = [
    "perf/src/memory_main.rs",
    "perf/tests/gpu_element_memory.rs",
    "perf/tests/global_allocator_confined.rs",
];

fn rust_files(dir: &Path, found: &mut Vec<PathBuf>) {
    let entries = std::fs::read_dir(dir).unwrap_or_else(|e| panic!("{}: {e}", dir.display()));
    for entry in entries {
        let path = entry.expect("a directory entry").path();
        if path.is_dir() && !path.ends_with("target") {
            rust_files(&path, found);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            found.push(path);
        }
    }
}

/// A line of code, not a comment, that names `global_allocator` in any
/// form: the attribute alone, before its static, or in a `cfg_attr`.
fn names_the_allocator(line: &str) -> bool {
    let code = line.trim_start();
    !code.starts_with("//") && code.contains("global_allocator")
}

#[test]
fn only_the_memory_binary_and_its_test_set_the_global_allocator() {
    // Given
    let crates = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut files = Vec::new();
    rust_files(&crates, &mut files);

    // When
    let setting: Vec<String> = files
        .iter()
        .filter(|path| {
            let source = std::fs::read_to_string(path).expect("a readable source");
            source.lines().any(names_the_allocator)
        })
        .map(|path| {
            path.strip_prefix(&crates)
                .expect("under crates")
                .display()
                .to_string()
        })
        .collect();

    // Then
    let mut allowed = ALLOWED.map(String::from).to_vec();
    allowed.sort();
    let mut setting = setting;
    setting.sort();
    assert_eq!(
        setting, allowed,
        "fix: keep the counting allocator in vapors-perf-memory"
    );
}
