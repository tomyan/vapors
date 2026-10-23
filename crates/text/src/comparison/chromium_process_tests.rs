//! The launcher's deadline against nextest's: one screenshot per scale
//! must time out, and print Chromium's stderr, before nextest kills the
//! comparison test and the stderr with it.

use std::time::Duration;

use super::DEADLINE;
use crate::comparison::SCALES;

/// Rendering both scales on lavapipe, beside the screenshots: 1.3 s
/// here (the whole test 2.6 s, two screenshots 0.65 s each). On the
/// runner the whole example, renders and screenshots together, took
/// 6.85 s (run 39989880425, gpu job 120538591130, from `Running
/// target/debug/examples/text_comparison` at 16:51:07.13 to
/// `composite written` at 16:51:13.98), so the renders took less; 10 s
/// is about one and a half times that whole.
const RENDER_ALLOWANCE: Duration = Duration::from_secs(10);

/// Every `slow-timeout = { period = "<n>s", terminate-after = <m> }` in
/// `text`, as the time nextest kills a test; any other shape of a
/// slow-timeout line, or an override, is refused naming its line.
/// Comment lines are skipped.
fn kill_times(text: &str, file: &str) -> Vec<Duration> {
    text.lines()
        .enumerate()
        .filter(|(_, line)| !line.trim_start().starts_with('#'))
        .filter(|(_, line)| line.contains("slow-timeout") || line.contains("overrides"))
        .map(|(index, line)| {
            kill_time(line).unwrap_or_else(|| {
                panic!(
                    "{file}:{}: `{line}` is not a shape this check reads; fix: teach kill_time it",
                    index + 1
                )
            })
        })
        .collect()
}

fn kill_time(line: &str) -> Option<Duration> {
    let rest = line.trim().strip_prefix("slow-timeout = { period = \"")?;
    let (seconds, rest) = rest.split_once("s\", terminate-after = ")?;
    let terminate_after = rest.strip_suffix(" }")?;
    let seconds: u64 = seconds.parse().ok()?;
    Some(Duration::from_secs(
        seconds * terminate_after.parse::<u64>().ok()?,
    ))
}

#[test]
fn the_launcher_times_out_before_nextest_kills_the_comparison_test() {
    // Given
    let file = concat!(env!("CARGO_MANIFEST_DIR"), "/../../.config/nextest.toml");
    let text = std::fs::read_to_string(file).unwrap_or_else(|e| panic!("{file}: {e}"));
    let kills = kill_times(&text, file);
    assert!(!kills.is_empty(), "{file}: no slow-timeout found");

    // When
    let worst = DEADLINE * u32::try_from(SCALES.len()).expect("few scales") + RENDER_ALLOWANCE;

    // Then
    for kill in kills {
        assert!(
            worst < kill,
            "{} screenshots at the launcher's {DEADLINE:?} plus {RENDER_ALLOWANCE:?} is {worst:?}, not under nextest's {kill:?} in {file}; fix: lower DEADLINE in chromium_process.rs or raise the slow-timeout",
            SCALES.len()
        );
    }
}

#[test]
fn a_slow_timeout_line_of_another_shape_is_refused() {
    // Given
    let text = "[profile.default]\nslow-timeout = \"30s\"\n";

    // When
    let refusal = std::panic::catch_unwind(|| kill_times(text, "nextest.toml"));

    // Then
    let message = *refusal
        .expect_err("refused")
        .downcast::<String>()
        .expect("a message");
    assert!(message.starts_with("nextest.toml:2: "), "{message}");
}
