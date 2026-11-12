//! The sweep: dead runs' directories of this user removed, a live run's
//! and another user's left, this run's own pid treated as stale, and an
//! unremovable directory refused once.

use std::os::unix::fs::PermissionsExt;

use super::super::sweep::{Who, sweep};
use crate::scratch;

fn me() -> u32 {
    crate::xvfb_display::current_uid().expect("uid")
}

#[test]
fn a_dead_runs_directory_is_removed_and_said_and_a_live_ones_kept() {
    // Given: runs 10 (dead, two files) and 11 (running), and another name
    let base = scratch::dir("run-tmp-sweep");
    std::fs::create_dir_all(base.join("xtask-run-10/sub")).expect("dir");
    std::fs::write(base.join("xtask-run-10/sub/a"), "").expect("file");
    std::fs::create_dir(base.join("xtask-run-11")).expect("dir");
    std::fs::create_dir(base.join("xtask-run-notapid")).expect("dir");
    let who = Who {
        pid: 1,
        uid: me(),
        is_running: &|pid| pid == 11,
    };

    // When
    let said = sweep(&base, &who).expect("swept");

    // Then
    assert_eq!(said.len(), 1, "{said:?}");
    assert!(
        said[0].contains("xtask-run-10") && said[0].contains("2 entries"),
        "{said:?}"
    );
    assert!(!base.join("xtask-run-10").exists());
    assert!(base.join("xtask-run-11").exists());
    assert!(base.join("xtask-run-notapid").exists());
}

#[test]
fn a_directory_with_this_runs_own_pid_is_stale_and_removed() {
    // Given: pid 12 is this run, so it is running, and has made nothing yet
    let base = scratch::dir("run-tmp-own");
    std::fs::create_dir(base.join("xtask-run-12")).expect("dir");
    let who = Who {
        pid: 12,
        uid: me(),
        is_running: &|_| true,
    };

    // When
    sweep(&base, &who).expect("swept");

    // Then
    assert!(!base.join("xtask-run-12").exists());
}

#[test]
fn another_users_directory_is_left_alone_and_said() {
    // Given: a dead run's directory whose owner is not the sweeper
    let base = scratch::dir("run-tmp-other");
    std::fs::create_dir(base.join("xtask-run-13")).expect("dir");
    let who = Who {
        pid: 1,
        uid: me() + 1,
        is_running: &|_| false,
    };

    // When
    let said = sweep(&base, &who).expect("swept");

    // Then
    assert!(base.join("xtask-run-13").exists());
    assert!(
        said.iter()
            .any(|l| l.contains("left") && l.contains("not this one's")),
        "{said:?}"
    );
}

#[test]
fn an_unremovable_directory_refuses_one_sweep_and_is_named_on_every_later_one() {
    if me() == 0 {
        println!("SKIPPED: running as root, which can remove a mode-000 directory's contents");
        return;
    }
    // Given: a dead run left a directory holding one no one can read
    let base = scratch::dir("run-tmp-stuck");
    let locked = base.join("xtask-run-14/locked");
    std::fs::create_dir_all(&locked).expect("dir");
    std::fs::write(locked.join("f"), "").expect("file");
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o000)).expect("chmod");
    let who = Who {
        pid: 1,
        uid: me(),
        is_running: &|_| false,
    };

    // When
    let first = sweep(&base, &who);
    let second = sweep(&base, &who);

    // Then
    let aside = base.join("xtask-run-14-unremovable-1");
    std::fs::set_permissions(aside.join("locked"), std::fs::Permissions::from_mode(0o700))
        .expect("chmod back");
    let why = first.expect_err("refused");
    assert!(
        why.contains("xtask-run-14") && why.contains("fix: chmod -R u+rwx"),
        "{why}"
    );
    let said = second.expect("the next sweep goes on");
    let named = format!("1 set-aside run directories: {}", aside.display());
    assert!(
        said.iter()
            .any(|l| l.starts_with(&named) && l.contains("fix: ")),
        "{said:?}"
    );
}

#[test]
fn more_set_aside_directories_than_the_limit_stop_runs_naming_them_and_the_fix() {
    // Given: four set-aside directories of this user's
    let base = scratch::dir("run-tmp-many-aside");
    for n in 1..=4 {
        std::fs::create_dir(base.join(format!("xtask-run-{n}-unremovable-1"))).expect("dir");
    }
    let who = Who {
        pid: 1,
        uid: me(),
        is_running: &|_| false,
    };

    // When
    let refusal = sweep(&base, &who).expect_err("too many");

    // Then
    assert!(refusal.contains("4 set-aside run directories"), "{refusal}");
    assert!(
        refusal.contains("more than 3") && refusal.contains("fix: "),
        "{refusal}"
    );
}
