//! Reading the guidelines and the hard limits, and refusing what is missing.

use super::*;

#[test]
fn a_clippy_number_is_read_past_comments_and_blank_lines() {
    // Given
    let text = "# a comment\n\ntoo-many-lines-threshold = 50\ncognitive-complexity-threshold=12\n";

    // When / Then
    assert_eq!(clippy_number(text, "too-many-lines-threshold"), Ok(50));
    assert_eq!(
        clippy_number(text, "cognitive-complexity-threshold"),
        Ok(12)
    );
}

#[test]
fn a_missing_clippy_key_is_refused_naming_it_and_the_fix() {
    let error = clippy_number("# nothing\n", "too-many-lines-threshold").expect_err("refused");
    assert_eq!(
        error,
        "clippy.toml: no `too-many-lines-threshold = <number>`; fix: set it from the measurement"
    );
}

#[test]
fn a_clippy_value_that_is_not_a_number_is_refused() {
    let error = clippy_number(
        "too-many-lines-threshold = many\n",
        "too-many-lines-threshold",
    )
    .expect_err("refused");
    assert!(
        error.contains("`too-many-lines-threshold = many` is not a number"),
        "{error}"
    );
}

#[test]
fn the_guidelines_are_read_from_quality() {
    // Given
    let config = Config::parse(
        "[quality]\nfile_lines = \"220\"\nfunction_lines = \"15\"\ncomplexity = \"6\"\nchain_hops = \"3\"\n",
    )
    .expect("parses");

    // When / Then
    assert_eq!(
        Guideline::from_config(&config),
        Ok(Guideline {
            function_lines: 15,
            complexity: 6,
            file_lines: 220,
            chain_hops: 3,
        })
    );
}

#[test]
fn a_missing_guideline_is_refused_naming_the_key() {
    // Given
    let config = Config::parse("[quality]\nfile_lines = \"220\"\n").expect("parses");

    // When
    let error = Guideline::from_config(&config).expect_err("refused");

    // Then
    assert!(error.contains("quality.function_lines"), "{error}");
}

#[test]
fn a_guideline_that_is_not_a_number_is_refused_naming_the_fix() {
    let config = Config::parse(
        "[quality]\nfile_lines = \"many\"\nfunction_lines = \"15\"\ncomplexity = \"6\"\nchain_hops = \"3\"\n",
    )
    .expect("parses");
    let error = Guideline::from_config(&config).expect_err("refused");
    assert_eq!(
        error,
        ".config/factory.toml: quality.file_lines = \"many\" is not a number; fix: set it to one"
    );
}

#[test]
fn the_hard_limits_come_from_clippy_toml_and_max_lines() {
    // Given
    let root = crate::scratch::dir("quality_hard");
    std::fs::write(
        root.join("clippy.toml"),
        "too-many-lines-threshold = 50\ncognitive-complexity-threshold = 12\n",
    )
    .expect("written");
    let config = Config::parse("[files]\nmax_lines = \"320\"\n").expect("parses");

    // When / Then
    assert_eq!(
        Hard::read(&root, &config),
        Ok(Hard {
            function_lines: 50,
            complexity: 12,
            file_lines: 320,
        })
    );
}

#[test]
fn no_clippy_toml_is_refused_naming_the_file() {
    let root = crate::scratch::dir("quality_no_clippy");
    let config = Config::parse("").expect("parses");
    let error = Hard::read(&root, &config).expect_err("refused");
    assert!(
        error.contains("clippy.toml") && error.contains("fix: add clippy.toml"),
        "{error}"
    );
}
