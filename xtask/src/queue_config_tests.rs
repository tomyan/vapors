use super::*;
use crate::config::Config;

const HOME: &str = "/home/q";

fn config(text: &str) -> Result<QueueConfig, String> {
    QueueConfig::from(&Config::parse(text).expect("valid"), HOME)
}

#[test]
fn a_queue_table_gives_its_repo_mode_worktree_log_and_serve_dir() {
    // When
    let queue = config(
        "[queue]\nrepo = \"tomyan/touche\"\nmerge_on = \"gate\"\nrun = \"~/.cache/w-queue/touche-run/touche\"\nlog = \"~/.cache/queue/touche.log\"\nserve = \"~/.cache/w-serve/touche\"\ntargets = \"/t\"\n",
    )
    .expect("read");

    // Then
    assert_eq!(queue.repo, "tomyan/touche");
    assert_eq!(queue.merge_on, MergeOn::Gate);
    assert_eq!(
        queue.run,
        std::path::PathBuf::from("/home/q/.cache/w-queue/touche-run/touche")
    );
    assert_eq!(
        queue.log,
        std::path::PathBuf::from("/home/q/.cache/queue/touche.log")
    );
    assert_eq!(
        queue.serve,
        Some(std::path::PathBuf::from("/home/q/.cache/w-serve/touche"))
    );
}

#[test]
fn checks_mode_and_no_serve_dir_are_read() {
    let queue = config("[queue]\nrepo = \"tomyan/vapors\"\nmerge_on = \"checks\"\nrun = \"/q/run\"\nlog = \"/q/log\"\ntargets = \"/t\"\n").expect("read");
    assert_eq!(queue.merge_on, MergeOn::Checks);
    assert_eq!(queue.serve, None);
}

#[test]
fn a_missing_key_or_an_unknown_mode_is_refused_naming_it() {
    let no_repo = config("[queue]\nmerge_on = \"gate\"\nrun = \"/q\"\nlog = \"/q/l\"\n")
        .expect_err("no repo");
    assert!(no_repo.contains("queue.repo"), "{no_repo}");
    let mode =
        config("[queue]\nrepo = \"a/b\"\nmerge_on = \"vibes\"\nrun = \"/q\"\nlog = \"/q/l\"\n")
            .expect_err("mode");
    assert!(
        mode.contains("\"vibes\"") && mode.contains("gate or checks"),
        "{mode}"
    );
}

const WITHOUT_TARGETS: &str =
    "[queue]\nrepo = \"a/b\"\nmerge_on = \"gate\"\nrun = \"/q\"\nlog = \"/q/l\"\n";

#[test]
fn targets_are_space_separated_roots_read_against_home() {
    // When
    let queue = config(&format!(
        "{WITHOUT_TARGETS}targets = \"~/.cache/touche-targets  /t/vapors-targets\"\n"
    ))
    .expect("read");

    // Then
    assert_eq!(
        queue.targets,
        [
            std::path::PathBuf::from("/home/q/.cache/touche-targets"),
            std::path::PathBuf::from("/t/vapors-targets")
        ]
    );
}

#[test]
fn missing_or_empty_targets_are_refused_naming_the_key() {
    let missing = config(WITHOUT_TARGETS).expect_err("missing");
    assert!(missing.contains("queue.targets is not set"), "{missing}");
    let empty = config(&format!("{WITHOUT_TARGETS}targets = \" \"\n")).expect_err("empty");
    assert_eq!(
        empty,
        "queue.targets names no directory; fix: list the target roots, space-separated"
    );
}
