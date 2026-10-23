//! The xtask copies compared, file by file.

use std::path::{Path, PathBuf};

use super::*;

/// A tree under the temp dir holding `files` (path, text), target/ included.
fn tree(test: &str, side: &str, files: &[(&str, &str)]) -> PathBuf {
    let root = std::env::temp_dir()
        .join(format!("xtask-twin-{test}-{}", std::process::id()))
        .join(side);
    let _ = std::fs::remove_dir_all(&root);
    for (path, text) in files {
        let file = root.join(path);
        std::fs::create_dir_all(file.parent().expect("a parent")).expect("dirs");
        std::fs::write(file, text).expect("file");
    }
    root
}

#[test]
fn identical_trees_have_no_differences_and_target_is_not_compared() {
    // Given
    let ours = tree(
        "same",
        "a",
        &[("src/main.rs", "fn main() {}"), ("target/x", "1")],
    );
    let theirs = tree(
        "same",
        "b",
        &[("src/main.rs", "fn main() {}"), ("target/x", "2")],
    );

    // When / Then
    assert_eq!(differences(&ours, &theirs), Ok(Vec::new()));
}

#[test]
fn each_differing_or_missing_file_is_named() {
    // Given
    let ours = tree(
        "diff",
        "a",
        &[("src/main.rs", "1"), ("src/only_here.rs", "x")],
    );
    let theirs = tree(
        "diff",
        "b",
        &[("src/main.rs", "2"), ("src/only_there.rs", "y")],
    );

    // When
    let found = differences(&ours, &theirs).expect("compared");

    // Then
    assert_eq!(
        found,
        [
            "differs: src/main.rs",
            "only here: src/only_here.rs",
            "only in the sibling: src/only_there.rs"
        ]
    );
}

#[test]
fn a_missing_sibling_is_refused_naming_it() {
    // When
    let refusal = differences(Path::new("."), Path::new("/nonexistent/xtask")).expect_err("absent");

    // Then
    assert!(refusal.contains("/nonexistent/xtask"), "{refusal}");
}
