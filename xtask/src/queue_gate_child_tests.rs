use std::path::PathBuf;

use super::*;

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("xtask-gate-child-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("dir");
    dir
}

fn shell(script: &str) -> Command {
    let mut command = Command::new("sh");
    command.args(["-c", script]);
    command
}

/// Whether process `pid` still exists.
fn alive(pid: &str) -> bool {
    Command::new("kill")
        .args(["-0", pid])
        .status()
        .is_ok_and(|s| s.success())
}

#[test]
fn a_finished_gate_gives_its_exit_and_last_lines() {
    // Given
    let dir = scratch("finished");
    let out = dir.join("gate.out");

    // When
    let gated = run(
        shell("seq 1 15; echo refused >&2; exit 3"),
        Duration::from_secs(30),
        &out,
    );
    let _ = std::fs::remove_dir_all(&dir);

    // Then
    let gated = gated.expect("ran");
    assert_eq!(gated.ended, Ended::Code(3));
    assert_eq!(gated.tail, "7\n8\n9\n10\n11\n12\n13\n14\n15\nrefused");
}

#[test]
fn a_gate_past_its_deadline_is_killed_with_everything_it_started() {
    // Given: the gate starts a grandchild that would outlive it, as gate --pr starts gate
    let dir = scratch("killed");
    let pid_file = dir.join("grandchild.pid");
    let script = format!("sleep 60 & echo $! > {}; wait", pid_file.display());

    // When
    let gated = run(
        shell(&script),
        Duration::from_secs(1),
        &dir.join("gate.out"),
    );

    // Then
    let why = gated.expect_err("killed");
    assert_eq!(
        why,
        "the gate did not finish within 1s; killed with its process group"
    );
    let grandchild = std::fs::read_to_string(&pid_file).expect("pid written");
    std::thread::sleep(Duration::from_millis(200));
    let survived = alive(grandchild.trim());
    let _ = std::fs::remove_dir_all(&dir);
    assert!(!survived, "the grandchild {grandchild} outlived the kill");
}
