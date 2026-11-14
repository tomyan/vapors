//! Killing a step and what it started.

use std::os::unix::process::CommandExt;
use std::process::{Command, Stdio};

use super::*;

/// A process group: a shell that starts a child, writes its pid to
/// `pid_file`, and sleeps; the shell leads its own group.
fn group(pid_file: &std::path::Path) -> Child {
    let script = format!("sleep 300 & echo $! > {}; sleep 300", pid_file.display());
    Command::new("sh")
        .args(["-c", &script])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .process_group(0)
        .spawn()
        .expect("the group")
}

/// Whether `pid` runs, from /proc: gone on `NotFound`, ended as a zombie
/// (a killed orphan stays one until init reaps it, which `kill -0`
/// counts as alive); any other read error or shape panics.
fn alive(pid: &str) -> bool {
    let path = format!("/proc/{pid}/stat");
    let stat = match std::fs::read_to_string(&path) {
        Ok(stat) => stat,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return false,
        Err(e) => panic!("{path}: {e}"),
    };
    let state = stat
        .rfind(')')
        .and_then(|end| stat[end + 1..].trim_start().chars().next())
        .unwrap_or_else(|| panic!("{path} reads `{}`", stat.trim()));
    state != 'Z'
}

/// Whether `pid` still runs after up to 5 s to end (a killed process
/// ends when next scheduled; text's chromium test measured 20 ms at
/// worst at load 76 to 114).
fn alive_after_kill(pid: &str) -> bool {
    let started = Instant::now();
    while alive(pid) {
        if started.elapsed() >= Duration::from_secs(5) {
            return true;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    false
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
    assert!(
        !alive_after_kill(child.trim()),
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
    let mut leader = started(&script, &ready);

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

/// A group whose leader runs `script` in bash, waiting until it has
/// written `ready`.
fn started(script: &str, ready: &std::path::Path) -> Child {
    // Output to /dev/null: a child left running must not hold the test
    // binary's output open, or cargo test waits for it.
    let leader = Command::new("bash")
        .args(["-c", script])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .process_group(0)
        .spawn()
        .expect("the group");
    assert!(appears(ready), "the group did not start");
    leader
}

#[test]
fn a_step_that_ignores_the_term_is_killed_after_the_grace() {
    // Given: TERM ignored, which the sleeps it starts inherit
    let scratch = crate::scratch::dir("kill-ignores-term");
    let ready = scratch.join("ready");
    let script = format!(
        "trap '' TERM; touch {}; while :; do sleep 0.1; done",
        ready.display()
    );
    let mut leader = started(&script, &ready);

    // When
    let asked = Instant::now();
    let killed = kill_group(&mut leader);

    // Then
    assert_eq!(killed, Ok(()));
    assert!(
        asked.elapsed() >= TERM_GRACE,
        "ended before the grace: {:?}",
        asked.elapsed()
    );
    assert!(leader.try_wait().expect("reaped").is_some());
}

#[test]
fn a_child_that_ignores_the_term_is_killed_once_its_leader_has_ended() {
    // Given: the leader ends on the TERM; its child ignores it
    let scratch = crate::scratch::dir("kill-child-ignores");
    let (ready, pid_file) = (scratch.join("ready"), scratch.join("child.pid"));
    let script = format!(
        "(trap '' TERM; exec sleep 300) & echo $! > {p}; trap 'exit 0' TERM; touch {r}; wait",
        p = pid_file.display(),
        r = ready.display()
    );
    let mut leader = started(&script, &ready);
    let child = std::fs::read_to_string(&pid_file).expect("the child's pid");

    // When
    let killed = kill_group(&mut leader);

    // Then
    assert_eq!(killed, Ok(()));
    assert!(
        !alive_after_kill(child.trim()),
        "the child {} outlived the KILL",
        child.trim()
    );
}

#[test]
fn groups_asked_to_end_run_their_traps_before_any_kill() {
    // Given
    let scratch = crate::scratch::dir("kill-ask-groups");
    let (work, ready) = (scratch.join("work"), scratch.join("ready"));
    let script = format!(
        "mkdir {w}; trap 'rm -rf {w}' EXIT; trap 'exit 1' TERM; touch {r}; sleep 300 & wait",
        w = work.display(),
        r = ready.display()
    );
    let mut leader = started(&script, &ready);

    // When
    ask_groups_to_end(&[leader.id()]);

    // Then
    assert!(!work.exists(), "{} was left behind", work.display());
    let _ = kill_group(&mut leader);
}
