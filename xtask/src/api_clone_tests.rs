use std::path::Path;

use super::make;
use crate::api_paths::Builder;
use crate::queue_git::git;

/// An upstream repository with one commit on main, a main repository
/// cloned from it with a second, unpushed branch, and a home for the
/// builder, all under `dir`; the main repository's path.
fn repositories(dir: &Path) -> std::path::PathBuf {
    let upstream = dir.join("upstream");
    std::fs::create_dir_all(&upstream).expect("made");
    git(&upstream, &["init", "--quiet", "--initial-branch=main"]).expect("init");
    let commit = [
        "-c",
        "user.name=t",
        "-c",
        "user.email=t@t",
        "commit",
        "--quiet",
        "--allow-empty",
        "-m",
        "one",
    ];
    git(&upstream, &commit).expect("commit");
    let main = dir.join("main");
    git(
        dir,
        &[
            "clone",
            "--quiet",
            &upstream.display().to_string(),
            &main.display().to_string(),
        ],
    )
    .expect("clone");
    git(&main, &["branch", "unpushed-work"]).expect("branch");
    main
}

#[test]
fn a_clone_holds_only_its_branch_and_origin_main_reading_objects_through_alternates() {
    // Given
    let dir = tempfile::tempdir().expect("a scratch directory");
    let main = repositories(dir.path());
    let builder = Builder::new("t", &dir.path().join("home")).expect("a name");
    let objects = main.join(".git/objects");

    // When
    make(&builder, (&main, &objects), "api/t").expect("made");

    // Then
    let refs = git(
        &builder.clone_dir(),
        &["for-each-ref", "--format=%(refname)"],
    )
    .expect("refs");
    assert_eq!(refs, "refs/heads/api/t\nrefs/remotes/origin/main");
    let alternates =
        std::fs::read_to_string(builder.clone_dir().join(".git/objects/info/alternates"));
    assert_eq!(
        alternates.expect("alternates"),
        format!("{}\n", objects.display())
    );
    assert_eq!(
        std::fs::read_link(builder.clone_dir().join("target")).ok(),
        Some(builder.target())
    );
}

#[test]
fn a_builder_already_there_is_refused_and_left_as_it_is() {
    // Given
    let dir = tempfile::tempdir().expect("a scratch directory");
    let builder = Builder::new("t", &dir.path().join("home")).expect("a name");
    std::fs::create_dir_all(builder.root().join("work")).expect("made");

    // When
    let refused = make(&builder, (dir.path(), dir.path()), "api/t").expect_err("refused");

    // Then
    assert!(refused.contains("already has"), "{refused}");
    assert!(builder.root().join("work").exists());
}

#[test]
fn a_make_that_fails_removes_what_it_made() {
    // Given: a main repository that is not one
    let dir = tempfile::tempdir().expect("a scratch directory");
    let builder = Builder::new("t", &dir.path().join("home")).expect("a name");
    let not_a_repository = dir.path().join("empty");
    std::fs::create_dir_all(&not_a_repository).expect("made");

    // When
    let failed = make(&builder, (&not_a_repository, &not_a_repository), "api/t");

    // Then
    assert!(failed.is_err());
    assert!(!builder.root().exists() && !builder.target().exists());
}
