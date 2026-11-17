use std::time::Duration;

use super::{PUSH_CALL_DEADLINE, session_name, time_left_to_push};

#[test]
fn a_session_name_a_ci_builder_could_take_is_refused() {
    assert_eq!(session_name("f2-slice"), Ok(()));
    let refused = session_name("f2-ci-1").expect_err("refused");
    assert!(
        refused.contains("`-ci-` names request_push's ci builders"),
        "{refused}"
    );
}

#[test]
fn the_push_starts_only_if_it_can_end_within_the_calls_deadline() {
    assert_eq!(time_left_to_push(Duration::from_mins(10)), Ok(()));
    let late = PUSH_CALL_DEADLINE.saturating_sub(Duration::from_secs(60));
    let refused = time_left_to_push(late).expect_err("refused");
    assert!(refused.contains("nothing was pushed"), "{refused}");
}

mod history {
    use std::path::Path;

    use super::super::Pusher;
    use crate::api_push_fetch::git;

    /// A repository whose `origin/main` is one commit, the pusher reading
    /// it, and `commits` (path, content or `None` to remove, message) on
    /// top; the tip.
    fn repository(dir: &Path, commits: &[(&str, Option<&str>, &str)]) -> (Pusher, String) {
        git(dir, &["init", "--quiet", "-b", "api/b"]).expect("init");
        let commit = |message: &str| {
            let identity = ["-c", "user.name=t", "-c", "user.email=t@t"];
            git(
                dir,
                &[
                    &identity[..],
                    &["commit", "--quiet", "--allow-empty", "-m", message],
                ]
                .concat(),
            )
            .expect("committed");
        };
        commit("base");
        git(dir, &["update-ref", "refs/remotes/origin/main", "HEAD"]).expect("main");
        for (path, content, message) in commits {
            match content {
                Some(content) => {
                    std::fs::create_dir_all(dir.join(path).parent().expect("a parent"))
                        .expect("made");
                    std::fs::write(dir.join(path), content).expect("written");
                    git(dir, &["add", path]).expect("added");
                }
                None => git(dir, &["rm", "--quiet", path])
                    .map(|_| ())
                    .expect("removed"),
            }
            commit(message);
        }
        let builder = crate::api_paths::Builder::new("b", dir).expect("a builder");
        let host = crate::api_sandbox::Host {
            xtask: "/x".into(),
            claude: "/c".into(),
            objects: "/o".into(),
            render_nodes: Vec::new(),
            advisories: Vec::new(),
        };
        let allowed = crate::api_push_checks::allowed(&[]).expect("defaults");
        let tip = git(dir, &["rev-parse", "HEAD"]).expect("a tip");
        (Pusher::new(dir.to_owned(), builder, host, allowed), tip)
    }

    #[test]
    fn a_commit_that_carries_what_a_later_one_removes_is_refused() {
        // Given: xtask/ added, then removed; the whole diff is empty
        let dir = tempfile::tempdir().expect("a scratch directory");
        let (pusher, tip) = repository(
            dir.path(),
            &[
                ("xtask/x.rs", Some("x\n"), "add"),
                ("xtask/x.rs", None, "remove"),
            ],
        );

        // When
        let refused = pusher.checked(dir.path(), &tip).expect_err("refused");

        // Then
        assert!(
            refused.contains("xtask/x.rs is outside crates/, docs/"),
            "{refused}"
        );
    }

    #[test]
    fn a_commit_message_that_closes_an_issue_is_refused() {
        let dir = tempfile::tempdir().expect("a scratch directory");
        let (pusher, tip) = repository(dir.path(), &[("crates/a.rs", Some("a\n"), "Fixes #7")]);
        let refused = pusher.checked(dir.path(), &tip).expect_err("refused");
        assert!(
            refused.starts_with("the pushed commits' names, emails and messages says"),
            "{refused}"
        );
    }

    #[test]
    fn an_author_email_holding_a_control_byte_is_refused() {
        // Given: an author email with an escape
        let dir = tempfile::tempdir().expect("a scratch directory");
        let (pusher, _) = repository(dir.path(), &[]);
        std::fs::write(dir.path().join("crates.rs"), "x").expect("written");
        git(dir.path(), &["add", "crates.rs"]).expect("added");
        let author = "--author=t <t\u{1b}[2J@t>";
        git(
            dir.path(),
            &[
                "-c",
                "user.name=t",
                "-c",
                "user.email=t@t",
                "commit",
                "--quiet",
                author,
                "-m",
                "m",
            ],
        )
        .expect("committed");
        let tip = git(dir.path(), &["rev-parse", "HEAD"]).expect("a tip");

        // When
        let refused = pusher.checked(dir.path(), &tip).expect_err("refused");

        // Then
        assert!(refused.contains("control character"), "{refused}");
    }

    #[test]
    fn a_line_of_clean_commits_passes_with_its_lines() {
        let dir = tempfile::tempdir().expect("a scratch directory");
        let (pusher, tip) = repository(dir.path(), &[("crates/a.rs", Some("a\nb\n"), "rows")]);
        assert_eq!(pusher.checked(dir.path(), &tip), Ok(2));
    }
}
