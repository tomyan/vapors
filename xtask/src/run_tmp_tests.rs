//! A run's own TMPDIR: made private, refused when it holds anything at
//! the end, removed either way; and directories of runs that died.

use std::os::unix::fs::PermissionsExt;

use super::*;
use crate::scratch;

#[test]
fn a_run_directory_is_made_private_and_named_for_the_run() {
    // Given
    let base = scratch::dir("run-tmp-made");

    // When
    let run = RunTmp::make(&base, 4242).expect("made");

    // Then
    assert_eq!(run.path(), base.join("xtask-run-4242"));
    let mode = std::fs::metadata(run.path())
        .expect("stat")
        .permissions()
        .mode();
    assert_eq!(mode & 0o777, 0o700);
}

#[test]
fn an_empty_run_directory_passes_and_is_removed() {
    // Given
    let base = scratch::dir("run-tmp-empty");
    let run = RunTmp::make(&base, 1).expect("made");
    let path = run.path().to_path_buf();

    // When
    let finished = run.finish("ci", false);

    // Then
    assert_eq!(finished, Ok(()));
    assert!(!path.exists());
}

#[test]
fn anything_left_fails_naming_each_file_and_is_removed() {
    // Given: a file and a directory holding one, as a test might leave
    let base = scratch::dir("run-tmp-left");
    let run = RunTmp::make(&base, 2).expect("made");
    let path = run.path().to_path_buf();
    std::fs::write(path.join("leaked.txt"), "x").expect("file");
    std::fs::create_dir(path.join("org.chromium.Chromium.abc")).expect("dir");
    std::fs::write(path.join("org.chromium.Chromium.abc/SingletonCookie"), "").expect("file");

    // When
    let refusal = run.finish("ci", false).expect_err("left files");

    // Then
    assert!(
        refusal.contains("ci left 3 entries in its TMPDIR"),
        "{refusal}"
    );
    assert!(refusal.contains("leaked.txt"), "{refusal}");
    assert!(
        refusal.contains("org.chromium.Chromium.abc/SingletonCookie"),
        "{refusal}"
    );
    assert!(refusal.contains("fix: "), "{refusal}");
    assert!(refusal.contains("reproduce: "), "{refusal}");
    assert!(!path.exists(), "removed after naming them");
}

#[test]
fn a_long_list_names_the_first_entries_and_counts_the_rest() {
    // Given
    let base = scratch::dir("run-tmp-many");
    let run = RunTmp::make(&base, 3).expect("made");
    for n in 0..30 {
        std::fs::write(run.path().join(format!("f{n:02}")), "").expect("file");
    }

    // When
    let refusal = run.finish("gate", false).expect_err("left files");

    // Then
    assert!(refusal.contains("gate left 30 entries"), "{refusal}");
    assert!(
        refusal.contains("f00") && refusal.contains("f19"),
        "{refusal}"
    );
    assert!(!refusal.contains("f20"), "{refusal}");
    assert!(refusal.contains("and 10 more"), "{refusal}");
}

#[test]
fn a_run_directory_already_there_is_refused_naming_it() {
    // Given: a directory of this pid's name, from a run before a reboot
    let base = scratch::dir("run-tmp-there");
    std::fs::create_dir(base.join("xtask-run-5")).expect("dir");

    // When
    let refusal = RunTmp::make(&base, 5).err().expect("refused");

    // Then
    assert!(
        refusal.contains(&base.join("xtask-run-5").display().to_string()),
        "{refusal}"
    );
    assert!(refusal.contains("fix: "), "{refusal}");
}

/// Whether this process runs as root, which reads any directory, so the
/// unreadable cases cannot be made; such a test says it is skipped.
fn as_root() -> bool {
    let root = crate::xvfb_display::current_uid().expect("uid") == 0;
    if root {
        println!("SKIPPED: running as root, which can read a mode-000 directory");
    }
    root
}

#[test]
fn a_tmpdir_a_step_removed_fails_the_run_naming_it() {
    // Given: a step removed $TMPDIR itself
    let base = scratch::dir("run-tmp-gone");
    let run = RunTmp::make(&base, 8).expect("made");
    std::fs::remove_dir(run.path()).expect("removed");

    // When
    let refusal = run.finish("ci", false).expect_err("gone");

    // Then
    assert!(
        refusal.contains("could not read") && refusal.contains("xtask-run-8"),
        "{refusal}"
    );
}

#[test]
fn an_unreadable_directory_fails_its_run_once_and_is_set_aside() {
    if as_root() {
        return;
    }
    // Given: a step left a directory no one can read, and an earlier run
    // with the same pid left a set-aside directory of that name already
    let base = scratch::dir("run-tmp-unreadable");
    std::fs::create_dir_all(base.join("xtask-run-9-unremovable-1/earlier")).expect("earlier");
    let run = RunTmp::make(&base, 9).expect("made");
    let locked = run.path().join("locked");
    std::fs::create_dir(&locked).expect("dir");
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o000)).expect("chmod");

    // When
    let refusal = run.finish("ci", false).expect_err("unreadable");
    let next = RunTmp::make(&base, 9);

    // Then
    let aside = base.join("xtask-run-9-unremovable-2");
    std::fs::set_permissions(aside.join("locked"), std::fs::Permissions::from_mode(0o700))
        .expect("chmod back");
    assert!(
        refusal.contains("locked") && refusal.contains("set aside"),
        "{refusal}"
    );
    assert!(refusal.contains("fix: chmod -R u+rwx"), "{refusal}");
    assert!(next.is_ok(), "the next run with that pid is not blocked");
    assert!(
        base.join("xtask-run-9-unremovable-1/earlier").exists(),
        "the earlier one is kept"
    );
}

#[test]
fn with_the_keep_switch_a_run_directory_with_leftovers_is_kept_and_said() {
    // Given
    let base = scratch::dir("run-tmp-keep");
    let run = RunTmp::make(&base, 6).expect("made");
    let path = run.path().to_path_buf();
    std::fs::write(path.join("look-at-me"), "x").expect("file");

    // When
    let refusal = run.finish("ci", true).expect_err("left files");

    // Then
    assert!(
        refusal.contains("look-at-me") && refusal.contains("kept"),
        "{refusal}"
    );
    assert!(path.join("look-at-me").exists());
}

#[path = "run_tmp_sweep_tests.rs"]
mod sweep_tests;

#[test]
fn with_the_keep_switch_a_tmpdir_that_could_not_be_read_is_kept_naming_it() {
    // Given: a step removed $TMPDIR itself, so nothing is listed but the
    // directory could not be read
    let base = scratch::dir("run-tmp-keep-gone");
    let run = RunTmp::make(&base, 16).expect("made");
    std::fs::remove_dir(run.path()).expect("removed");

    // When
    let refusal = run.finish("ci", true).expect_err("unreadable");

    // Then
    assert!(
        refusal.contains("could not read") && refusal.contains("kept"),
        "{refusal}"
    );
}
