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

#[test]
fn a_symlink_in_either_tree_is_refused_naming_it() {
    // Given
    let ours = tree("link", "a", &[("src/main.rs", "1")]);
    let theirs = tree("link", "b", &[("src/main.rs", "1")]);
    std::os::unix::fs::symlink(theirs.join("src/main.rs"), ours.join("src/linked.rs"))
        .expect("symlink");

    // When
    let refusal = differences(&ours, &theirs).expect_err("refused");

    // Then
    assert!(refusal.contains("src/linked.rs"), "{refusal}");
    assert!(refusal.contains("is a symlink"), "{refusal}");
}

const PIN: (&str, &str) = (".config/vapors-rev", "abc\n");

#[test]
fn the_fix_names_this_repositorys_pin_when_it_has_one() {
    // Given
    let here = tree("pin-here", "touche", &[PIN]);
    let sibling = tree("pin-here", "vapors", &[]);

    // When
    let fix = fix_line(&here, &sibling);

    // Then
    assert!(fix.contains("in vapors first"), "{fix}");
    assert!(
        fix.contains(&format!("{}/.config/vapors-rev", here.display())),
        "{fix}"
    );
}

#[test]
fn the_fix_names_the_siblings_pin_when_it_has_one() {
    // Given
    let here = tree("pin-there", "vapors", &[]);
    let sibling = tree("pin-there", "touche", &[PIN]);

    // When
    let fix = fix_line(&here, &sibling);

    // Then
    assert!(
        fix.contains(&format!("{}/.config/vapors-rev", sibling.display())),
        "{fix}"
    );
    assert!(!fix.contains(&here.display().to_string()), "{fix}");
}

#[test]
fn the_fix_names_no_pin_when_neither_has_one() {
    // Given
    let here = tree("pin-none", "a", &[]);
    let sibling = tree("pin-none", "b", &[]);

    // When
    let fix = fix_line(&here, &sibling);

    // Then
    assert!(
        fix.contains("neither repository has .config/vapors-rev"),
        "{fix}"
    );
}
