//! Finding and killing a browser's processes by its profile argument.

use std::path::{Path, PathBuf};
use std::process::{Child, Command};

use super::*;

/// A shell that loops until killed, with `arg` among its arguments. The
/// loop keeps the shell from exec-ing its command, which would drop the
/// argument, and bounds an orphaned `sleep` to 0.2 s.
fn holder(arg: &str) -> Child {
    Command::new("sh")
        .args(["-c", "while :; do sleep 0.2; done", "holder", arg])
        // Not the test's: an orphaned `sleep` holding them is a leak to nextest.
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .expect("sh starts")
}

fn profile(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("vapors-pids-{}-{name}", std::process::id()))
}

fn sorted(mut pids: Vec<u32>) -> Vec<u32> {
    pids.sort_unstable();
    pids
}

fn reap(children: &mut [Child]) {
    for child in children {
        let _ = child.kill();
        let _ = child.wait();
    }
}

#[test]
fn the_processes_holding_a_profile_are_found_by_the_whole_argument() {
    // Given: two holders of one profile, one of a profile that extends its
    // path, one of another
    let ours = profile("a");
    let flag = |p: &Path| format!("--user-data-dir={}", p.display());
    let mut children = vec![
        holder(&flag(&ours)),
        holder(&flag(&ours)),
        holder(&format!("{}-longer", flag(&ours))),
        holder(&flag(&profile("b"))),
    ];

    // When
    let found = sorted(holding(&ours));

    // Then
    let wanted = sorted(vec![children[0].id(), children[1].id()]);
    reap(&mut children);
    assert_eq!(found, wanted);
}

#[test]
fn killing_a_profile_s_processes_ends_them_and_spares_the_rest_and_this_process() {
    // Given
    let (ours, theirs) = (profile("c"), profile("d"));
    let flag = |p: &Path| format!("--user-data-dir={}", p.display());
    let mut children = vec![
        holder(&flag(&ours)),
        holder(&flag(&ours)),
        holder(&flag(&theirs)),
    ];

    // When
    let killed = kill_holding(&ours);

    // Then
    for child in &mut children[..2] {
        let _ = child.wait();
    }
    let spared = holding(&theirs);
    let others = children[2].id();
    reap(&mut children);
    assert_eq!(killed, Ok(()));
    assert_eq!(holding(&ours), Vec::<u32>::new());
    assert_eq!(spared, vec![others]);
}

#[test]
fn a_profile_no_process_holds_is_killed_at_once() {
    // Given / When / Then
    assert_eq!(kill_holding(&profile("none")), Ok(()));
}
