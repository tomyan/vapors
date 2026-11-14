//! The 60 s run's verdict: every failure together, or none.

use std::time::Duration;

use super::verdict;
use crate::sampler::JudgedFrame;
use crate::scene_kind::SceneKind;

fn frames(idle_millis: &[u64]) -> Vec<JudgedFrame> {
    let frame = |(index, &millis)| JudgedFrame {
        index,
        idle: Duration::from_millis(millis),
        ..JudgedFrame::default()
    };
    idle_millis.iter().enumerate().map(frame).collect()
}

#[test]
fn frames_within_budget_on_a_quiet_machine_pass() {
    assert_eq!(
        verdict(&frames(&[5, 9, 16]), Ok(0.3), SceneKind::Racer),
        Ok(())
    );
}

#[test]
fn each_frame_over_budget_is_named_and_the_run_fails() {
    let error =
        verdict(&frames(&[5, 17, 9, 30]), Ok(0.3), SceneKind::Racer).expect_err("over budget");
    assert!(error.starts_with("2 frames over 16.7ms"), "{error}");
    assert!(error.contains("frame 1 at 0.016 s,"), "{error}");
    assert!(error.contains("frame 3 at 0.048 s,"), "{error}");
    assert!(
        error.ends_with("reproduce: cargo run --release -p vapors-perf -- --scene-run"),
        "{error}"
    );
}

#[test]
fn a_busy_machine_at_the_end_fails_the_run_beside_any_slow_frame() {
    let error = verdict(&frames(&[20]), Ok(2.25), SceneKind::Racer).expect_err("busy");
    assert!(error.contains("frame 0 at 0.000 s,"), "{error}");
    assert!(error.contains("load average is 2.25 at the end"), "{error}");
}
