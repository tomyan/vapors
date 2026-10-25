//! What may be served at `/`, and how it is put there.

use std::path::PathBuf;

use super::*;

#[test]
fn only_a_clean_tree_at_origin_main_is_served() {
    assert_eq!(main_refusal("abc", "abc", false), None);
    let other = main_refusal("abc", "def", false).expect("another commit");
    assert!(other.contains("not origin/main def"), "{other}");
    let dirty = main_refusal("abc", "abc", true).expect("a changed tree");
    assert!(dirty.contains("has changes"), "{dirty}");
}

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("xtask-serve-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

#[test]
fn publishing_replaces_the_served_files_whole() {
    // Given: an old build served, a new one built
    let root = scratch("publish");
    let (dist, served) = (root.join("dist"), root.join("touche"));
    std::fs::create_dir_all(dist.join("icons")).expect("dist");
    std::fs::write(dist.join("index.html"), "new").expect("index");
    std::fs::write(dist.join("icons/a.png"), "png").expect("icon");
    std::fs::create_dir_all(&served).expect("served");
    std::fs::write(served.join("stale.js"), "old").expect("stale");

    // When
    publish(&dist, &served).expect("published");

    // Then
    assert_eq!(
        std::fs::read_to_string(served.join("index.html")).expect("index"),
        "new"
    );
    assert!(served.join("icons/a.png").is_file());
    assert!(
        !served.join("stale.js").exists(),
        "the old build's files are gone"
    );
    assert!(!served.with_extension("new").exists() && !served.with_extension("old").exists());
    let _ = std::fs::remove_dir_all(root);
}
