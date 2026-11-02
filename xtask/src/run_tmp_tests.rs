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
    let finished = run.finish("ci");

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
    let refusal = run.finish("ci").expect_err("left files");

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
    let refusal = run.finish("gate").expect_err("left files");

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

#[test]
fn a_dead_runs_directory_is_removed_and_said_and_a_live_ones_kept() {
    // Given: runs 10 (dead, two files) and 11 (running), and another name
    let base = scratch::dir("run-tmp-sweep");
    std::fs::create_dir_all(base.join("xtask-run-10/sub")).expect("dir");
    std::fs::write(base.join("xtask-run-10/sub/a"), "").expect("file");
    std::fs::create_dir(base.join("xtask-run-11")).expect("dir");
    std::fs::create_dir(base.join("xtask-run-notapid")).expect("dir");

    // When
    let said = sweep(&base, &|pid| pid == 11).expect("swept");

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
