//! `[queue] targets`: the roots the queue empties targets in, checked
//! when the settings load.

use std::path::{Path, PathBuf};

use super::*;
use crate::config::Config;
use crate::queue_targets_scratch::Scratch;

const WITHOUT_TARGETS: &str =
    "[queue]\nrepo = \"a/b\"\nmerge_on = \"gate\"\nrun = \"/q\"\nlog = \"/q/l\"\n";

fn with_targets(targets: &str, home: &Path) -> Result<QueueConfig, String> {
    let text = format!("{WITHOUT_TARGETS}targets = \"{targets}\"\n");
    QueueConfig::from(
        &Config::parse(&text).expect("valid"),
        &home.display().to_string(),
    )
}

#[test]
fn targets_are_canonical_roots_read_against_home() {
    // Given: one root under home, one named through a link
    let home = Scratch::new("config-home");
    let touche = home.dir(".cache/touche-targets");
    let vapors = home.dir("vapors-real");
    let alias = home.link("vapors-alias", &vapors);

    // When
    let queue = with_targets(
        &format!("~/.cache/touche-targets  {}", alias.display()),
        &home.0,
    );

    // Then
    assert_eq!(queue.map(|q| q.targets), Ok(vec![touche, vapors]));
}

#[test]
fn missing_or_empty_targets_are_refused_naming_the_key() {
    let home = Scratch::new("config-empty");
    let missing = QueueConfig::from(&Config::parse(WITHOUT_TARGETS).expect("valid"), "/home/q")
        .expect_err("missing");
    assert!(missing.contains("queue.targets is not set"), "{missing}");
    assert_eq!(
        with_targets(" ", &home.0).expect_err("empty"),
        "queue.targets names no directory; fix: list the target roots, space-separated"
    );
}

#[test]
fn a_root_that_is_not_an_existing_directory_of_its_own_is_refused_naming_it() {
    // Given
    let home = Scratch::new("config-refused");
    let file = home.0.join("file");
    std::fs::write(&file, "x").expect("file");
    let shown = |p: &PathBuf| p.display().to_string();
    let cases = [
        ("relative".to_owned(), "is not an absolute path".to_owned()),
        ("~".to_owned(), "is not an absolute path".to_owned()),
        (
            shown(&home.0.join("nope")),
            format!(
                "does not exist; fix: mkdir -p {}",
                shown(&home.0.join("nope"))
            ),
        ),
        (shown(&file), "is not a directory".to_owned()),
        ("/".to_owned(), "is the root directory".to_owned()),
        ("~/".to_owned(), "is the home directory".to_owned()),
    ];

    for (root, said) in cases {
        // When
        let why = with_targets(&root, &home.0).expect_err(&root);

        // Then
        assert!(why.starts_with("queue.targets: "), "{root}: {why}");
        assert!(why.contains(&said), "{root}: {why}");
        assert!(why.contains("fix: "), "{root}: {why}");
    }
}

#[test]
fn a_root_that_holds_the_run_worktree_or_home_is_refused() {
    // Given: home under a scratch directory, and the run worktree under home's .cache
    let scratch = Scratch::new("config-holds");
    let home = scratch.dir("home");
    let cache = scratch.dir("home/.cache");
    scratch.dir("home/.cache/w-queue/run");
    let text = |root: &str| {
        format!(
            "[queue]\nrepo = \"a/b\"\nmerge_on = \"gate\"\nrun = \"~/.cache/w-queue/run\"\nlog = \"/q/l\"\ntargets = \"{root}\"\n"
        )
    };
    let load = |root: &str| {
        QueueConfig::from(
            &Config::parse(&text(root)).expect("valid"),
            &home.display().to_string(),
        )
    };

    // Then
    let holds_run = load(&cache.display().to_string()).expect_err("holds run");
    assert!(
        holds_run.contains("holds the queue's run worktree"),
        "{holds_run}"
    );
    let holds_home = load(&scratch.0.display().to_string()).expect_err("holds home");
    assert!(
        holds_home.contains("holds the home directory"),
        "{holds_home}"
    );
}
