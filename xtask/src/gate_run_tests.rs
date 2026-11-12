//! Each gate step: how it runs, how it reproduces, and its deadline.

use std::path::Path;
use std::time::{Duration, Instant};

use super::*;

fn planned(run: &str) -> PlannedStep {
    PlannedStep {
        name: "gpu: gpu tier".to_owned(),
        run: run.to_owned(),
        env: [("CI", "true"), ("VAPORS_GPU_ADAPTER", "llvmpipe")]
            .map(|(k, v)| (k.to_owned(), v.to_owned()))
            .into(),
        dir: ".".to_owned(),
        job: "gpu".to_owned(),
        job_timeout: Duration::from_mins(10),
        step_timeout: None,
    }
}

#[test]
fn a_gate_step_is_bash_with_pipefail_without_a_gpu_device_and_reproduces_with_its_environment() {
    // When
    let step = step_for(
        &planned("cargo xtask test --tier gpu"),
        Path::new("/t/out"),
        Wrap::HideGpuDevice,
    );

    // Then
    assert_eq!(step.name, "gpu: gpu tier");
    assert_eq!(
        step.reproduce(),
        "CI=true VAPORS_GPU_ADAPTER=llvmpipe GITHUB_OUTPUT=/t/out bwrap --dev-bind / / --tmpfs /dev/dri --die-with-parent bash -eo pipefail -c 'cargo xtask test --tier gpu'"
    );
}

#[test]
fn a_composite_step_reproduces_from_its_directory() {
    // Given
    let mut composite = planned("echo hi");
    composite.dir = "..".to_owned();

    // When / Then
    assert!(
        step_for(&composite, Path::new("/o"), Wrap::HideGpuDevice)
            .reproduce()
            .starts_with("cd .. && CI=true")
    );
}

#[test]
fn the_deadline_is_what_is_left_of_the_job_or_the_steps_own_if_sooner() {
    // Given
    let mut step = planned("true");

    // When / Then
    assert_eq!(
        deadline(&step, Duration::from_secs(100)),
        Duration::from_secs(500)
    );
    step.step_timeout = Some(Duration::from_mins(1));
    assert_eq!(
        deadline(&step, Duration::from_secs(100)),
        Duration::from_mins(1)
    );
    assert_eq!(
        deadline(&step, Duration::from_secs(590)),
        Duration::from_secs(10)
    );
    assert_eq!(deadline(&step, Duration::from_secs(700)), Duration::ZERO);
}

#[test]
fn a_step_past_its_deadline_is_killed_with_what_it_started() {
    // Given: a step that starts a child and waits on it
    let dir = crate::scratch::dir("gate-deadline");
    let pid_file = dir.join("child.pid");
    let script = format!("sleep 300 & echo $! > {}; wait", pid_file.display());
    // As the gate runs it here: hidden where this machine has a /dev/dri
    let wrap = wrap_for_this_machine();
    let mut step = step_for(&planned(&script), &dir.join("out"), wrap);
    step.deadline = Some(Duration::from_secs(1));

    // When
    let started = Instant::now();
    let outcome = crate::command::run(&step);

    // Then
    assert!(!outcome.passed);
    assert!(
        started.elapsed() < Duration::from_secs(20),
        "{:?}",
        started.elapsed()
    );
    assert!(
        outcome.output.contains("killed: still running after 1 s"),
        "{}",
        outcome.output
    );
    let child = std::fs::read_to_string(&pid_file)
        .expect("pid")
        .trim()
        .to_owned();
    let alive = std::process::Command::new("kill")
        .args(["-0", &child])
        .status()
        .expect("kill -0");
    assert!(
        !alive.success(),
        "the step's child {child} outlived the deadline"
    );
}

#[test]
fn with_the_gpu_device_not_hidden_a_step_is_plain_bash() {
    // When
    let step = step_for(&planned("true"), Path::new("/t/out"), Wrap::SwitchedOff);

    // Then
    assert!(
        step.reproduce()
            .ends_with("GITHUB_OUTPUT=/t/out bash -eo pipefail -c true"),
        "{}",
        step.reproduce()
    );
}

#[test]
fn every_step_runs_with_the_runs_tmpdir_and_a_leak_fails_the_gate() {
    // Given: a step that checks its TMPDIR is the run's, then leaks a file
    let base = crate::scratch::dir("gate-tmpdir");
    let run_dir = base.join(format!("xtask-run-{}", std::process::id()));
    let script = format!(
        r#"[ "$TMPDIR" = "{}" ] && touch "$TMPDIR/left""#,
        run_dir.display()
    );
    let output = base.join("out");

    // When
    let reporter = crate::report::Reporter::new(false);
    let ran = crate::run_tmp::around("gate", &base, |tmp| {
        let code = run_in(
            &[planned(&script)],
            &output,
            Wrap::SwitchedOff,
            &reporter,
            tmp,
        );
        assert_eq!(code, ExitCode::SUCCESS, "the step saw the run's TMPDIR");
        code
    });

    // Then
    let refusal = ran.expect_err("the leak fails the gate");
    assert!(
        refusal.contains("gate left 1 entry ") && refusal.contains("left"),
        "{refusal}"
    );
}

fn wrap_for_this_machine() -> Wrap {
    sandbox_wrap(&crate::config::Config::parse("").expect("empty")).expect("the sandbox")
}

#[test]
fn an_unsandboxed_gate_says_so_again_just_before_its_pass_line() {
    // When
    let hidden = pass_lines("abc", Wrap::HideGpuDevice);
    let plain = pass_lines("abc", Wrap::NoRenderNode);

    // Then
    assert_eq!(hidden, ["gate: pass abc"]);
    assert_eq!(plain.len(), 2, "{plain:?}");
    assert!(plain[0].contains("no render node"), "{plain:?}");
    assert_eq!(plain[1], "gate: pass abc");
}
