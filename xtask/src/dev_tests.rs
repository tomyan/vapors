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
fn port_zero_is_refused_naming_the_range() {
    // Given
    let zero = Config::parse("[dev]\nport = \"0\"\n").expect("valid");

    // When
    let error = port_from(&zero).expect_err("nothing listens on port 0");

    // Then
    assert_eq!(
        error,
        ".config/factory.toml: [dev] port = \"0\" is not a port number (1..=65535)"
    );
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
