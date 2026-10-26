//! The caps checked against the exact count.

use super::*;

fn ux() -> UxConfig {
    crate::ux_review::tests::ux()
}

#[test]
fn a_count_within_every_cap_gives_its_worst_case() {
    // When: 20,000 x $2 + 12,000 x $12, per million
    let worst = within(&ux(), 30_000, 20_000, 0.0).expect("within");

    // Then
    assert!((worst - 0.184).abs() < 1e-9, "{worst}");
}

#[test]
fn a_count_over_the_estimate_is_refused_as_the_estimate_failing() {
    let refused = within(&ux(), 20_000, 20_001, 0.0).expect_err("refused");
    assert!(
        refused.contains("over the estimate of 20000 that should bound it"),
        "{refused}"
    );
}

#[test]
fn a_count_over_the_input_cap_is_refused() {
    let refused = within(&ux(), 90_000, 61_000, 0.0).expect_err("refused");
    assert!(
        refused.contains("over ux.max_input_tokens 60000"),
        "{refused}"
    );
}

#[test]
fn a_count_that_would_pass_the_day_s_ceiling_is_refused() {
    let refused = within(&ux(), 30_000, 20_000, 2.90).expect_err("refused");
    assert!(refused.contains("over ux.daily_cost_usd"), "{refused}");
}

#[test]
fn the_count_endpoint_is_beside_the_responses_endpoint() {
    let mut ux = ux();
    ux.endpoint = "https://api.openai.com/v1/responses/".to_owned();
    assert_eq!(
        count_url(&ux),
        "https://api.openai.com/v1/responses/input_tokens"
    );
}

#[test]
fn a_count_of_zero_or_of_another_object_is_refused() {
    let zero = serde_json::json!({ "object": "response.input_tokens", "input_tokens": 0 });
    let other = serde_json::json!({ "object": "response", "input_tokens": 10 });
    assert!(
        input_tokens(&zero)
            .expect_err("zero")
            .contains("not a positive whole number")
    );
    assert!(
        input_tokens(&other)
            .expect_err("other")
            .contains("not a `response.input_tokens`")
    );
    let good = serde_json::json!({ "object": "response.input_tokens", "input_tokens": 26_924 });
    assert_eq!(input_tokens(&good), Ok(26_924));
}
