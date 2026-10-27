//! The request names only what was sent, and caps its output.

use std::path::PathBuf;

use super::*;
use crate::ux_checklist::parse;
use crate::ux_estimate::Estimate;
use crate::ux_run::{Frame, Viewport};

const CHECKLIST: &str = "- measured smallTargets: Targets.\n- judged first-screen: First screen.\n- judged next-step: Next step.\n";

fn frame_file(test: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("xtask-ux-request-{test}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("made");
    let path = dir.join("01-first-open.png");
    std::fs::write(&path, b"\x89PNG not decoded here").expect("written");
    path
}

fn prepared(path: PathBuf) -> Prepared {
    let frame = Frame {
        name: "phone/01-first-open.png".to_owned(),
        path,
        size: (412, 915),
        text: String::new(),
    };
    Prepared {
        items: parse("c.md", CHECKLIST).expect("parsed"),
        viewports: vec![Viewport {
            name: "phone".to_owned(),
            facts: json!({}),
            frames: vec![frame],
        }],
        prompt: "the prompt".to_owned(),
        estimate: Estimate { text: 0, images: 0 },
    }
}

fn ux() -> UxConfig {
    crate::ux_review::tests::ux()
}

#[test]
fn the_schema_lets_the_model_name_only_judged_items_and_sent_frames() {
    // When
    let schema = schema(&prepared(frame_file("schema")));

    // Then
    let judgement = &schema["properties"]["judgements"]["items"]["properties"];
    assert_eq!(
        judgement["item"]["enum"],
        json!(["first-screen", "next-step"])
    );
    assert_eq!(
        judgement["frames"]["items"]["enum"],
        json!(["phone/01-first-open.png"])
    );
}

#[test]
fn the_body_caps_output_stores_nothing_and_attaches_each_frame() {
    // Given
    let path = frame_file("body");
    let png = std::fs::read(&path).expect("read");

    // When
    let body = body(&ux(), &prepared(path)).expect("built");

    // Then
    assert_eq!(body["max_output_tokens"], json!(12_000));
    assert_eq!(body["store"], json!(false));
    assert_eq!(body["model"], json!("gpt-5.6-terra"));
    let content = &body["input"][0]["content"];
    assert_eq!(content[0]["text"], json!("the prompt"));
    let url = format!("data:image/png;base64,{}", crate::ux_base64::encode(&png));
    assert_eq!(content[1]["image_url"], json!(url));
    assert_eq!(content[1]["detail"], json!("high"));
}

#[test]
fn a_frame_that_cannot_be_read_is_refused_naming_it() {
    // When
    let refused = body(&ux(), &prepared(PathBuf::from("/nonexistent/f.png"))).expect_err("refused");

    // Then
    assert!(refused.starts_with("/nonexistent/f.png: "), "{refused}");
}
