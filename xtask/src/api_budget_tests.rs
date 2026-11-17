use std::collections::BTreeMap;

use super::Budget;
use crate::api_prices::{Price, Usage};
use crate::api_request_policy::Demand;
use crate::api_usage::Counted;

/// $1 a million of every kind of token.
const DOLLAR: Price = Price {
    input: 1.0,
    output: 1.0,
    cache_write: 1.0,
    cache_read: 1.0,
};

fn budget(limit: f64) -> Budget {
    Budget::new(limit, BTreeMap::from([("m".to_owned(), DOLLAR)])).expect("a budget")
}

/// A request whose worst case is $`dollars`: that many million output
/// tokens and no input.
fn demand(dollars: u64) -> Demand {
    Demand {
        model: "m".into(),
        max_tokens: dollars * 1_000_000,
        input_bound: 0,
    }
}

fn whole(output: u64) -> Counted {
    Counted::Whole(
        Usage {
            output,
            ..Usage::default()
        },
        "m".into(),
    )
}

#[test]
fn a_request_is_admitted_only_if_every_worst_case_in_flight_still_fits() {
    // Given: a $2 budget and a $1 request in flight
    let budget = budget(2.0);
    let first = budget.admit(&demand(1)).expect("admitted");

    // When: a second $1 fits; a third does not, though nothing is spent yet
    let second = budget.admit(&demand(1)).expect("admitted");
    let third = budget.admit(&demand(1));

    // Then
    assert!(
        third
            .expect_err("refused")
            .starts_with("this request could take the spend past the budget")
    );
    budget.settle(first, &whole(0));
    budget.settle(second, &whole(0));
}

#[test]
fn a_request_is_settled_at_what_it_counted_and_frees_its_reservation() {
    let budget = budget(2.0);
    let ticket = budget.admit(&demand(2)).expect("admitted");
    budget.settle(ticket, &whole(500_000));
    assert_eq!(budget.spent(), (0.5, 0.0));
}

#[test]
fn a_cut_response_is_charged_its_worst_case() {
    let budget = budget(5.0);
    let ticket = budget.admit(&demand(2)).expect("admitted");
    budget.settle(
        ticket,
        &Counted::Cut("a stream that ended before message_stop".into()),
    );
    assert_eq!(budget.spent(), (2.0, 0.0));
    assert_eq!(budget.stopped(), None);
}

#[test]
fn an_unpriced_response_is_charged_its_worst_case_and_stops_the_session() {
    let budget = budget(5.0);
    let ticket = budget.admit(&demand(1)).expect("admitted");
    budget.settle(
        ticket,
        &Counted::Unpriced("the usage field \"speed\"".into()),
    );
    assert_eq!(budget.spent(), (1.0, 0.0));
    assert!(
        budget
            .admit(&demand(1))
            .expect_err("stopped")
            .contains("speed")
    );
}

#[test]
fn a_budget_that_is_not_a_positive_amount_is_refused() {
    for limit in [f64::NAN, f64::INFINITY, 0.0, -1.0] {
        let refused = Budget::new(limit, BTreeMap::new()).err().expect("refused");
        assert!(
            refused.contains("is not a dollar amount above zero"),
            "{limit}: {refused}"
        );
    }
}

#[test]
fn a_model_with_no_price_is_refused() {
    let refused = budget(2.0)
        .admit(&Demand {
            model: "other".into(),
            ..demand(1)
        })
        .expect_err("refused");
    assert!(
        refused.starts_with("model \"other\" has no price"),
        "{refused}"
    );
}

#[test]
fn a_closed_budget_admits_nothing() {
    let budget = budget(2.0);
    budget.close("the session ended");
    assert_eq!(
        budget.admit(&demand(1)).err(),
        Some("the session ended".to_owned())
    );
}
