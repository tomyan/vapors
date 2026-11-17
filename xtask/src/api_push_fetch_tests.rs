use std::path::Path;

use super::{fetched, git};
use crate::api_paths::Builder;

/// A script that leaves `marker` when it runs.
fn marking(dir: &Path, name: &str, marker: &Path) -> String {
    let script = dir.join(name);
    std::fs::write(&script, format!("#!/bin/sh\ntouch {}\n", marker.display())).expect("written");
    std::fs::set_permissions(&script, std::os::unix::fs::PermissionsExt::from_mode(0o755))
        .expect("mode");
    script.display().to_string()
}

/// `builder`'s clone with one commit on `api/b`, set up as a session
/// could leave it: an fsmonitor and every hook leave `marker`.
fn hostile_clone(builder: &Builder, marker: &Path) {
    let clone = builder.clone_dir();
    std::fs::create_dir_all(&clone).expect("made");
    let hooks = builder.root().join("hooks");
    std::fs::create_dir_all(&hooks).expect("made");
    for hook in [
        "pre-push",
        "reference-transaction",
        "post-checkout",
        "pre-commit",
    ] {
        marking(&hooks, hook, marker);
    }
    let fsmonitor = marking(&builder.root(), "fsmonitor", marker);
    git(&clone, &["init", "--quiet", "-b", "api/b"]).expect("init");
    std::fs::write(clone.join("a.txt"), "a\n").expect("written");
    git(&clone, &["add", "a.txt"]).expect("added");
    let identity = [
        "-c",
        "user.name=t",
        "-c",
        "user.email=t@t",
        "-c",
        "core.hooksPath=/dev/null",
    ];
    git(
        &clone,
        &[&identity[..], &["commit", "--quiet", "-m", "a"]].concat(),
    )
    .expect("committed");
    git(&clone, &["config", "core.fsmonitor", &fsmonitor]).expect("set");
    git(
        &clone,
        &["config", "core.hooksPath", &hooks.display().to_string()],
    )
    .expect("set");
    let _ = std::fs::remove_file(marker);
}

#[test]
fn the_branch_is_fetched_into_a_bare_repository_and_nothing_of_the_clones_runs() {
    // Given
    let home = tempfile::tempdir().expect("a scratch directory");
    let builder = Builder::new("b", home.path()).expect("a builder");
    let marker = home.path().join("ran");
    hostile_clone(&builder, &marker);
    let main = home.path().join("main.git");
    git(
        Path::new("/"),
        &["init", "--quiet", "--bare", &main.display().to_string()],
    )
    .expect("init");

    // When
    let bare = fetched(&builder, &main.join("objects"), "api/b").expect("fetched");

    // Then
    let tip = |dir: &Path| git(dir, &["rev-parse", "refs/heads/api/b"]).expect("a tip");
    assert_eq!(tip(&bare), tip(&builder.clone_dir()));
    assert!(
        !marker.exists(),
        "something the session wrote ran on the host"
    );
}

#[test]
fn a_branch_not_in_the_clone_says_to_commit_on_it() {
    let home = tempfile::tempdir().expect("a scratch directory");
    let builder = Builder::new("b", home.path()).expect("a builder");
    hostile_clone(&builder, &home.path().join("ran"));
    let main = home.path().join("main.git");
    git(
        Path::new("/"),
        &["init", "--quiet", "--bare", &main.display().to_string()],
    )
    .expect("init");
    let refused = fetched(&builder, &main.join("objects"), "api/other").expect_err("refused");
    assert!(
        refused.contains("commit your work on that branch"),
        "{refused}"
    );
}
