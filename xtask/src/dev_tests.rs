//! The dev loop's arguments and refusals, without trunk or tailscale.

use super::*;

#[test]
fn trunk_serves_a_debug_build_on_the_loopback_port_watching_the_root() {
    // When
    let args = trunk_args(8080, Path::new("/ws"));

    // Then
    assert_eq!(
        args,
        [
            "serve",
            "--address",
            "127.0.0.1",
            "--port",
            "8080",
            "--watch",
            "/ws",
            "--dist",
            "/ws/target/web-dev"
        ]
    );
    assert!(!args.iter().any(|a| a == "--release"), "debug, for speed");
}

#[test]
fn the_port_defaults_to_8080_and_reads_dev_port() {
    // Given
    let unset = Config::parse("[web]\npackage = \"x\"\n").expect("valid");
    let set = Config::parse("[dev]\nport = \"9000\"\n").expect("valid");

    // When / Then
    assert_eq!(port_from(&unset), Ok(DEFAULT_PORT));
    assert_eq!(port_from(&set), Ok(9000));
}

#[test]
fn a_port_that_is_not_a_number_is_refused_naming_the_setting() {
    // Given
    let bad = Config::parse("[dev]\nport = \"eighty\"\n").expect("valid");

    // When
    let error = port_from(&bad).expect_err("not a number");

    // Then
    assert_eq!(
        error,
        ".config/factory.toml: [dev] port = \"eighty\" is not a port number (1..=65535)"
    );
    assert!(port_from(&Config::parse("[dev]\nport = \"70000\"\n").expect("valid")).is_err());
}

#[test]
fn a_port_in_use_is_refused_naming_it_and_a_free_one_passes() {
    // Given: something listening on an ephemeral port
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind");
    let port = listener.local_addr().expect("addr").port();

    // When
    let error = check_port_free(port).expect_err("in use");

    // Then
    assert!(
        error.starts_with(&format!("127.0.0.1:{port} is in use")),
        "{error}"
    );
    assert!(error.contains("[dev] port"), "{error}");
    drop(listener);
    assert_eq!(check_port_free(port), Ok(()));
}

#[test]
fn a_missing_trunk_is_refused_naming_the_setup_command() {
    // When
    let error = require_trunk_in(Some(OsStr::new("/nonexistent"))).expect_err("absent");

    // Then
    assert_eq!(
        error,
        "trunk is not on PATH; fix: `cargo xtask setup` installs the pinned version"
    );
    assert!(require_trunk_in(None).is_err());
}

/// A stand-in for trunk: a shell that prints `script`'s lines.
fn fake_trunk(script: &str) -> Child {
    let mut child = Command::new("sh")
        .args(["-c", script])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("sh runs");
    let _ = &mut child;
    child
}

#[test]
fn the_ready_line_ends_the_wait() {
    // Given
    let mut child =
        fake_trunk("echo 'INFO starting build'; echo 'INFO server listening at:'; sleep 5");
    let lines = stream_lines(&mut child);

    // When
    let ready = wait_until_ready(
        &mut child,
        &lines,
        &Reporter::new(false),
        Duration::from_secs(5),
    );

    // Then
    assert_eq!(ready, Ok(()));
    let _ = child.kill();
}

#[test]
fn a_server_that_is_not_up_by_the_deadline_is_a_failure_naming_trunk() {
    // Given: trunk never prints its ready line
    let mut child = fake_trunk("echo 'INFO starting build'; sleep 30");
    let lines = stream_lines(&mut child);

    // When
    let error = wait_until_ready(
        &mut child,
        &lines,
        &Reporter::new(false),
        Duration::from_millis(300),
    )
    .expect_err("timed out");

    // Then
    assert!(
        error.starts_with("trunk did not print `server listening at:` within 0s"),
        "{error}"
    );
    assert!(error.contains("fix: run `trunk serve`"), "{error}");
    assert!(
        child.try_wait().expect("waited").is_some(),
        "killed at the deadline"
    );
}

#[test]
fn a_trunk_that_exits_before_the_ready_line_is_a_failure() {
    // Given
    let mut child = fake_trunk("echo 'ERROR error from build pipeline'; exit 3");
    let lines = stream_lines(&mut child);

    // When
    let error = wait_until_ready(
        &mut child,
        &lines,
        &Reporter::new(false),
        Duration::from_secs(5),
    )
    .expect_err("exited");

    // Then
    assert!(
        error.starts_with("trunk exited before its server was up (exit status: 3)"),
        "{error}"
    );
}
