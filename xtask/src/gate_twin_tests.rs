//! The gate's twin step: xtask against the configured sibling's.

use std::path::PathBuf;

use super::*;

/// `<test>/<repo>/xtask/src/main.rs` holding `main_rs`, or no xtask at all.
fn checkout(test: &str, repo: &str, main_rs: Option<&str>) -> PathBuf {
    let root = std::env::temp_dir()
        .join(format!("xtask-gate-twin-{test}-{}", std::process::id()))
        .join(repo);
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("root");
    if let Some(main_rs) = main_rs {
        std::fs::create_dir_all(root.join("xtask/src")).expect("dirs");
        std::fs::write(root.join("xtask/src/main.rs"), main_rs).expect("file");
    }
    root
}

#[test]
fn without_a_configured_twin_it_says_touche_checks_it() {
    // When
    let said = check(Path::new("."), None);

    // Then
    assert_eq!(
        said,
        Ok("twin: not checked here; touche checks its xtask against vapors at its pin".to_owned())
    );
}

#[test]
fn a_twin_with_the_same_xtask_passes_naming_it() {
    // Given
    let here = checkout("same", "touche", Some("fn main() {}"));
    let sibling = checkout("same", "vapors", Some("fn main() {}"));

    // When
    let said = check(&here, Some(&sibling)).expect("the same");

    // Then
    assert!(said.starts_with("ok   twin"), "{said}");
}

#[test]
fn an_absent_twin_is_refused_naming_the_config_key() {
    // Given
    let here = checkout("absent", "touche", Some("fn main() {}"));
    let sibling = checkout("absent", "vapors", None);

    // When
    let refusal = check(&here, Some(&sibling)).expect_err("absent");

    // Then
    assert!(refusal.contains("[gate] twin"), "{refusal}");
    assert!(
        refusal.contains(&sibling.join("xtask").display().to_string()),
        "{refusal}"
    );
}

#[test]
fn a_twin_that_differs_is_refused_naming_the_file_and_the_fix() {
    // Given
    let here = checkout("differs", "touche", Some("fn main() {}"));
    let sibling = checkout("differs", "vapors", Some("fn main() { run() }"));

    // When
    let refusal = check(&here, Some(&sibling)).expect_err("differs");

    // Then
    assert!(refusal.contains("differs: src/main.rs"), "{refusal}");
    assert!(refusal.contains("fix: "), "{refusal}");
}
