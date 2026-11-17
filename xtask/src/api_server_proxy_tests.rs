use super::{betas, settled};
use crate::api_http::Request;
use crate::api_prices::Usage;
use crate::api_usage::Counted;

#[test]
fn an_api_error_answered_whole_with_an_error_status_is_charged_nothing() {
    for status in [400, 429, 500, 529] {
        let charged = settled((status, Counted::Error("overloaded_error".into())), "m");
        assert_eq!(
            charged,
            Counted::Whole(Usage::default(), "m".into()),
            "{status}"
        );
    }
}

#[test]
fn an_error_body_with_a_success_status_is_charged_as_cut() {
    let charged = settled((200, Counted::Error("api_error".into())), "m");
    assert!(matches!(charged, Counted::Cut(_)), "{charged:?}");
}

#[test]
fn an_error_status_whose_response_was_cut_or_not_an_error_body_stays_as_counted() {
    for counted in [Counted::Cut("cut".into()), Counted::Unpriced("x".into())] {
        let expected = format!("{counted:?}");
        assert_eq!(format!("{:?}", settled((529, counted), "m")), expected);
    }
}

#[test]
fn a_request_whose_anthropic_beta_header_names_another_beta_is_refused() {
    let request = |beta: &str| Request {
        method: "POST".into(),
        target: "/v1/messages".into(),
        headers: vec![("anthropic-beta".into(), beta.into())],
        body: Vec::new(),
    };
    assert_eq!(betas(&request("claude-code-20250219")), Ok(()));
    assert!(betas(&request("context-1m-2025-08-07")).is_err());
}
