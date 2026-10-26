//! `[ux]` read and checked.

use super::*;

const GOOD: &str = r#"
[ux]
run_dir = "target/ux"
checklist = "design/ux-checklist.md"
report_dir = "~/touche-ux"
model = "gpt-5"
endpoint = "https://api.openai.com/v1/responses"
key_env = "OPENAI_API_KEY"
reasoning = "medium"
image_detail = "high"
price_input = "1.25"
price_cached = "0.125"
price_output = "10.00"
price_source = "developers.openai.com/api/docs/pricing, read 2026-10-26"
max_input_tokens = "60000"
max_output_tokens = "12000"
max_cost_usd = "0.30"
daily_cost_usd = "3.00"

[ux.design]
"design/patterns.md" = "the patterns the page follows"
"#;

fn read(text: &str) -> Result<UxConfig, String> {
    let config = Config::parse(text).map_err(|e| e.to_string())?;
    UxConfig::from_config(&config, Some("/home/u"))
}

fn with(key: &str, value: &str) -> String {
    GOOD.lines()
        .map(|line| {
            if line.starts_with(&format!("{key} =")) {
                format!("{key} = \"{value}\"")
            } else {
                line.to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn a_complete_table_is_read_with_home_expanded() {
    // When
    let ux = read(GOOD).expect("read");

    // Then
    assert_eq!(ux.report_dir, PathBuf::from("/home/u/touche-ux"));
    assert_eq!(ux.design, vec![PathBuf::from("design/patterns.md")]);
    assert_eq!(ux.caps.output_tokens, 12_000);
    assert!((ux.prices.output - 10.0).abs() < f64::EPSILON);
}

#[test]
fn a_missing_cap_is_refused_naming_it() {
    // Given
    let text = GOOD.replace("max_cost_usd = \"0.30\"\n", "");

    // When
    let refused = read(&text).expect_err("refused");

    // Then
    assert!(refused.contains("ux.max_cost_usd"), "{refused}");
}

#[test]
fn numbers_that_are_not_positive_and_finite_are_refused() {
    for (key, value) in [
        ("price_output", "0"),
        ("price_input", "NaN"),
        ("max_cost_usd", "-1"),
        ("max_input_tokens", "6e4"),
    ] {
        // When
        let refused = read(&with(key, value)).expect_err(key);

        // Then
        assert!(refused.contains(&format!("ux.{key}")), "{refused}");
    }
}

#[test]
fn a_price_source_with_no_date_is_refused() {
    // When
    let refused = read(&with("price_source", "the pricing page")).expect_err("refused");

    // Then
    assert!(refused.contains("no YYYY-MM-DD date"), "{refused}");
}

#[test]
fn an_endpoint_that_is_not_https_or_local_is_refused() {
    // When
    let refused =
        read(&with("endpoint", "http://api.openai.com/v1/responses")).expect_err("refused");

    // Then
    assert!(refused.contains("neither https://"), "{refused}");
}

#[test]
fn a_reasoning_effort_off_the_list_is_refused() {
    // When
    let refused = read(&with("reasoning", "max")).expect_err("refused");

    // Then
    assert!(refused.contains("ux.reasoning is `max`"), "{refused}");
}

#[test]
fn no_design_files_is_refused() {
    // Given
    let text = GOOD.replace(
        "[ux.design]\n\"design/patterns.md\" = \"the patterns the page follows\"\n",
        "",
    );

    // When
    let refused = read(&text).expect_err("refused");

    // Then
    assert!(
        refused.contains("[ux.design] lists no design files"),
        "{refused}"
    );
}

#[test]
fn a_home_path_with_no_home_is_refused() {
    // Given
    let config = Config::parse(GOOD).expect("parsed");

    // When
    let refused = UxConfig::from_config(&config, None).expect_err("refused");

    // Then
    assert!(refused.contains("HOME is not set"), "{refused}");
}

#[test]
fn every_key_the_review_reads_is_one_the_schema_accepts() {
    // Given
    let config = Config::parse(GOOD).expect("parsed");

    // When
    let checked = crate::config_schema::check_known(&config);

    // Then
    assert_eq!(checked, Ok(()));
}
