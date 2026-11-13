//! Killing a step and what it started.

use std::os::unix::process::CommandExt;
use std::process::Command;

use super::*;

/// A process group: a shell that starts a child, writes its pid to
/// `pid_file`, and sleeps; the shell leads its own group.
fn group(pid_file: &std::path::Path) -> Child {
    let script = format!("sleep 300 & echo $! > {}; sleep 300", pid_file.display());
    Command::new("sh")
        .args(["-c", &script])
        .process_group(0)
        .spawn()
        .expect("the group")
}

/// Whether `pid` runs: from /proc, a zombie counting as ended. A killed
/// orphan stays a zombie until init reaps it, which `kill -0` counts as
/// alive.
fn alive(pid: &str) -> bool {
    std::fs::read_to_string(format!("/proc/{pid}/stat"))
        .ok()
        .and_then(|stat| stat[stat.rfind(')')? + 1..].trim_start().chars().next())
        .is_some_and(|state| state != 'Z')
}

/// Wait up to 10 s for `path` to exist.
fn appears(path: &std::path::Path) -> bool {
    let started = Instant::now();
    while !path.exists() && started.elapsed() < Duration::from_secs(10) {
        std::thread::sleep(Duration::from_millis(20));
    }
    path.exists()
}

#[test]
fn killing_the_group_kills_the_child_it_started_too() {
    // Given
    let scratch = crate::scratch::dir("kill-group");
    let pid_file = scratch.join("child.pid");
    let mut leader = group(&pid_file);
    assert!(appears(&pid_file), "the group did not start its child");
    let child = std::fs::read_to_string(&pid_file).expect("the child's pid");

    // When
    let killed = kill_group(&mut leader);

    // Then
    assert_eq!(killed, Ok(()));
    std::thread::sleep(Duration::from_millis(100));
    assert!(
        !alive(child.trim()),
        "the child {} outlived the group kill",
        child.trim()
    );
}

#[test]
fn a_group_that_has_already_exited_is_not_a_failed_kill() {
    // Given: a group whose only process has exited and been reaped
    let mut leader = Command::new("true").process_group(0).spawn().expect("true");
    leader.wait().expect("reaped");

    // When / Then
    assert_eq!(kill_group(&mut leader), Ok(()));
}

#[test]
fn a_step_killed_at_its_deadline_is_asked_to_end_first_so_its_traps_tidy_up() {
    // Given: a step that removes its work on exit, as the page contract does
    let scratch = crate::scratch::dir("kill-term");
    let (work, ready) = (scratch.join("work"), scratch.join("ready"));
    let script = format!(
        "mkdir {w}; trap 'rm -rf {w}' EXIT; trap 'exit 1' TERM; touch {r}; sleep 300 & wait",
        w = work.display(),
        r = ready.display()
    );
    let mut leader = Command::new("bash")
        .args(["-c", &script])
        .process_group(0)
        .spawn()
        .expect("the step");
    assert!(appears(&ready), "the step did not start");

    // When
    let asked = Instant::now();
    let killed = kill_group(&mut leader);

    // Then: it ended on the TERM, within the grace, not at the KILL
    let took = asked.elapsed();
    println!("ended {took:?} after the TERM");
    assert_eq!(killed, Ok(()));
    assert!(!work.exists(), "{} was left behind", work.display());
    assert!(took < TERM_GRACE, "it waited out the grace: {took:?}");
}
