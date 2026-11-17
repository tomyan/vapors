//! The 60 s run's verdict: every failure together, or none.

use std::time::Duration;

use super::{profile_line, reproducing, require_adapter, verdict};
use crate::args::SceneProfile;
use crate::gpu_time::FEATURES;
use crate::sampler::JudgedFrame;
use crate::scene_judge::require_hardware;
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

#[test]
fn a_refusal_before_frame_zero_names_the_asked_scenes_command() {
    // Given
    let refused =
        require_hardware(wgpu::DeviceType::Cpu, "llvmpipe", "llvmpipe").expect_err("software");

    // When
    let line = reproducing(SceneKind::OneScene)(refused);

    // Then
    assert!(
        line.ends_with("reproduce: cargo run --release -p vapors-perf -- --scene-run one-scene"),
        "{line}"
    );
}

const LLVMPIPE: (wgpu::DeviceType, &str, &str) = (wgpu::DeviceType::Cpu, "llvmpipe", "llvmpipe");

#[test]
fn a_judged_run_refuses_a_software_adapter_and_a_profile_runs_on_one() {
    // Given
    let profile_only = SceneProfile {
        profile_only: true,
        ..SceneProfile::default()
    };

    // When
    let judged = require_adapter(LLVMPIPE, FEATURES, SceneProfile::default());
    let profiled = require_adapter(LLVMPIPE, FEATURES, profile_only);

    // Then
    assert!(judged.expect_err("software").contains("llvmpipe"));
    assert_eq!(profiled, Ok(()));
}

#[test]
fn a_profile_still_needs_timestamps() {
    let profile_only = SceneProfile {
        profile_only: true,
        ..SceneProfile::default()
    };
    let refused = require_adapter(LLVMPIPE, wgpu::Features::empty(), profile_only);
    assert!(refused.is_err());
}

#[test]
fn the_runs_line_says_its_size_and_whether_it_is_judged() {
    let profile_only = SceneProfile {
        profile_only: true,
        ..SceneProfile::default()
    };
    assert_eq!(
        profile_line((1920, 1080), profile_only),
        "Size: 1920 x 1080; not judged (--profile-only): no budget verdict, any adapter\n"
    );
    assert_eq!(
        profile_line((1100, 720), SceneProfile::default()),
        "Size: 1100 x 720; judged against the frame budget\n"
    );
}
