use std::os::unix::fs::symlink;
use std::path::Path;

use super::*;
use crate::queue_targets_scratch::Scratch;

/// The porcelain line for a worktree on `branch`.
fn on(path: &Path, branch: &str) -> String {
    format!(
        "worktree {}\nHEAD abc\nbranch refs/heads/{branch}\n",
        path.display()
    )
}

#[test]
fn emptying_removes_the_contents_keeps_the_directory_and_follows_no_link() {
    // Given: a target with a file, a directory, and a link to a directory outside it
    let scratch = Scratch::new("empty");
    let target = scratch.dir("t/x");
    let outside = scratch.dir("outside");
    std::fs::write(outside.join("keep"), "kept").expect("file");
    std::fs::write(target.join("file"), "gone").expect("file");
    std::fs::write(scratch.dir("t/x/debug/deps").join("a"), "gone").expect("file");
    symlink(&outside, target.join("link")).expect("link");

    // When
    let removed = empty(&target);

    // Then
    assert_eq!(removed, Ok(3));
    assert_eq!(std::fs::read_dir(&target).expect("kept").count(), 0);
    assert!(outside.join("keep").exists());
}

#[test]
fn a_root_child_that_links_elsewhere_is_left_alone_and_its_destination_kept() {
    // Given: touche-targets/x links to precious, and the merged worktree's target to x
    let scratch = Scratch::new("precious");
    let root = scratch.dir("touche-targets");
    let precious = scratch.dir("precious");
    std::fs::write(precious.join("keep"), "kept").expect("file");
    let x = scratch.link("touche-targets/x", &precious);
    let worktree = scratch.repo("w/touche");
    scratch.link("w/touche/target", &x);

    // When
    let lines = empty_on(&on(&worktree, "b"), "b", &[root], &scratch.0.join("home"));

    // Then
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(
        lines[0].contains("which is itself a link; left alone; fix: "),
        "{lines:?}"
    );
    assert!(precious.join("keep").exists());
}

#[test]
fn the_home_directory_or_a_worktree_is_never_emptied() {
    // Given: a root that holds the home directory and another worktree
    let scratch = Scratch::new("forbidden");
    let root = scratch.0.clone();
    let home = scratch.dir("home");
    let other = scratch.repo("other");
    let worktree = scratch.repo("w/touche");
    scratch.repo("w/vapors");
    scratch.link("w/touche/target", &home);
    scratch.link("w/vapors/target", &other);
    let porcelain = format!("{}\n{}", on(&worktree, "b"), on(&other, "c"));

    // When
    let mut lines = empty_on(&porcelain, "b", &[root], &home);
    lines.sort();

    // Then
    assert_eq!(
        lines,
        [
            format!(
                "refused to empty {}: it is the home directory",
                home.display()
            ),
            format!("refused to empty {}: it is a worktree", other.display()),
        ]
    );
    assert!(other.join(".git").exists());
}

#[test]
fn the_root_directory_is_never_emptied() {
    assert_eq!(
        forbidden_as(Path::new("/"), &[]),
        Some("is the root directory".to_owned())
    );
}

#[test]
fn an_emptied_target_is_logged_with_its_entry_count_and_time() {
    // Given
    let scratch = Scratch::new("logged");
    let root = scratch.dir("touche-targets");
    std::fs::write(scratch.dir("touche-targets/x").join("a"), "gone").expect("file");
    let worktree = scratch.repo("w/touche");
    scratch.link("w/touche/target", &root.join("x"));

    // When
    let lines = empty_on(
        &on(&worktree, "b"),
        "b",
        std::slice::from_ref(&root),
        &scratch.0.join("home"),
    );

    // Then
    let start = format!(
        "emptied {} (1 top-level entry, in ",
        root.join("x").display()
    );
    assert!(
        lines.len() == 1 && lines[0].starts_with(&start) && lines[0].ends_with(" s)"),
        "{lines:?}"
    );
}

#[test]
fn a_directory_that_holds_a_worktree_is_never_emptied() {
    // Given: the review's scenario, a root holding worktrees and a target linking to their parent
    let scratch = Scratch::new("holds");
    let root = scratch.0.clone();
    let worktree = scratch.repo("w/touche");
    let sibling = scratch.repo("w/vapors");
    scratch.link("w/touche/target", &scratch.0.join("w"));
    let porcelain = format!("{}\n{}", on(&worktree, "b"), on(&sibling, "c"));

    // When
    let lines = empty_on(&porcelain, "b", &[root], &scratch.0.join("home"));

    // Then
    assert_eq!(
        lines,
        [format!(
            "refused to empty {}: it holds a worktree",
            scratch.0.join("w").display()
        )]
    );
    assert!(worktree.join(".git").exists() && sibling.join(".git").exists());
}
