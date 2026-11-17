use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use super::{origin_of, pr_text, pushed_to};
use crate::api_push_fetch::git;

/// A local http remote that requires a login: every request without an
/// `Authorization` header gets 401, one with it is recorded and gets
/// 403. Its URL and the headers it saw.
fn login_remote() -> (String, Arc<Mutex<Vec<String>>>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("a port");
    let url = format!(
        "http://{}/r.git",
        listener.local_addr().expect("an address")
    );
    let seen = Arc::new(Mutex::new(Vec::new()));
    let recorded = Arc::clone(&seen);
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let mut reader = BufReader::new(stream.try_clone().expect("a handle"));
            let mut login = None;
            let mut line = String::new();
            while reader.read_line(&mut line).is_ok_and(|read| read > 2) {
                if line.to_lowercase().starts_with("authorization:") {
                    login = Some(line.trim().to_owned());
                }
                line.clear();
            }
            let answer = match login {
                Some(login) => {
                    recorded.lock().expect("the record").push(login);
                    "HTTP/1.1 403 Forbidden\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                }
                None => {
                    "HTTP/1.1 401 Unauthorized\r\nWWW-Authenticate: Basic realm=\"r\"\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                }
            };
            let _ = (&stream).write_all(answer.as_bytes());
        }
    });
    (url, seen)
}

/// A bare repository at `dir` with one commit on `api/b`; its tip.
fn repository(dir: &Path) -> String {
    let work = dir.join("work");
    std::fs::create_dir_all(&work).expect("made");
    git(&work, &["init", "--quiet", "-b", "api/b"]).expect("init");
    std::fs::write(work.join("a.txt"), "a\n").expect("written");
    git(&work, &["add", "a.txt"]).expect("added");
    git(
        &work,
        &[
            "-c",
            "user.name=t",
            "-c",
            "user.email=t@t",
            "commit",
            "--quiet",
            "-m",
            "a",
        ],
    )
    .expect("committed");
    git(&work, &["rev-parse", "HEAD"]).expect("a tip")
}

/// A credential helper in `dir` that answers `get` with `answer`.
fn helper(dir: &Path, answer: &str) -> String {
    let script = dir.join("helper");
    std::fs::write(
        &script,
        format!("#!/bin/sh\n[ \"$1\" = get ] && printf '{answer}'\nexit 0\n"),
    )
    .expect("written");
    std::fs::set_permissions(&script, std::os::unix::fs::PermissionsExt::from_mode(0o755))
        .expect("mode");
    format!("!{}", script.display())
}

#[test]
fn the_push_alone_is_given_the_hosts_credential_and_sends_it() {
    // Given
    let dir = tempfile::tempdir().expect("a scratch directory");
    let tip = repository(dir.path());
    let (url, seen) = login_remote();

    // When
    let pushed = pushed_to(
        &dir.path().join("work"),
        (&url, dir.path()),
        (&tip, "api/b"),
        &helper(dir.path(), "username=u\\npassword=p\\n"),
    );

    // Then: the remote saw the helper's login (and refused it)
    assert!(pushed.is_err(), "the test remote refuses every push");
    assert!(
        seen.lock()
            .expect("the record")
            .iter()
            .any(|login| login.ends_with("Basic dTpw")),
        "{seen:?}"
    );
}

#[test]
fn a_push_with_no_credential_fails_at_once_instead_of_prompting() {
    // Given: a helper that knows no login
    let dir = tempfile::tempdir().expect("a scratch directory");
    let tip = repository(dir.path());
    let (url, _) = login_remote();
    let started = Instant::now();

    // When
    let refused = pushed_to(
        &dir.path().join("work"),
        (&url, dir.path()),
        (&tip, "api/b"),
        &helper(dir.path(), ""),
    )
    .expect_err("refused");

    // Then
    assert!(refused.contains("terminal prompts disabled"), "{refused}");
    assert!(
        started.elapsed() < Duration::from_secs(30),
        "{:?}",
        started.elapsed()
    );
}

#[test]
fn the_helper_is_keyed_to_the_remotes_origin_and_only_https_is_pushed_to() {
    assert_eq!(
        origin_of("https://github.com/tomyan/vapors.git"),
        Ok("https://github.com")
    );
    assert_eq!(
        origin_of("http://127.0.0.1:9/r.git"),
        Ok("http://127.0.0.1:9")
    );
    assert!(origin_of("git@github.com:tomyan/vapors.git").is_err());
}

#[test]
fn a_pr_text_that_mentions_closes_or_holds_control_bytes_is_refused() {
    assert_eq!(
        pr_text("F3: rows", "What it does.\n\n\tsee docs/plans/x.md"),
        Ok(())
    );
    for (title, body, named) in [
        ("t", "thanks @tomyan", "mentions"),
        ("t", "Fixes #12", "closes an issue"),
        ("t\x1b[2J", "b", "control character"),
    ] {
        let refused = pr_text(title, body).expect_err(named);
        assert!(refused.contains(named), "{refused}");
    }
}
