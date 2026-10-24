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
