use std::path::PathBuf;

use super::*;
use crate::queue_targets_scratch::Scratch;

#[test]
fn worktrees_on_a_branch_are_read_from_the_porcelain_list() {
    // Given
    let porcelain = "worktree /p/touche\nHEAD aaa\nbranch refs/heads/main\n\nworktree /w1/touche\nHEAD bbb\nbranch refs/heads/b1\n\nworktree /w10/touche\nHEAD ccc\nbranch refs/heads/b10\n\nworktree /m/touche\nHEAD bbb\ndetached\n\nworktree /w1b/touche\nHEAD bbb\nbranch refs/heads/b1\n";

    // When
    let found = worktrees_on(porcelain, "b1");

    // Then
    assert_eq!(
        found,
        [PathBuf::from("/w1/touche"), PathBuf::from("/w1b/touche")]
    );
    assert_eq!(all_worktrees(porcelain).len(), 5);
}

#[test]
fn a_worktree_s_target_and_its_sibling_repository_s_are_found_in_their_roots() {
    // Given: w/touche and w/vapors are repositories whose targets link into two roots
    let scratch = Scratch::new("found");
    let (touche_root, vapors_root) = (scratch.dir("touche-targets"), scratch.dir("vapors-targets"));
    let worktree = scratch.repo("w/touche");
    scratch.repo("w/vapors");
    scratch.link("w/touche/target", &scratch.dir("touche-targets/x"));
    scratch.link("w/vapors/target", &scratch.dir("vapors-targets/x"));

    // When
    let mut dirs = target_dirs(&worktree, &[touche_root.clone(), vapors_root.clone()]);
    dirs.sort();

    // Then
    assert_eq!(dirs, [Ok(touche_root.join("x")), Ok(vapors_root.join("x"))]);
}

#[test]
fn a_sibling_that_is_not_a_repository_is_not_scanned() {
    // Given: w/notes has a target link too, but no .git
    let scratch = Scratch::new("notrepo");
    let root = scratch.dir("touche-targets");
    let worktree = scratch.repo("w/touche");
    scratch.dir("w/notes");
    scratch.link("w/notes/target", &scratch.dir("touche-targets/notes"));

    // Then
    assert_eq!(target_dirs(&worktree, &[root]), []);
}

#[test]
fn a_target_reached_through_a_link_to_its_root_resolves_to_the_root() {
    // Given: the worktree's target names the root by an alias that links to it
    let scratch = Scratch::new("alias");
    let root = scratch.dir("touche-targets");
    let alias = scratch.link("alias", &root);
    let worktree = scratch.repo("w/touche");
    scratch.dir("touche-targets/x");
    scratch.link("w/touche/target", &alias.join("x"));

    // Then
    assert_eq!(
        target_dirs(&worktree, std::slice::from_ref(&root)),
        [Ok(root.join("x"))]
    );
}

#[test]
fn a_target_that_is_not_a_link_to_a_directory_in_a_root_is_left_alone_naming_it() {
    // Given: a real directory, a link outside the roots, a root child that is
    // itself a link elsewhere, and a relative link
    let scratch = Scratch::new("alone");
    let root = scratch.dir("touche-targets");
    let precious = scratch.dir("precious");
    let real = scratch.dir(&format!("{}/target", scratch.repo("w/a").display()));
    let elsewhere = scratch.dir("elsewhere/x");
    let worktree = scratch.repo("w/b");
    let outside = scratch.link("w/b/target", &elsewhere);
    scratch.repo("w/c");
    let child_link = scratch.link("touche-targets/x", &precious);
    let hop = scratch.link("w/c/target", &child_link);
    scratch.repo("w/d");
    let relative = scratch.link("w/d/target", std::path::Path::new("../../touche-targets/y"));

    // When
    let mut dirs = target_dirs(&worktree, &[root]);
    dirs.sort();

    // Then
    let said = |path: &PathBuf, why: String| {
        Err(format!(
            "{} {why}; left alone; fix: link it to a directory directly in a targets root, as touche's CLAUDE.md worktree recipe does",
            path.display()
        ))
    };
    assert_eq!(
        dirs,
        [
            said(
                &real,
                "is a directory, not a link into a targets root".into()
            ),
            said(
                &outside,
                format!(
                    "links to {}, not a directory directly in a targets root",
                    elsewhere.display()
                )
            ),
            said(
                &hop,
                format!("links to {}, which is itself a link", child_link.display())
            ),
            said(
                &relative,
                "links to the relative path ../../touche-targets/y".into()
            ),
        ]
    );
}
