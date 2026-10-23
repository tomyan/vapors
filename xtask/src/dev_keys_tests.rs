//! The dev server's keys: the `--key` argument and the file checks.
//! Every key file here is a temporary file; no message may contain the
//! key.

use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

use super::*;

const KEY: &str = "sk-test-dev-key-0001";

/// A key file named for the test, holding `contents`, with `mode`.
fn key_file(test: &str, contents: &str, mode: u32) -> PathBuf {
    let path = std::env::temp_dir().join(format!("xtask-dev-key-{test}-{}", std::process::id()));
    std::fs::write(&path, contents).expect("temporary key file");
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(mode)).expect("its mode");
    path
}

/// A words file shaped as the page's: a stub without a key and two keyed providers.
const WORDS: &str = r#"
pub const PROVIDER_STUB: &str = "stub";
pub const PROVIDER_ANTHROPIC: &str = "anthropic";
pub const PROVIDER_OPENAI: &str = "openai";
pub fn key_source(provider: &str) -> Result<&'static KeySource, String> {
    match provider {
        PROVIDER_ANTHROPIC => Ok(&anthropic::KEY),
        PROVIDER_OPENAI => Ok(&openai::KEY),
        PROVIDER_STUB => Err("the scripted demo needs no key".into()),
    }
}
"#;

fn providers() -> Vec<String> {
    keyed_providers(WORDS, "words.rs").expect("keyed providers")
}

#[test]
fn the_keyed_providers_are_the_constants_the_key_source_maps_to_a_key() {
    assert_eq!(providers(), ["anthropic", "openai"]);
}

#[test]
fn a_words_file_where_no_provider_takes_a_key_is_refused_naming_it() {
    // When
    let refused = keyed_providers("pub const PROVIDER_STUB: &str = \"stub\";\n", "words.rs");

    // Then
    let why = refused.expect_err("none keyed");
    assert!(
        why.starts_with("words.rs: no provider takes a key"),
        "{why}"
    );
    assert!(why.contains("[dev] words"), "{why}");
}

fn dev_key(provider: &str, file: PathBuf) -> DevKey {
    DevKey {
        provider: provider.to_owned(),
        file,
    }
}

#[test]
fn a_key_argument_names_a_keyed_provider_and_a_file() {
    // When
    let parsed = parse("openai=/keys/openai", &providers());

    // Then
    assert_eq!(parsed, Ok(dev_key("openai", PathBuf::from("/keys/openai"))));
}

#[test]
fn a_key_argument_without_an_equals_sign_is_refused_naming_the_form() {
    // When
    let refused = parse("openai", &providers());

    // Then
    let why = refused.expect_err("no file");
    assert!(why.contains("--key <provider>=<file>"), "{why}");
}

#[test]
fn a_provider_that_takes_no_key_is_refused_naming_the_keyed_ones() {
    for provider in ["stub", "local", ""] {
        // When
        let refused = parse(&format!("{provider}=/keys/x"), &providers());

        // Then
        let why = refused.expect_err(provider);
        assert!(why.contains("anthropic") && why.contains("openai"), "{why}");
    }
}

#[test]
fn a_key_file_readable_by_others_is_refused_naming_chmod_600() {
    // Given
    let file = key_file("mode644", KEY, 0o644);

    // When
    let refused = read(&dev_key("openai", file.clone()));

    // Then
    let why = refused.expect_err("mode 644");
    assert!(why.contains(&file.display().to_string()), "{why}");
    assert!(why.contains("chmod 600"), "{why}");
    assert!(!why.contains(KEY), "the key is in the message");
    let _ = std::fs::remove_file(file);
}

#[test]
fn a_key_file_with_mode_600_gives_its_key_without_the_line_end() {
    // Given
    let file = key_file("mode600", &format!("{KEY}\n"), 0o600);

    // When
    let key = read(&dev_key("openai", file.clone()));

    // Then
    assert_eq!(key.as_deref(), Ok(KEY));
    let _ = std::fs::remove_file(file);
}

#[test]
fn a_key_file_of_only_whitespace_is_refused_naming_the_file() {
    // Given
    let file = key_file("blank", " \n\t\n", 0o600);

    // When
    let refused = read(&dev_key("anthropic", file.clone()));

    // Then
    let why = refused.expect_err("blank");
    assert!(why.contains(&file.display().to_string()), "{why}");
    assert!(why.contains("empty"), "{why}");
    let _ = std::fs::remove_file(file);
}

#[test]
fn a_missing_key_file_is_refused_without_repeating_what_was_typed() {
    // Given
    let file = std::env::temp_dir().join("xtask-dev-key-missing-never-written");

    // When
    let refused = read(&dev_key("openai", file.clone()));

    // Then
    let why = refused.expect_err("missing");
    assert!(why.contains("does not exist"), "{why}");
    assert!(!why.contains(&file.display().to_string()), "{why}");
}

#[test]
fn the_notice_names_the_provider_and_the_file_but_never_the_key() {
    // Given
    let file = key_file("notice", KEY, 0o600);

    // When
    let line = notice(&dev_key("openai", file.clone()));

    // Then
    assert!(
        line.contains("openai") && line.contains(&file.display().to_string()),
        "{line}"
    );
    assert!(!line.contains(KEY), "{line}");
    let _ = std::fs::remove_file(file);
}
