//! The trunk handle against a shell standing in for trunk.

use std::net::TcpListener;
use std::time::Instant;

use super::*;

/// A stand-in for trunk: a shell that runs `script`.
fn fake_trunk(script: &str) -> Trunk {
    let mut command = Command::new("sh");
    command.args(["-c", script]);
    Trunk::spawn(command).expect("sh runs")
}

fn quiet() -> Reporter {
    Reporter::new(false)
}

/// Whether `pid` still exists, asked with `kill -0`.
fn alive(pid: u32) -> bool {
    Command::new("kill")
        .args(["-0", &pid.to_string()])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

/// The kernel delivers a group signal after `kill` returns, so a check
/// right after the drop races it: poll until the process is gone.
/// Measured well under 50 ms on the dev box on 2026-10-22; 2 s leaves
/// room for a loaded CI runner and still names a leak within the test.
const GONE_DEADLINE: Duration = Duration::from_secs(2);

/// True once `pid` is gone, polling every 20 ms up to [`GONE_DEADLINE`].
fn gone_within_deadline(pid: u32) -> bool {
    let started = Instant::now();
    while alive(pid) {
        if started.elapsed() >= GONE_DEADLINE {
            return false;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    true
}

#[test]
fn the_ready_line_ends_the_wait() {
    // Given
    let mut trunk =
        fake_trunk("echo 'INFO starting build'; echo 'INFO server listening at:'; sleep 5");

    // When
    let ready = trunk.wait_until_ready(1, &quiet(), Duration::from_secs(5));

    // Then
    assert_eq!(ready, Ok(()));
}

#[test]
fn a_listening_port_ends_the_wait_without_the_ready_line() {
    // Given: nothing on trunk's output, but something on the port
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind");
    let port = listener.local_addr().expect("addr").port();
    let mut trunk = fake_trunk("sleep 5");

    // When
    let ready = trunk.wait_until_ready(port, &quiet(), Duration::from_secs(5));

    // Then
    assert_eq!(ready, Ok(()));
    drop(listener);
}

#[test]
fn a_line_that_is_not_utf8_does_not_stop_the_reader() {
    // Given
    let mut trunk =
        fake_trunk("printf '\\377\\376 bad\\n'; echo 'INFO server listening at:'; sleep 5");

    // When
    let ready = trunk.wait_until_ready(1, &quiet(), Duration::from_secs(5));

    // Then
    assert_eq!(ready, Ok(()));
}

#[test]
fn a_server_that_is_not_up_by_the_deadline_is_a_failure_with_the_last_lines() {
    // Given: trunk never prints its ready line, and port 1 answers nobody
    let mut trunk = fake_trunk("echo 'INFO starting build'; echo 'Compiling x'; sleep 30");

    // When
    let error = trunk
        .wait_until_ready(1, &quiet(), Duration::from_secs(1))
        .expect_err("timed out");

    // Then
    assert!(
        error.starts_with(
            "trunk did not print `server listening at:` or open 127.0.0.1:1 within 1s"
        ),
        "{error}"
    );
    assert!(error.contains("fix: run `trunk serve`"), "{error}");
    assert!(
        error.contains("its last 2 lines:\n     INFO starting build\n     Compiling x"),
        "{error}"
    );
    assert!(!alive(trunk.id()), "killed at the deadline");
}

#[test]
fn a_trunk_that_exits_before_the_ready_line_is_a_failure_with_the_last_lines() {
    // Given
    let mut trunk = fake_trunk("echo 'ERROR error from build pipeline'; exit 3");

    // When
    let error = trunk
        .wait_until_ready(1, &quiet(), Duration::from_secs(5))
        .expect_err("exited");

    // Then
    assert!(
        error.starts_with("trunk exited before its server was up (exit status: 3)"),
        "{error}"
    );
    assert!(error.contains("ERROR error from build pipeline"), "{error}");
}

#[test]
fn dropping_the_handle_kills_the_child() {
    // Given
    let trunk = fake_trunk("sleep 30");
    let pid = trunk.id();
    assert!(alive(pid));

    // When
    drop(trunk);

    // Then
    assert!(
        gone_within_deadline(pid),
        "the child {pid} outlived its handle by {GONE_DEADLINE:?}"
    );
}

#[cfg(unix)]
#[test]
fn dropping_the_handle_kills_the_childs_own_children_too() {
    // Given: the stand-in starts a build of its own and tells its pid
    let mut trunk = fake_trunk("sleep 30 & echo $!; wait");
    let build = trunk
        .next_line(Duration::from_secs(5))
        .ok()
        .flatten()
        .and_then(|line| line.trim().parse::<u32>().ok())
        .expect("the grandchild's pid");
    assert!(alive(build));

    // When
    drop(trunk);

    // Then
    assert!(
        gone_within_deadline(build),
        "the grandchild {build} outlived the group by {GONE_DEADLINE:?}"
    );
}

#[test]
fn the_last_forty_lines_are_kept_and_an_empty_history_says_so() {
    // Given
    let mut trunk = fake_trunk("sleep 1");
    assert_eq!(trunk.recent_lines(), "; trunk printed nothing");

    // When
    for n in 0..50 {
        trunk.remember(&format!("line {n}"));
    }

    // Then
    assert!(
        trunk
            .recent_lines()
            .starts_with("; its last 40 lines:\n     line 10\n")
    );
}
