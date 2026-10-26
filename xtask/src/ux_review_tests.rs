//! The last step of a review: caps, then the dry run or the refusal.

use super::*;
use crate::ux_config::{Caps, Prices};

fn ux() -> UxConfig {
    UxConfig {
        run_dir: "ux".into(),
        checklist: "c.md".into(),
        design: vec!["d.md".into()],
        report_dir: "r".into(),
        model: crate::ux_models::find("gpt-5.6-terra").expect("listed"),
        endpoint: "https://api.openai.com/v1/responses".to_owned(),
        key_env: "OPENAI_API_KEY".to_owned(),
        reasoning: "medium".to_owned(),
        image_detail: "high".to_owned(),
        prices: Prices {
            input: 2.0,
            cached: 0.2,
            output: 12.0,
            source: "read 2026-10-26".to_owned(),
        },
        caps: Caps {
            input_tokens: 60_000,
            output_tokens: 12_000,
            cost_usd: 0.30,
            daily_usd: 3.00,
        },
    }
}

fn prepared(text: u64) -> Prepared {
    Prepared {
        items: Vec::new(),
        viewports: Vec::new(),
        prompt: String::new(),
        estimate: Estimate { text, images: 0 },
    }
}

#[test]
fn a_dry_run_within_the_caps_says_nothing_was_sent() {
    // When
    let lines = checked(&ux(), &prepared(1000), true).expect("summarised");

    // Then
    assert_eq!(
        lines.last().map(String::as_str),
        Some("dry run: nothing was sent")
    );
}

#[test]
fn a_live_run_is_refused_naming_the_flag_until_it_is_built() {
    // When
    let refused = checked(&ux(), &prepared(1000), false).expect_err("refused");

    // Then
    assert!(refused.contains("fix: pass --dry-run"), "{refused}");
}

#[test]
fn a_run_over_a_cap_is_refused_before_the_dry_run_or_the_live_call() {
    for dry_run in [true, false] {
        // When
        let refused = checked(&ux(), &prepared(70_000), dry_run).expect_err("refused");

        // Then
        assert!(refused.contains("over ux.max_input_tokens"), "{refused}");
    }
}
