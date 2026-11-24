//! Finding and killing a browser's processes by its profile argument.

use std::path::{Path, PathBuf};
use std::process::{Child, Command};

use super::*;

/// A shell with `arg` among its arguments that stops itself (SIGSTOP)
/// and so never ends but by a kill. `kill` is built in, so it starts no
/// process: a loop of `sleep` forked copies of the shell, holding `arg`
/// for a moment each, which the search then found too. A `read` of a
/// piped stdin ended by itself when `Child::wait` closed the pipe, so a
/// kill that did nothing still passed.
fn holder(arg: &str) -> Child {
    let child = Command::new("sh")
        .args(["-c", "kill -s STOP $$", "holder", arg])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .expect("sh starts");
    started(&child, arg);
    child
}

/// Wait until `child`'s command line holds `arg`: once in 15 runs one
/// was not found by a search straight after `spawn` returned.
fn started(child: &Child, arg: &str) {
    let path = format!("/proc/{}/cmdline", child.id());
    let since = std::time::Instant::now();
    while !std::fs::read(&path)
        .is_ok_and(|line| line.split(|&b| b == 0).any(|a| a == arg.as_bytes()))
    {
        assert!(
            since.elapsed() < KILLED_END_WITHIN,
            "{path} never held `{arg}`"
        );
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
}

fn profile(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("vapors-pids-{}-{name}", std::process::id()))
}

fn sorted(mut pids: Vec<u32>) -> Vec<u32> {
    pids.sort_unstable();
    pids
}

fn reap(children: &mut [Child]) {
    for child in children {
        let _ = child.kill();
        let _ = child.wait();
    }
}

#[test]
fn the_processes_holding_a_profile_are_found_by_the_whole_argument() {
    // Given: two holders of one profile, one of a profile that extends its
    // path, one of another
    let ours = profile("a");
    let flag = |p: &Path| format!("--user-data-dir={}", p.display());
    let mut children = vec![
        holder(&flag(&ours)),
        holder(&flag(&ours)),
        holder(&format!("{}-longer", flag(&ours))),
        holder(&flag(&profile("b"))),
    ];

    // When
    let found = sorted(holding(&ours).expect("/proc is readable"));

    // Then
    let wanted = sorted(vec![children[0].id(), children[1].id()]);
    reap(&mut children);
    assert_eq!(found, wanted);
}

#[test]
fn killing_a_profile_s_processes_ends_them_and_spares_the_rest_and_this_process() {
    // Given
    let (ours, theirs) = (profile("c"), profile("d"));
    let flag = |p: &Path| format!("--user-data-dir={}", p.display());
    let mut children = vec![
        holder(&flag(&ours)),
        holder(&flag(&ours)),
        holder(&flag(&theirs)),
    ];

    // When
    let killed = kill_holding(&ours);
    let left = holding(&ours);

    // Then: read before any wait on the holders, which cannot end by
    // themselves
    let spared = holding(&theirs).expect("/proc is readable");
    let others = children[2].id();
    reap(&mut children);
    assert_eq!(killed, Ok(()));
    assert_eq!(left, Ok(Vec::new()));
    assert_eq!(spared, vec![others]);
}

#[test]
fn a_profile_no_process_holds_is_killed_at_once() {
    // Given / When / Then
    assert_eq!(kill_holding(&profile("none")), Ok(()));
}

#[test]
fn a_title_rewritten_into_one_space_joined_argument_is_found_by_the_whole_argument() {
    // Given: Chromium's child processes rewrite their title, so their
    // command line is one string of space-joined arguments: one such
    // title holding the profile, one holding a profile that extends its path
    let ours = profile("e");
    let title =
        |flag: &str| format!("/usr/lib/chromium/chromium --type=zygote {flag} --lang=en-US");
    let mut children = vec![
        holder(&title(&format!("--user-data-dir={}", ours.display()))),
        holder(&title(&format!("--user-data-dir={}2", ours.display()))),
        holder(&format!("--user-data-dir={}", ours.display())),
    ];

    // When
    let found = sorted(holding(&ours).expect("/proc is readable"));

    // Then
    let wanted = sorted(vec![children[0].id(), children[2].id()]);
    reap(&mut children);
    assert_eq!(found, wanted);
}

#[test]
fn a_process_table_that_cannot_be_read_is_refused_and_an_ended_process_is_absent() {
    // Given: a table with one process holding the profile, one that has
    // ended (no command line), then one whose command line is unreadable
    let table = tempfile::tempdir().expect("a scratch process table");
    let ours = profile("f");
    let entry = |pid: &str, line: Option<&str>| {
        let dir = table.path().join(pid);
        std::fs::create_dir(&dir).expect("entry");
        if let Some(line) = line {
            std::fs::write(dir.join("cmdline"), line).expect("cmdline");
        }
    };
    entry(
        "10",
        Some(&format!("chromium\0--user-data-dir={}\0", ours.display())),
    );
    entry("11", None);
    let absent = holding_in(table.path(), &ours);
    std::fs::create_dir_all(table.path().join("12/cmdline")).expect("an unreadable entry");
    let unreadable = holding_in(table.path(), &ours);

    // When / Then
    assert_eq!(absent, Ok(vec![10]));
    let refusal = unreadable.expect_err("a command line that is a directory");
    assert!(refusal.contains("12/cmdline"), "{refusal}");
    let missing = holding_in(&table.path().join("none"), &ours).expect_err("no table");
    assert!(missing.starts_with("listing "), "{missing}");
}
