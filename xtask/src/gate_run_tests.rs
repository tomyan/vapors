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
fn a_gate_step_is_bash_with_pipefail_and_reproduces_with_its_environment() {
    // When
    let step = step_for(&planned("cargo xtask test --tier gpu"), Path::new("/t/out"));

    // Then
    assert_eq!(step.name, "gpu: gpu tier");
    assert_eq!(
        step.reproduce(),
        "CI=true VAPORS_GPU_ADAPTER=llvmpipe GITHUB_OUTPUT=/t/out bash -eo pipefail -c 'cargo xtask test --tier gpu'"
    );
}

#[test]
fn a_composite_step_reproduces_from_its_directory() {
    // Given
    let mut composite = planned("echo hi");
    composite.dir = "..".to_owned();

    // When / Then
    assert!(
        step_for(&composite, Path::new("/o"))
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
    let dir = std::env::temp_dir().join(format!("xtask-gate-deadline-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("tmp dir");
    let pid_file = dir.join("child.pid");
    let script = format!("sleep 300 & echo $! > {}; wait", pid_file.display());
    let mut step = step_for(&planned(&script), &dir.join("out"));
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
