//! A measurement against the local stand-in: counts become a row, and a
//! billed request must match its count.

use std::path::PathBuf;

use super::*;
use crate::ux_config::UxConfig;
use crate::ux_stub_tests::stand_in_many;

const KEY: &str = "sk-test-0123456789abcdefghijklmnopqrstuv";

fn run_dir(test: &str) -> (PathBuf, PathBuf) {
    let root = std::env::temp_dir().join(format!("xtask-ux-measure-{test}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let phone = root.join("ux/phone");
    std::fs::create_dir_all(&phone).expect("made");
    let file = std::fs::File::create(phone.join("01-a.png")).expect("png");
    let mut encoder = png::Encoder::new(file, 4, 8);
    encoder.set_color(png::ColorType::Rgb);
    encoder
        .write_header()
        .expect("header")
        .write_image_data(&[0; 96])
        .expect("data");
    std::fs::write(phone.join("01-a.txt"), "").expect("text");
    std::fs::write(
        phone.join("facts.json"),
        r#"{"steps": [{"frame": "01-a.png", "text": "01-a.txt"}]}"#,
    )
    .expect("facts");
    (root.join("ux"), root.join("reports"))
}

fn ux(test: &str, endpoint: String) -> UxConfig {
    let (run, reports) = run_dir(test);
    let mut ux = crate::ux_review::tests::ux();
    ux.run_dir = run;
    ux.report_dir = reports;
    ux.endpoint = endpoint;
    ux.model = crate::ux_models::unmeasured("gpt-new");
    ux
}

fn counted(n: u64) -> (&'static str, String) {
    (
        "200 OK",
        format!(r#"{{"object": "response.input_tokens", "input_tokens": {n}}}"#),
    )
}

fn billed(input: u64) -> (&'static str, String) {
    (
        "200 OK",
        format!(
            r#"{{"status": "incomplete", "usage": {{"input_tokens": {input}, "output_tokens": 16}}}}"#
        ),
    )
}

#[test]
fn counts_become_a_row_with_each_bound_twenty_percent_over() {
    // Given: 10 without the frame, 110 at low detail, 510 at high
    let (url, _seen) = stand_in_many(vec![counted(10), counted(110), counted(510), billed(110)]);
    let secret = Secret::new(KEY.to_owned()).expect("guarded");

    // When
    let lines = crate::ux_measure_run::run(&ux("row", url), &secret).expect("measured");

    // Then
    assert_eq!(
        lines[0],
        "gpt-new 4x8 low: 110 with the frame, 10 without: 100 tokens, bound 120"
    );
    assert!(
        lines
            .last()
            .expect("row")
            .contains("Measured { size: (4, 8), low: 120, high: 600 }"),
        "{lines:?}"
    );
    assert!(
        lines
            .last()
            .expect("row")
            .contains("4x8 low 110-10; 4x8 high 510-10"),
        "{lines:?}"
    );
}

#[test]
fn a_billed_request_that_differs_from_its_count_is_refused() {
    // Given
    let (url, _seen) = stand_in_many(vec![counted(10), counted(110), counted(510), billed(140)]);
    let secret = Secret::new(KEY.to_owned()).expect("guarded");

    // When
    let refused = crate::ux_measure_run::run(&ux("differs", url), &secret).expect_err("refused");

    // Then
    assert!(
        refused.contains("said 110 input tokens, but the same request was billed 140"),
        "{refused}"
    );
}

#[test]
fn a_frame_that_counts_fewer_tokens_than_no_frame_is_refused() {
    let refused = Count::new((4, 8), "low", 9, 10).expect_err("refused");
    assert!(
        refused.contains("4x8 at low counted 9 tokens with the frame, under the 10 without it"),
        "{refused}"
    );
}
