use super::*;

#[test]
fn page_sources_are_the_text_files_under_the_dirs_outside_build_output() {
    // Given
    let root = std::env::temp_dir().join(format!("xtask-page-sources-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    for dir in ["web/src", "web/target", "web/node_modules/x", "web/.cache"] {
        std::fs::create_dir_all(root.join(dir)).expect("dir");
    }
    for file in [
        "web/index.html",
        "web/src/a.rs",
        "web/src/b.js",
        "web/logo.png",
        "web/target/c.rs",
        "web/node_modules/x/d.js",
        "web/.cache/e.js",
    ] {
        std::fs::write(root.join(file), "text").expect("file");
    }

    // When
    let sources = page_sources(&root, "web");
    let _ = std::fs::remove_dir_all(&root);

    // Then
    let mut paths: Vec<String> = sources.expect("read").into_iter().map(|s| s.path).collect();
    paths.sort();
    assert_eq!(paths, ["web/index.html", "web/src/a.rs", "web/src/b.js"]);
}

#[test]
fn a_page_dir_that_does_not_exist_is_refused_naming_it() {
    let why =
        page_sources(std::path::Path::new("/nonexistent"), "crates/web").expect_err("refused");
    assert!(why.starts_with("[docs] page: crates/web: "), "{why}");
}
