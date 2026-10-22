//! The walk's exclusions, symlinks and the line count.

use super::*;

/// A fresh tree under the temp dir for `name`.
fn tree(name: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!("xtask-walk-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("root");
    root
}

fn file(root: &Path, relative: &str, text: &str) {
    let path = root.join(relative);
    std::fs::create_dir_all(path.parent().expect("parent")).expect("dirs");
    std::fs::write(path, text).expect("file");
}

fn names(root: &Path) -> Vec<String> {
    workspace_files(root)
        .expect("walks")
        .iter()
        .map(|p| p.display().to_string())
        .collect()
}

#[test]
fn covered_extensions_are_found_and_others_are_not() {
    // Given
    let root = tree("covered");
    file(&root, "crates/a/lib.rs", "a\n");
    file(&root, "crates/a/shader.wgsl", "x\n");
    file(&root, "crates/a/page.html", "x\n");
    file(&root, "crates/a/glue.js", "x\n");
    file(&root, "Cargo.toml", "x\n");
    file(&root, "crates/a/notes.md", "not covered\n");
    file(&root, "crates/a/image.png", "x");

    // When / Then
    assert_eq!(
        names(&root),
        [
            "Cargo.toml",
            "crates/a/glue.js",
            "crates/a/lib.rs",
            "crates/a/page.html",
            "crates/a/shader.wgsl"
        ]
    );
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn target_git_node_modules_and_fonts_are_skipped_at_any_depth() {
    // Given
    let root = tree("skipped");
    file(&root, "target/debug/big.rs", "x\n");
    file(&root, "crates/a/target/big.rs", "x\n");
    file(&root, ".git/hooks/x.rs", "x\n");
    file(&root, "crates/a/node_modules/m/index.js", "x\n");
    file(&root, "design/fonts/font.toml", "x\n");
    file(&root, "crates/a/lib.rs", "x\n");

    // When / Then
    assert_eq!(names(&root), ["crates/a/lib.rs"]);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn a_directory_whose_name_only_starts_like_a_skipped_one_is_walked() {
    // Given
    let root = tree("targets");
    file(&root, "crates/targets/lib.rs", "x\n");
    file(&root, "crates/a/targeting.rs", "x\n");

    // When / Then
    assert_eq!(
        names(&root),
        ["crates/a/targeting.rs", "crates/targets/lib.rs"]
    );
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn hidden_directories_other_than_git_are_walked() {
    // Given
    let root = tree("hidden");
    file(&root, ".config/factory.toml", "x\n");
    file(&root, ".cargo/config.toml", "x\n");

    // When / Then
    assert_eq!(names(&root), [".cargo/config.toml", ".config/factory.toml"]);
    let _ = std::fs::remove_dir_all(root);
}

#[cfg(unix)]
#[test]
fn a_symlinked_file_is_counted_through_its_target() {
    // Given
    let root = tree("symfile");
    file(&root, "real/a.rs", "1\n2\n3\n");
    std::fs::create_dir_all(root.join("crates")).expect("dir");
    std::os::unix::fs::symlink(root.join("real/a.rs"), root.join("crates/link.rs")).expect("link");

    // When / Then
    assert_eq!(names(&root), ["crates/link.rs", "real/a.rs"]);
    assert_eq!(count_lines(&root.join("crates/link.rs")), Ok(3));
    let _ = std::fs::remove_dir_all(root);
}

#[cfg(unix)]
#[test]
fn a_symlinked_directory_is_walked_once_and_a_cycle_ends() {
    // Given: b links to a, and a/loop links back to the root
    let root = tree("symdir");
    file(&root, "a/x.rs", "x\n");
    std::os::unix::fs::symlink(root.join("a"), root.join("b")).expect("link");
    std::os::unix::fs::symlink(&root, root.join("a/loop")).expect("cycle");

    // When / Then: a is walked under its first name only
    assert_eq!(names(&root), ["a/x.rs"]);
    let _ = std::fs::remove_dir_all(root);
}

#[cfg(unix)]
#[test]
fn a_dangling_symlink_is_refused_naming_it() {
    // Given
    let root = tree("dangling");
    std::fs::create_dir_all(root.join("crates")).expect("dir");
    std::os::unix::fs::symlink(root.join("gone.rs"), root.join("crates/gone.rs")).expect("link");

    // When
    let error = workspace_files(&root).expect_err("dangling");

    // Then
    assert!(
        error.starts_with("error: crates/gone.rs: dangling symlink"),
        "{error}"
    );
    assert!(error.contains("fix: remove it"), "{error}");
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn an_unreadable_directory_names_the_relative_path() {
    // When
    let error = workspace_files(Path::new("/nonexistent-xtask-root")).expect_err("missing");

    // Then
    assert!(error.starts_with(".: "), "{error}");
}

#[test]
fn lines_are_terminators_plus_a_final_unterminated_line() {
    // Given
    let root = tree("count");
    file(&root, "n.rs", "a\nb\nc\n");
    file(&root, "u.rs", "a\nb\nc");
    file(&root, "e.rs", "");

    // When / Then
    assert_eq!(count_lines(&root.join("n.rs")), Ok(3));
    assert_eq!(count_lines(&root.join("u.rs")), Ok(3));
    assert_eq!(count_lines(&root.join("e.rs")), Ok(0));
    let _ = std::fs::remove_dir_all(root);
}
