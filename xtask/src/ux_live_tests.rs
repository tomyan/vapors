//! A live review against the local stand-in: the spend is recorded
//! before the answer is judged, and nothing is sent past the day's
//! ceiling.

use std::path::{Path, PathBuf};

use serde_json::json;

use super::*;
use crate::ux_checklist::parse;
use crate::ux_estimate::Estimate;
use crate::ux_run::{Frame, Viewport};
use crate::ux_stub_tests::stand_in_many;

const KEY: &str = "sk-test-0123456789abcdefghijklmnopqrstuv";
const FRAME: &str = "phone/01-first-open.png";
pub(super) const SHA: &str = "61bfd4e945860b03ba3f2b419406c88bb8434cf8";

pub(super) fn report_dir(test: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("xtask-ux-live-{test}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("made");
    dir
}

pub(super) fn prepared(dir: &Path) -> Prepared {
    let path = dir.join("frame.png");
    std::fs::write(&path, b"png bytes").expect("written");
    Prepared {
        items: parse("c.md", "- judged first-screen: First.\n").expect("parsed"),
        viewports: vec![Viewport {
            name: "phone".to_owned(),
            facts: json!({}),
            frames: vec![Frame {
                name: FRAME.to_owned(),
                path,
                size: (412, 915),
                text: String::new(),
            }],
        }],
        prompt: "p".to_owned(),
        estimate: Estimate {
            text: 20_000,
            images: 0,
        },
    }
}

pub(super) fn ux(dir: &Path, endpoint: String) -> UxConfig {
    let mut ux = crate::ux_review::tests::ux();
    ux.report_dir = dir.to_path_buf();
    ux.endpoint = endpoint;
    ux
}

pub(super) fn response(item: &str, input: u64) -> String {
    let answer = json!({ "judgements": [{ "item": item, "verdict": "concern", "severity": 2, "frames": [FRAME], "note": "Unclear." }] });
    json!({
        "status": "completed",
        "output": [{ "type": "message", "content": [{ "type": "output_text", "text": answer.to_string() }] }],
        "usage": { "input_tokens": input, "output_tokens": 500 }
    })
    .to_string()
}

pub(super) fn secret() -> Secret {
    Secret::new(KEY.to_owned()).expect("guarded")
}

pub(super) fn ledger_lines(dir: &Path) -> usize {
    // The count's line, the sent line, and, once a response came, the
    // settled line.
    std::fs::read_to_string(dir.join(crate::ux_ledger::FILE)).map_or(0, |t| t.lines().count())
}

pub(super) fn counted(n: u64) -> (&'static str, String) {
    (
        "200 OK",
        format!(r#"{{"object": "response.input_tokens", "input_tokens": {n}}}"#),
    )
}

pub(super) fn answered(item: &str, input: u64) -> (&'static str, String) {
    ("200 OK", response(item, input))
}

#[test]
fn a_good_answer_writes_the_report_the_scorecard_and_the_spend() {
    // Given
    let dir = report_dir("good");
    let (url, _seen) = stand_in_many(vec![counted(15_000), answered("first-screen", 15_000)]);

    // When
    let lines = run(&ux(&dir, url), &prepared(&dir), &secret(), None, SHA).expect("reviewed");

    // Then: the count's line and the run's
    assert_eq!(ledger_lines(&dir), 3);
    let card = std::fs::read_to_string(dir.join("scorecard.md")).expect("scorecard");
    assert!(card.contains("| concern | 2 | first-screen |"), "{card}");
    assert!(
        lines[0].contains("15000 input and 500 output tokens"),
        "{lines:?}"
    );
}

#[test]
fn a_count_that_fails_refuses_the_run_without_sending_it() {
    // Given
    let dir = report_dir("count-fails");
    let (url, seen) = stand_in_many(vec![
        ("500 Internal Server Error", "{}".to_owned()),
        answered("first-screen", 15_000),
    ]);

    // When
    let refused = run(&ux(&dir, url), &prepared(&dir), &secret(), None, SHA).expect_err("refused");

    // Then
    assert!(refused.contains("HTTP 500"), "{refused}");
    seen.recv().expect("the count");
    assert!(seen.try_recv().is_err(), "the review was sent");
}

#[test]
fn a_count_over_the_estimate_refuses_the_run_without_sending_it() {
    // Given
    let dir = report_dir("count-over");
    let (url, seen) = stand_in_many(vec![counted(25_000), answered("first-screen", 15_000)]);

    // When
    let refused = run(&ux(&dir, url), &prepared(&dir), &secret(), None, SHA).expect_err("refused");

    // Then
    assert!(
        refused.contains("counted 25000 input tokens, over the estimate of 20000"),
        "{refused}"
    );
    seen.recv().expect("the count");
    assert!(seen.try_recv().is_err(), "the review was sent");
}

#[test]
fn a_wrong_answer_is_refused_after_its_spend_is_recorded() {
    // Given
    let dir = report_dir("wrong");
    let (url, _seen) = stand_in_many(vec![counted(15_000), answered("colour", 15_000)]);

    // When
    let refused = run(&ux(&dir, url), &prepared(&dir), &secret(), None, SHA).expect_err("refused");

    // Then
    assert!(refused.contains("`first-screen` 0 times"), "{refused}");
    assert_eq!(ledger_lines(&dir), 3);
}

#[test]
fn billed_input_over_the_count_is_refused_after_its_spend_is_recorded() {
    // Given
    let dir = report_dir("over");
    let (url, _seen) = stand_in_many(vec![counted(15_000), answered("first-screen", 16_000)]);

    // When
    let refused = run(&ux(&dir, url), &prepared(&dir), &secret(), None, SHA).expect_err("refused");

    // Then
    assert!(
        refused.contains("billed 16000 input tokens, over the 15000 the API counted"),
        "{refused}"
    );
    assert_eq!(ledger_lines(&dir), 3);
}

#[test]
fn nothing_is_counted_or_sent_once_the_day_s_spend_would_pass_the_ceiling() {
    // Given today's runs at $2.95, and a worst case of $0.18
    let dir = report_dir("ceiling");
    let today = &crate::ux_ledger::utc(SystemTime::now())[..10];
    let line = json!({ "time": format!("{today}T00:00:00Z"), "kind": "settled", "id": "earlier", "cost": 2.95 });
    std::fs::write(dir.join(crate::ux_ledger::FILE), format!("{line}\n")).expect("ledger");
    let (url, seen) = stand_in_many(vec![counted(15_000)]);

    // When
    let refused = run(&ux(&dir, url), &prepared(&dir), &secret(), None, SHA).expect_err("refused");

    // Then
    assert!(
        refused.contains("over ux.daily_cost_usd $3.00"),
        "{refused}"
    );
    assert!(seen.try_recv().is_err(), "a request was sent");
}
