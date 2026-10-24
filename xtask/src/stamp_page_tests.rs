//! Which references get the build's query.

use super::*;

#[test]
fn each_page_script_and_style_gets_the_query() {
    // Given
    let index = r#"<link rel="stylesheet" href="shell.css" /><script src="page.js"></script><script src="first_run.js"></script>"#;

    // When
    let stamped = versioned(index, "touche%200.0.0%2Babc").expect("names files");

    // Then
    assert_eq!(
        stamped,
        r#"<link rel="stylesheet" href="shell.css?v=touche%200.0.0%2Babc" /><script src="page.js?v=touche%200.0.0%2Babc"></script><script src="first_run.js?v=touche%200.0.0%2Babc"></script>"#
    );
}

#[test]
fn trunks_hashed_files_the_manifest_and_icons_are_left_alone() {
    // Given
    let index = r#"<link rel="modulepreload" href="/touche-web-5291867cc12527f0.js"><link rel="manifest" href="manifest.webmanifest" /><link rel="icon" href="icons/icon-192.png" /><script src="page.js"></script>"#;

    // When
    let stamped = versioned(index, "v1").expect("names a file");

    // Then
    assert!(
        stamped.contains(r#"href="/touche-web-5291867cc12527f0.js""#),
        "{stamped}"
    );
    assert!(
        stamped.contains(r#"href="manifest.webmanifest""#),
        "{stamped}"
    );
    assert!(
        stamped.contains(r#"href="icons/icon-192.png""#),
        "{stamped}"
    );
    assert!(stamped.contains(r#"src="page.js?v=v1""#), "{stamped}");
}

#[test]
fn an_index_naming_no_page_file_is_refused() {
    // When
    let refused = versioned("<p>nothing</p>", "v1");

    // Then
    assert!(refused.expect_err("nothing to version").contains("fix:"));
}

/// A workspace root under the temp dir with `config` as its factory.toml.
fn root_with(test: &str, config: &str) -> std::path::PathBuf {
    let root = std::env::temp_dir().join(format!("xtask-stamp-root-{test}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join(".config")).expect("dirs");
    std::fs::write(root.join(".config/factory.toml"), config).expect("config");
    root
}

#[test]
fn the_workspace_root_is_found_wherever_the_hook_runs() {
    // Given: tests run in xtask's own directory, as trunk's hook runs in
    // crates/web; neither holds .config

    // When
    let root = workspace_root();

    // Then
    assert!(
        root.join(".config/factory.toml").is_file(),
        "{}",
        root.display()
    );
}

#[test]
fn the_product_is_read_from_the_roots_config() {
    // Given
    let root = root_with("named", "[web]\nproduct = \"touche\"\n");

    // When / Then
    assert_eq!(product(&root), Ok("touche".to_owned()));
}

#[test]
fn a_config_without_a_product_is_refused_naming_the_key_and_file() {
    // Given
    let root = root_with("unnamed", "[web]\npackage = \"x\"\n");

    // When
    let refusal = product(&root).expect_err("refused");

    // Then
    assert!(refusal.contains("[web] product"), "{refusal}");
    assert!(refusal.contains(&root.display().to_string()), "{refusal}");
}
