//! The report keeps measured facts apart from judgements, worst first.

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
    let facts = json!({ "tapsToFirstEdit": 7, "steps": [{ "facts": { "smallTargets": [1] } }, { "facts": { "smallTargets": null } }] });
    Prepared {
        items: parse("c.md", "- measured smallTargets: Targets.\n- measured tapsToFirstEdit: Taps.\n- judged a: A.\n- judged b: B.\n").expect("parsed"),
        viewports: vec![Viewport { name: "phone".to_owned(), facts, frames: vec![frame] }],
        prompt: String::new(),
        estimate: Estimate { text: 0, images: 0 },
    }
}

fn judgement(item: &str, verdict: &str, severity: u64) -> Judgement {
    Judgement {
        item: item.to_owned(),
        verdict: verdict.to_owned(),
        severity,
        frames: vec!["phone/01-first-open.png".to_owned()],
        note: "Seen | here.".to_owned(),
    }
}

fn header() -> Header<'static> {
    Header {
        name: "2026-10-26-abc",
        model: "gpt-x",
        usage: Usage {
            input: 30_000,
            cached: 0,
            output: 2_000,
        },
        cost: 0.08,
        estimate: 70_000,
        prices: "read 2026-10-26",
    }
}

#[test]
fn measured_facts_are_shown_per_viewport_from_facts_json() {
    // When
    let text = report(&header(), &prepared(), &[], "frames");

    // Then
    assert!(
        text.contains("| smallTargets: Targets. | 1, not measured (per step) |"),
        "{text}"
    );
    assert!(text.contains("| tapsToFirstEdit: Taps. | 7 |"), "{text}");
}

#[test]
fn judgements_are_listed_worst_first_under_the_model_s_name_with_frame_links() {
    // Given
    let judged = [judgement("a", "pass", 1), judgement("b", "fail", 2)];

    // When
    let text = report(&header(), &prepared(), &judged, "2026-10-26-abc");

    // Then
    let fail = text.find("| fail | 2 | b |").expect("fail row");
    let pass = text.find("| pass | 1 | a |").expect("pass row");
    assert!(fail < pass, "{text}");
    assert!(text.contains("## Judged by gpt-x"));
    assert!(text.contains("[phone/01-first-open.png](2026-10-26-abc/phone/01-first-open.png)"));
    assert!(
        text.contains("Seen \\| here."),
        "a note's bar would split the table: {text}"
    );
}

#[test]
fn the_header_states_tokens_estimate_and_cost() {
    // When
    let text = report(&header(), &prepared(), &[], "f");

    // Then
    assert!(text.contains("gpt-x: 30000 input tokens (0 cached; estimated at most 70000), 2000 output tokens. Cost $0.0800 at read 2026-10-26."), "{text}");
}

#[test]
fn the_scorecard_lists_what_did_not_pass_worst_first() {
    // Given
    let judged = [
        judgement("a", "pass", 3),
        judgement("b", "concern", 1),
        judgement("c", "fail", 1),
    ];

    // When
    let text = scorecard("2026-10-26-abc", &judged, "reports/2026-10-26-abc");

    // Then
    assert!(!text.contains("| pass |"), "{text}");
    let fail = text.find("| fail | 1 | c |").expect("fail row");
    let concern = text.find("| concern | 1 | b |").expect("concern row");
    assert!(fail < concern, "{text}");
}
