//! The workspace measured: code and tests apart, generated files out,
//! a file that does not parse refused.

use super::*;

fn workspace(files: &[(&str, &str)]) -> crate::scratch::Scratch {
    let root = crate::scratch::dir("quality_collect");
    for (path, text) in files {
        let full = root.join(path);
        std::fs::create_dir_all(full.parent().expect("a parent")).expect("made");
        std::fs::write(full, text).expect("written");
    }
    root
}

const CODE: &str = "pub fn f(s: &S) -> i32 {\n    let _ = s.a.b.c;\n    if s.x { 1 } else { 2 }\n}\n#[cfg(test)]\nmod tests {\n    #[test]\n    fn t() { assert!(true); }\n}\n";

#[test]
fn files_functions_and_chains_are_sorted_into_code_and_tests() {
    // Given
    let root = workspace(&[
        ("crates/a/src/lib.rs", CODE),
        (
            "crates/a/src/lib_tests.rs",
            "fn helper(s: &S) { let _ = s.a.b.c; }\n",
        ),
    ]);

    // When
    let collected = collect(&root, &BTreeMap::new()).expect("measured");

    // Then
    assert_eq!(collected.files.code[0].label, "crates/a/src/lib.rs");
    assert_eq!(collected.files.tests[0].label, "crates/a/src/lib_tests.rs");
    assert_eq!(
        collected.function_lines.code[0].label,
        "crates/a/src/lib.rs:1 f"
    );
    assert_eq!(collected.function_lines.code[0].value, 2);
    assert_eq!(collected.complexity.code[0].value, 2);
    assert_eq!(
        collected.function_lines.tests.len(),
        2,
        "the #[test] and the helper"
    );
    let chains: Vec<&str> = collected.chains.iter().map(|c| c.label.as_str()).collect();
    assert_eq!(
        chains,
        ["crates/a/src/lib.rs:2 s.a.b.c", "crates/a/src/lib.rs:3 s.x"],
        "chains in test code are left out"
    );
}

#[test]
fn a_generated_file_is_left_out() {
    // Given
    let root = workspace(&[("crates/a/src/gen.rs", CODE)]);
    let generated = BTreeMap::from([("crates/a/src/gen.rs".to_owned(), "built".to_owned())]);

    // When
    let collected = collect(&root, &generated).expect("measured");

    // Then
    assert_eq!(collected, Collected::default());
}

#[test]
fn a_file_that_does_not_parse_is_refused_naming_it_and_the_fix() {
    // Given
    let root = workspace(&[("crates/a/src/bad.rs", "fn broken( {\n")]);

    // When
    let error = collect(&root, &BTreeMap::new()).expect_err("refused");

    // Then
    assert!(error.starts_with("crates/a/src/bad.rs: line 1:"), "{error}");
    assert!(
        error.ends_with("fix: make it parse, or list it under [files.generated]"),
        "{error}"
    );
}

#[test]
fn test_paths_are_tests_files_and_tests_directories() {
    assert!(is_test_path("xtask/src/files_tests.rs"));
    assert!(is_test_path("crates/a/tests/support/page.rs"));
    assert!(is_test_path("crates/a/src/tests.rs"));
    assert!(!is_test_path("crates/a/src/tests_runner.rs"));
    assert!(!is_test_path("crates/a/src/lib.rs"));
}
