use std::path::Path;

use super::{fetched, git};
use crate::api_paths::Builder;
use std::io::Write;

/// A script that leaves `marker` when it runs.
fn marking(dir: &Path, name: &str, marker: &Path) -> String {
    let script = dir.join(name);
    std::fs::write(&script, format!("#!/bin/sh\ntouch {}\n", marker.display())).expect("written");
    std::fs::set_permissions(&script, std::os::unix::fs::PermissionsExt::from_mode(0o755))
        .expect("mode");
    script.display().to_string()
}

// The fetches below run upload-pack in bwrap (`upload_pack`), which
// only the host can start: they are host_ tests (`steps_tiers.rs`).

/// A home with a builder `b`, the main repository's objects, and the
/// builder's clone as `api_clone` makes it: one commit on `api/b`,
/// alternates naming the main objects.
struct Fixture {
    home: tempfile::TempDir,
    builder: Builder,
    objects: std::path::PathBuf,
}

fn fixture() -> Fixture {
    let home = tempfile::tempdir().expect("a scratch directory");
    let builder = Builder::new("b", home.path()).expect("a builder");
    let main = home.path().join("main.git");
    git(
        Path::new("/"),
        &["init", "--quiet", "--bare", &main.display().to_string()],
    )
    .expect("init");
    let clone = builder.clone_dir();
    std::fs::create_dir_all(&clone).expect("made");
    git(&clone, &["init", "--quiet", "-b", "api/b"]).expect("init");
    let objects = main.join("objects");
    let alternates = clone.join(".git/objects/info/alternates");
    std::fs::write(alternates, format!("{}\n", objects.display())).expect("written");
    std::fs::write(clone.join("a.txt"), "a\n").expect("written");
    git(&clone, &["add", "a.txt"]).expect("added");
    commit(&clone, &["commit", "--quiet", "-m", "a"]);
    Fixture {
        home,
        builder,
        objects,
    }
}

/// git `args` in `dir` as an author would run them.
fn commit(dir: &Path, args: &[&str]) -> String {
    let identity = [
        "-c",
        "user.name=t",
        "-c",
        "user.email=t@t",
        "-c",
        "core.hooksPath=/dev/null",
    ];
    git(dir, &[&identity[..], args].concat()).expect("committed")
}

/// The clone set up as a session could leave it: an fsmonitor and every
/// hook leave `marker`.
fn hostile(fixture: &Fixture, marker: &Path) {
    let root = fixture.builder.root();
    let hooks = root.join("hooks");
    std::fs::create_dir_all(&hooks).expect("made");
    for hook in [
        "pre-push",
        "reference-transaction",
        "post-checkout",
        "pre-commit",
    ] {
        marking(&hooks, hook, marker);
    }
    let fsmonitor = marking(&root, "fsmonitor", marker);
    let clone = fixture.builder.clone_dir();
    git(&clone, &["config", "core.fsmonitor", &fsmonitor]).expect("set");
    git(
        &clone,
        &["config", "core.hooksPath", &hooks.display().to_string()],
    )
    .expect("set");
}

#[test]
fn host_the_branch_is_fetched_into_a_bare_repository_and_nothing_of_the_clones_runs() {
    // Given
    let fixture = fixture();
    let marker = fixture.home.path().join("ran");
    hostile(&fixture, &marker);

    // When
    let bare = fetched(&fixture.builder, &fixture.objects, ("api/b", 1)).expect("fetched");

    // Then
    let tip = |dir: &Path| git(dir, &["rev-parse", "refs/heads/api/b"]).expect("a tip");
    assert_eq!(tip(&bare), tip(&fixture.builder.clone_dir()));
    assert!(
        !marker.exists(),
        "something the session wrote ran on the host"
    );
}

#[test]
fn host_a_branch_not_in_the_clone_says_to_commit_on_it() {
    let fixture = fixture();
    let refused =
        fetched(&fixture.builder, &fixture.objects, ("api/other", 1)).expect_err("refused");
    assert!(
        refused.contains("commit your work on that branch"),
        "{refused}"
    );
}

#[test]
fn host_each_push_fetches_into_a_repository_of_its_own() {
    let fixture = fixture();
    let first = fetched(&fixture.builder, &fixture.objects, ("api/b", 1)).expect("fetched");
    let again = fetched(&fixture.builder, &fixture.objects, ("api/b", 1)).expect_err("refused");
    assert!(
        again.contains("each push starts in a new repository"),
        "{again}"
    );
    assert_ne!(
        first,
        fetched(&fixture.builder, &fixture.objects, ("api/b", 2)).expect("fetched")
    );
}

