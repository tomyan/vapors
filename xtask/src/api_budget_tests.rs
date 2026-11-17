use std::collections::BTreeMap;

use super::Budget;
use crate::api_prices::{Price, Usage};

/// $1 a million of every kind of token.
const DOLLAR: Price = Price {
    input: 1.0,
    output: 1.0,
    cache_write: 1.0,
    cache_read: 1.0,
};

fn budget(limit: f64) -> Budget {
    Budget::new(limit, BTreeMap::from([("m".to_owned(), DOLLAR)]))
}

const MILLION_OUT: Option<Usage> = Some(Usage {
    input: 0,
    output: 1_000_000,
    cache_write: 0,
    cache_read: 0,
});

#[test]
fn requests_go_out_until_the_spend_reaches_the_budget() {
    // Given
    let budget = budget(2.0);

    // When: $1, then $1 more
    budget.charge(DOLLAR, MILLION_OUT, "");
    let after_one = budget.admit("m");
    budget.charge(DOLLAR, MILLION_OUT, "");
    let after_two = budget.admit("m");

    // Then
    assert!(after_one.is_ok());
    assert!(
        after_two
            .expect_err("refused")
            .starts_with("the budget of $2.0000 is spent")
    );
}

#[test]
fn a_model_with_no_price_is_refused() {
    let refused = budget(2.0).admit("other").expect_err("refused");
    assert!(
        refused.starts_with("model \"other\" has no price"),
        "{refused}"
    );
}

#[test]
fn a_success_with_no_usage_stops_every_request_after_it() {
    // Given
    let budget = budget(100.0);

    // When
    budget.charge(DOLLAR, None, "a 200 with no usage");

    // Then
    let refused = budget.admit("m").expect_err("refused");
    assert!(refused.contains("a 200 with no usage"), "{refused}");
}
