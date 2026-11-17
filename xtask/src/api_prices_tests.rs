use super::{Price, Usage, prices};
use crate::config::Config;

#[test]
fn a_models_price_is_read_from_its_table() {
    // Given
    let config = Config::parse("[api_build.prices]\nclaude-sonnet-5-5 = \"2 10 2.5 0.2\"\n")
        .expect("parses");

    // When
    let prices = prices(&config).expect("read");

    // Then
    let sonnet = Price {
        input: 2.0,
        output: 10.0,
        cache_write: 2.5,
        cache_read: 0.2,
    };
    assert_eq!(prices.get("claude-sonnet-5-5"), Some(&sonnet));
}

#[test]
fn a_price_that_is_not_four_numbers_is_refused_naming_it() {
    let config = Config::parse("[api_build.prices]\nm = \"2 10\"\n").expect("parses");
    let refused = prices(&config).expect_err("refused");
    assert!(
        refused.starts_with("[api_build.prices] m = \"2 10\""),
        "{refused}"
    );
}

#[test]
fn usage_costs_each_kind_of_token_at_its_price() {
    // Given: a million of each
    let price = Price {
        input: 2.0,
        output: 10.0,
        cache_write: 2.5,
        cache_read: 0.2,
    };
    let usage = Usage {
        input: 1_000_000,
        output: 1_000_000,
        cache_write: 1_000_000,
        cache_read: 1_000_000,
    };

    // Then
    assert!((price.of(usage) - 14.7).abs() < 1e-9);
}
