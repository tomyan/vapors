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

fn alive(pid: &str) -> bool {
    Command::new("kill")
        .args(["-0", pid])
        .stderr(Stdio::null())
        .status()
        .expect("kill -0")
        .success()
}

#[test]
fn killing_the_group_kills_the_child_it_started_too() {
    // Given
    let pid_file = std::env::temp_dir().join(format!("xtask-kill-group-{}", std::process::id()));
    let _ = std::fs::remove_file(&pid_file);
    let mut leader = group(&pid_file);
    let started = Instant::now();
    while !pid_file.exists() && started.elapsed() < Duration::from_secs(10) {
        std::thread::sleep(Duration::from_millis(20));
    }
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
    let _ = std::fs::remove_file(&pid_file);
}

#[test]
fn a_group_that_has_already_exited_is_not_a_failed_kill() {
    // Given: a group whose only process has exited and been reaped
    let mut leader = Command::new("true").process_group(0).spawn().expect("true");
    leader.wait().expect("reaped");

    // When / Then
    assert_eq!(kill_group(&mut leader), Ok(()));
}
