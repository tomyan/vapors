//! Publishing: first time, replacing, refusing what it did not make, and
//! a failure partway that leaves the served build as it was.

use std::path::{Path, PathBuf};

use super::*;

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("xtask-publish-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch");
    dir
}

fn build(root: &Path, index: &str) -> PathBuf {
    let dist = root.join(format!("dist-{index}"));
    std::fs::create_dir_all(dist.join("icons")).expect("dist");
    std::fs::write(dist.join("index.html"), index).expect("index");
    std::fs::write(dist.join("icons/a.png"), "png").expect("icon");
    dist
}

fn served(dir: &Path) -> String {
    std::fs::read_to_string(dir.join("index.html")).expect("served index")
}

#[test]
fn the_first_publish_serves_the_build_through_a_link() {
    // Given
    let root = scratch("first");
    let dir = root.join("touche");

    // When
    publish(&build(&root, "one"), &dir).expect("published");

    // Then
    assert_eq!(served(&dir), "one");
    assert!(
        std::fs::symlink_metadata(&dir)
            .expect("dir")
            .file_type()
            .is_symlink()
    );
    assert!(dir.join("icons/a.png").is_file() && dir.join(MARKER).is_file());
}

#[test]
fn a_new_build_replaces_the_served_one_and_its_copy_goes() {
    // Given
    let root = scratch("replace");
    let dir = root.join("touche");
    let first = publish(&build(&root, "one"), &dir).expect("first");

    // When
    publish(&build(&root, "two"), &dir).expect("second");

    // Then
    assert_eq!(served(&dir), "two");
    assert!(!first.exists(), "the old copy is removed");
}

#[test]
fn a_dir_publish_did_not_make_is_refused_and_kept() {
    // Given: someone's own directory at the path
    let root = scratch("foreign");
    let dir = root.join("projects");
    std::fs::create_dir_all(&dir).expect("theirs");
    std::fs::write(dir.join("notes.txt"), "keep").expect("theirs");

    // When
    let refused = publish(&build(&root, "one"), &dir).expect_err("not ours");

    // Then
    assert!(
        refused.contains("is not a copy `cargo xtask serve` made"),
        "{refused}"
    );
    assert_eq!(
        std::fs::read_to_string(dir.join("notes.txt")).expect("kept"),
        "keep"
    );
}

#[test]
fn a_failure_partway_leaves_the_served_build_as_it_was() {
    // Given: one build served, and a "build" that cannot be read
    let root = scratch("partway");
    let dir = root.join("touche");
    publish(&build(&root, "one"), &dir).expect("first");

    // When
    let failed = publish(&root.join("no-such-dist"), &dir);

    // Then
    assert!(failed.is_err());
    assert_eq!(served(&dir), "one");
    assert!(
        !root.join("touche.2").exists(),
        "the partial copy is removed"
    );
}
