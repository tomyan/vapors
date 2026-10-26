//! Usage read, and answers that do not match what was asked refused.

use serde_json::json;

use super::*;
use crate::ux_checklist::parse;
use crate::ux_estimate::Estimate;
use crate::ux_run::{Frame, Viewport};

fn prepared() -> Prepared {
    let frame = Frame {
        name: "phone/01-first-open.png".to_owned(),
        path: "f.png".into(),
        size: (412, 915),
        text: String::new(),
    };
    Prepared {
        items: parse(
            "c.md",
            "- judged first-screen: First.\n- judged next-step: Next.\n",
        )
        .expect("parsed"),
        viewports: vec![Viewport {
            name: "phone".to_owned(),
            facts: json!({}),
            frames: vec![frame],
        }],
        prompt: String::new(),
        estimate: Estimate { text: 0, images: 0 },
    }
}

fn judged(item: &str, verdict: &str, frames: &[&str]) -> Value {
    json!({ "item": item, "verdict": verdict, "severity": 2, "frames": frames, "note": "A person sees it." })
}

fn response(judgements: &[Value]) -> Value {
    let text = json!({ "judgements": judgements }).to_string();
    json!({
        "status": "completed",
        "output": [{ "type": "reasoning" }, { "type": "message", "content": [{ "type": "output_text", "text": text }] }],
        "usage": { "input_tokens": 30_000, "input_tokens_details": { "cached_tokens": 10_000 }, "output_tokens": 2_000 }
    })
}

fn good() -> Value {
    response(&[
        judged("first-screen", "pass", &["phone/01-first-open.png"]),
        judged("next-step", "cannot-judge", &[]),
    ])
}

#[test]
fn usage_and_its_cost_are_read_with_the_cache_discounted() {
    // Given
    let prices = Prices {
        input: 2.0,
        cached: 0.2,
        output: 10.0,
        source: String::new(),
    };

    // When
    let usage = usage(&good()).expect("read");

    // Then: 20,000 x $2 + 10,000 x $0.20 + 2,000 x $10, per million
    assert_eq!(
        usage,
        Usage {
            input: 30_000,
            cached: 10_000,
            output: 2_000
        }
    );
    assert!((usage.cost(&prices) - 0.062).abs() < 1e-9);
}

#[test]
fn a_response_with_no_usage_is_refused() {
    let refused = usage(&json!({ "status": "completed" })).expect_err("refused");
    assert!(refused.contains("no `input_tokens`"), "{refused}");
}

#[test]
fn a_complete_answer_is_read() {
    // When
    let found = judgements(&good(), &prepared()).expect("read");

    // Then
    assert_eq!(found.len(), 2);
    assert_eq!(found[0].frames, ["phone/01-first-open.png"]);
}

#[test]
fn an_incomplete_response_is_refused_with_its_reason() {
    // Given
    let mut response = good();
    response["status"] = json!("incomplete");
    response["incomplete_details"] = json!({ "reason": "max_output_tokens" });

    // When
    let refused = judgements(&response, &prepared()).expect_err("refused");

    // Then
    assert!(refused.contains("max_output_tokens"), "{refused}");
}

#[test]
fn a_refusal_is_refused_with_its_words() {
    // Given
    let mut response = good();
    response["output"][1]["content"] = json!([{ "type": "refusal", "refusal": "I cannot help." }]);

    // When
    let refused = judgements(&response, &prepared()).expect_err("refused");

    // Then
    assert!(refused.contains("I cannot help."), "{refused}");
}

#[test]
fn answers_that_do_not_match_the_question_are_refused() {
    let cases = [
        (
            vec![judged("first-screen", "pass", &["phone/01-first-open.png"])],
            "`next-step` 0 times",
        ),
        (
            vec![
                judged("first-screen", "pass", &["phone/02-x.png"]),
                judged("next-step", "pass", &["phone/01-first-open.png"]),
            ],
            "was not sent",
        ),
        (
            vec![
                judged("first-screen", "great", &["phone/01-first-open.png"]),
                judged("next-step", "pass", &["phone/01-first-open.png"]),
            ],
            "verdict `great`",
        ),
        (
            vec![
                judged("first-screen", "fail", &[]),
                judged("next-step", "pass", &["phone/01-first-open.png"]),
            ],
            "cites no frame",
        ),
    ];
    for (list, expected) in cases {
        // When
        let refused = judgements(&response(&list), &prepared()).expect_err(expected);

        // Then
        assert!(refused.contains(expected), "{expected}: {refused}");
    }
}

#[test]
fn an_item_judged_twice_or_not_asked_is_refused() {
    // Given
    let frame = ["phone/01-first-open.png"];
    let twice = [
        judged("first-screen", "pass", &frame),
        judged("first-screen", "fail", &frame),
        judged("next-step", "pass", &frame),
    ];
    let stray = [
        judged("first-screen", "pass", &frame),
        judged("next-step", "pass", &frame),
        judged("colour", "pass", &frame),
    ];

    // When
    let refused_twice = judgements(&response(&twice), &prepared()).expect_err("twice");
    let refused_stray = judgements(&response(&stray), &prepared()).expect_err("stray");

    // Then
    assert!(
        refused_twice.contains("`first-screen` 2 times"),
        "{refused_twice}"
    );
    assert!(
        refused_stray.contains("`colour`, which is not a judged item"),
        "{refused_stray}"
    );
}