#[test]
fn a_clone_repointed_at_other_objects_is_refused_before_anything_is_fetched() {
    let repointings: [fn(&Path, &Path); 3] = [
        |git_dir, _| {
            std::fs::write(
                git_dir.join("objects/info/alternates"),
                "/home/u/private/.git/objects\n",
            )
            .expect("written");
        },
        |git_dir, elsewhere| {
            std::fs::rename(git_dir.join("objects/pack"), elsewhere.join("pack")).expect("moved");
            std::os::unix::fs::symlink(elsewhere.join("pack"), git_dir.join("objects/pack"))
                .expect("linked");
        },
        |git_dir, elsewhere| {
            std::fs::rename(git_dir, elsewhere.join("real.git")).expect("moved");
            std::fs::write(
                git_dir,
                format!("gitdir: {}\n", elsewhere.join("real.git").display()),
            )
            .expect("written");
        },
    ];
    for repoint in repointings {
        // Given
        let fixture = fixture();
        repoint(
            &fixture.builder.clone_dir().join(".git"),
            fixture.home.path(),
        );

        // When
        let refused =
            fetched(&fixture.builder, &fixture.objects, ("api/b", 1)).expect_err("refused");

        // Then
        assert!(refused.contains("nothing is fetched"), "{refused}");
        assert!(!fixture.builder.root().join("push-1.git").exists());
    }
}

#[test]
fn host_a_tree_git_would_refuse_to_check_out_is_refused_at_the_fetch() {
    // Given: a commit whose tree has an entry named `..`
    let fixture = fixture();
    let clone = fixture.builder.clone_dir();
    let blob = git(&clone, &["rev-parse", "HEAD:a.txt"]).expect("a blob");
    let mut tree = b"100644 ..\0".to_vec();
    tree.extend((0..20).map(|at| u8::from_str_radix(&blob[at * 2..at * 2 + 2], 16).expect("hex")));
    // the environment cleared, as a git hook exports GIT_DIR (CLAUDE.md)
    let mut hash = std::process::Command::new("git")
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .args(["hash-object", "-t", "tree", "--literally", "-w", "--stdin"])
        .current_dir(&clone)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("hash-object");
    hash.stdin
        .take()
        .expect("stdin")
        .write_all(&tree)
        .expect("written");
    let tree_id = String::from_utf8(hash.wait_with_output().expect("hashed").stdout).expect("text");
    let parent = git(&clone, &["rev-parse", "HEAD"]).expect("a tip");
    let bad = commit(
        &clone,
        &["commit-tree", tree_id.trim(), "-p", &parent, "-m", "dotdot"],
    );
    git(&clone, &["update-ref", "refs/heads/api/b", &bad]).expect("moved");

    // When
    let refused = fetched(&fixture.builder, &fixture.objects, ("api/b", 1)).expect_err("refused");

    // Then
    assert!(refused.contains("fetching branch api/b"), "{refused}");
}

#[test]
fn host_an_object_directory_linked_to_another_repository_hands_over_nothing() {
    // Given: the clone's fan-out directory for a blob that only another
    // repository on the host holds, linked there, and a commit that
    // adds and a commit that removes a file naming that blob (the
    // verify's probe 1)
    let fixture = fixture();
    let other = fixture.home.path().join("private");
    std::fs::create_dir_all(&other).expect("made");
    git(&other, &["init", "--quiet"]).expect("init");
    std::fs::write(other.join("secret.txt"), "private\n").expect("written");
    let blob = git(&other, &["hash-object", "-w", "secret.txt"]).expect("a blob");
    let clone = fixture.builder.clone_dir();
    let fan_out = |root: &Path| root.join(".git/objects").join(&blob[..2]);
    let _ = std::fs::remove_dir_all(fan_out(&clone));
    std::os::unix::fs::symlink(fan_out(&other), fan_out(&clone)).expect("linked");
    let entry = format!("100644,{blob},crates/secret.txt");
    git(&clone, &["update-index", "--add", "--cacheinfo", &entry]).expect("staged");
    commit(&clone, &["commit", "--quiet", "-m", "add"]);
    git(&clone, &["rm", "--quiet", "--cached", "crates/secret.txt"]).expect("unstaged");
    commit(&clone, &["commit", "--quiet", "-m", "remove"]);

    // When
    let fetched = fetched(&fixture.builder, &fixture.objects, ("api/b", 1));

    // Then: refused by upload-pack itself, which started in bwrap and
    // could not read the object (its pack-objects died: it ran, so bwrap
    // started), not by bwrap failing to start
    let refused = fetched.expect_err("refused");
    assert!(
        refused.contains("git upload-pack: git-pack-objects died"),
        "{refused}"
    );
    // and the blob is nowhere in the push repository
    let bare = fixture.builder.root().join("push-1.git");
    assert!(
        git(&bare, &["cat-file", "-e", &blob]).is_err(),
        "the other repository's blob was handed over"
    );
}

#[test]
fn an_alternates_file_that_is_a_fifo_or_a_link_is_refused_unread() {
    for swap in ["fifo", "link"] {
        // Given
        let fixture = fixture();
        let alternates = fixture
            .builder
            .clone_dir()
            .join(".git/objects/info/alternates");
        std::fs::remove_file(&alternates).expect("removed");
        if swap == "fifo" {
            let made = std::process::Command::new("mkfifo")
                .arg(&alternates)
                .status()
                .expect("mkfifo");
            assert!(made.success());
        } else {
            std::os::unix::fs::symlink("/dev/zero", &alternates).expect("linked");
        }

        // When
        let refused = fetched(&fixture.builder, &fixture.objects, ("api/b", 1)).expect_err(swap);

        // Then
        assert!(
            refused.contains("is not a file of at most 4096 bytes"),
            "{swap}: {refused}"
        );
    }
}
