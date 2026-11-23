//! The committed answers against the names that need one.

use super::*;

/// A directory holding empty files named `files`.
fn dir_with(files: &[&str]) -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("a scratch dir under the temp dir");
    for file in files {
        std::fs::write(dir.path().join(file), "").expect("file");
    }
    dir
}

fn names(names: &[&str]) -> Vec<String> {
    names.iter().map(|n| (*n).to_owned()).collect()
}

#[test]
fn an_answer_for_each_wanted_name_matches() {
    // Given
    let dir = dir_with(&["a.ax.json", "b.ax.json", "a.wisp", "a.boxes.json"]);

    // When
    let found = unmatched_in(dir.path(), &names(&["a", "b"]), ".ax.json").expect("reads");

    // Then: other suffixes are not counted
    assert_eq!(found, Unmatched::default());
}

#[test]
fn a_wanted_name_without_an_answer_is_missing() {
    // Given
    let dir = dir_with(&["a.ax.json"]);

    // When
    let found = unmatched_in(dir.path(), &names(&["a", "b"]), ".ax.json").expect("reads");

    // Then
    assert_eq!(found.missing, names(&["b"]));
    assert!(found.orphaned.is_empty());
}

#[test]
fn an_answer_no_name_wants_is_orphaned() {
    // Given: c's fixture removed, or moved to another page
    let dir = dir_with(&["a.ax.json", "c.ax.json", "c.wisp"]);

    // When
    let found = unmatched_in(dir.path(), &names(&["a"]), ".ax.json").expect("reads");

    // Then
    assert_eq!(found.orphaned, names(&["c"]));
    assert!(found.missing.is_empty());
}

#[test]
fn a_directory_that_cannot_be_read_is_refused_naming_it() {
    // Given
    let dir = dir_with(&[]);
    let gone = dir.path().join("gone");

    // When
    let refused = unmatched_in(&gone, &names(&["a"]), ".ax.json").expect_err("refused");

    // Then
    assert!(refused.contains("gone"), "{refused}");
}

#[test]
fn an_orphans_refusal_names_the_file_and_the_fix() {
    // When
    let refusal = orphan("c", ".ax.json");

    // Then
    assert!(
        refusal.contains("c.ax.json") && refusal.contains("fix: remove it"),
        "{refusal}"
    );
}
