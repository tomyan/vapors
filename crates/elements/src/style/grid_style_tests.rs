//! The grid property readers: auto-flow keywords, and the implicit
//! tracks' initial value costing no allocation.

use taffy::GridAutoFlow;
use vapors_css::value::Value;

use super::{auto_flow, auto_tracks};

fn keyword(word: &str) -> Value {
    Value::Keyword(word.to_owned())
}

#[test]
fn auto_flow_reads_each_form() {
    // Given / When / Then
    let cases = [
        (keyword("row"), GridAutoFlow::Row),
        (keyword("column"), GridAutoFlow::Column),
        (keyword("dense"), GridAutoFlow::RowDense),
        (
            Value::List(vec![keyword("row"), keyword("dense")]),
            GridAutoFlow::RowDense,
        ),
        (
            Value::List(vec![keyword("column"), keyword("dense")]),
            GridAutoFlow::ColumnDense,
        ),
    ];
    for (value, flow) in cases {
        assert_eq!(auto_flow(&value), Ok(flow), "{}", value.to_css());
    }
}

#[test]
fn auto_flow_refuses_an_unknown_form_naming_it() {
    let error =
        auto_flow(&Value::List(vec![keyword("dense"), keyword("row")])).expect_err("refused");
    assert!(error.contains("dense row"), "{error}");
}

#[test]
fn auto_tracks_of_auto_is_the_empty_list_taffy_repeats_auto_for() {
    assert_eq!(auto_tracks(&keyword("auto")).map(|t| t.len()), Ok(0));
}

#[test]
fn auto_tracks_of_a_single_size_is_one_track() {
    assert_eq!(auto_tracks(&keyword("min-content")).map(|t| t.len()), Ok(1));
}
