//! A test process killed with SIGKILL while it holds a browser, as nextest kills a
//! timed-out test: its process group. `Drop` never runs, so only the
//! browser being in that group stops it. The test re-runs this binary as
//! the holder (`helper_holds_a_browser_until_killed`). Its browser's
//! processes were gone 26 to 61 ms after the kill (3 runs, 2026-11-23);
//! the wait for them is [`KILLED_END_WITHIN`], 5 s.

use std::io::{BufRead, BufReader};
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use super::{chromium, entries, page, scratch};
use crate::comparison::chromium_pids::{KILLED_END_WITHIN, holding, kill_holding};
use crate::comparison::chromium_process::DEADLINE;

const HELPER: &str =
    "comparison::chromium_browser::tests::kill_tests::helper_holds_a_browser_until_killed";
const ROOT_ENV: &str = "VAPORS_BROWSER_HELPER_DIR";

/// How long the holder may take to have its browser up: one screenshot,
/// browser start included, has the launcher's deadline; 5 s more for the
/// holder process itself.
const READY_WITHIN: Duration = Duration::from_secs(DEADLINE.as_secs() + 5);

/// Run only by the test below, which sets [`ROOT_ENV`]; on its own it
/// returns at once.
#[test]
#[ignore = "the holder process of the SIGKILL test; it runs only from there"]
fn helper_holds_a_browser_until_killed() {
    let Some(dir) = std::env::var_os(ROOT_ENV).map(std::path::PathBuf::from) else {
        return;
    };
    let fine = page(&dir, "fine", "<p>held</p>");
    let mut browser = chromium(&dir, DEADLINE).browser();
    browser
        .screenshot(&fine, &dir.join("held.png"), 640, 400, 1)
        .unwrap_or_else(|e| panic!("{e}"));
    println!("ready");
    std::thread::sleep(READY_WITHIN);
}

/// The holder started in a process group of its own, as nextest runs a
/// test, its TMPDIR `tmp`, and whether it had its browser up by
/// [`READY_WITHIN`].
fn start_holder(dir: &Path, tmp: &Path) -> (Child, bool) {
    let exe = std::env::current_exe().expect("this test binary");
    let mut holder = Command::new(exe)
        .args([HELPER, "--exact", "--ignored", "--nocapture"])
        .env(ROOT_ENV, dir)
        .env("TMPDIR", tmp)
        .process_group(0)
        .stdout(Stdio::piped())
        .spawn()
        .expect("the holder starts");
    let stdout = holder.stdout.take().expect("piped");
    let (ready, said) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let up = BufReader::new(stdout)
            .lines()
            .map_while(Result::ok)
            .any(|line| line == "ready");
        let _ = ready.send(up);
    });
    let up = said.recv_timeout(READY_WITHIN) == Ok(true);
    (holder, up)
}

/// SIGKILL to `holder`'s process group, as nextest kills a timed-out
/// test; then up to [`KILLED_END_WITHIN`] for `profile`'s processes to
/// end. Whether the kill ran, and the processes left.
fn kill_group(holder: &mut Child, profile: &Path) -> (bool, Vec<u32>) {
    let group = format!("-{}", holder.id());
    let killed = Command::new("kill")
        .args(["-s", "KILL", "--", &group])
        .status()
        .is_ok_and(|s| s.success());
    let _ = holder.wait();
    let started = Instant::now();
    while !holding(profile).is_empty() && started.elapsed() < KILLED_END_WITHIN {
        std::thread::sleep(Duration::from_millis(20));
    }
    println!("browser gone {:?} after the kill", started.elapsed());
    let left = holding(profile);
    let _ = kill_holding(profile);
    (killed, left)
}

#[test]
fn a_test_process_killed_with_its_group_leaves_no_browser_process() {
    // Given: the holder with its browser up. Its TMPDIR is this test's:
    // it is killed, so its launch's TMPDIR is left, and goes when `held`
    // is dropped. Directly under the temp dir, as short as `launch_tmp`'s:
    // Chromium's socket path in it must fit in 108 bytes, which one
    // nested in `dir` did not under xtask's TMPDIR.
    let dir = scratch("killed");
    let held = tempfile::Builder::new()
        .prefix("vk-")
        .tempdir()
        .expect("the holder's TMPDIR");
    let (mut holder, up) = start_holder(dir.path(), held.path());
    let root = dir.path().join("profiles");
    let profile = root.join(entries(&root).first().cloned().unwrap_or_default());

    // When
    let (killed, left) = kill_group(&mut holder, &profile);

    // Then: what is left is the expiry-named profile, for the retention
    // job, and the launch's TMPDIR in the holder's
    assert!(up, "the holder never had its browser up");
    assert!(killed, "kill -s KILL -- -<holder>");
    assert_eq!(
        left,
        Vec::<u32>::new(),
        "browser processes outlived their test"
    );
    let names = entries(&root);
    assert!(
        names.len() == 1 && names[0].starts_with("expires-"),
        "{names:?}"
    );
    let tmps = entries(held.path());
    assert!(
        tmps.len() == 1 && tmps[0].starts_with("chromium-"),
        "the holder's TMPDIR: {tmps:?}"
    );
}
