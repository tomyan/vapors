//! The dev loop's arguments and refusals, without trunk or tailscale.

use super::*;

#[test]
fn trunk_serves_a_debug_build_on_the_loopback_port_watching_the_root() {
    // When
    let args = trunk_args(8080, Path::new("/ws"), 9100);

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
            "/ws/target/web-dev",
            "--proxy-backend",
            "http://127.0.0.1:9100/dev/keys.json",
            "--proxy-rewrite",
            "/dev/keys.json"
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
    // Port 0 asks for any free port, so this cannot race another
    // process for a port this test just freed.
    assert_eq!(check_port_free(0), Ok(()));
    drop(listener);
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

/// A key file named for the test, with `mode`.
fn key_file(test: &str, mode: u32) -> std::path::PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let path = std::env::temp_dir().join(format!("xtask-dev-{test}-{}", std::process::id()));
    std::fs::write(&path, "sk-test-dev-key-0001").expect("temporary key file");
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(mode)).expect("its mode");
    path
}

#[test]
fn a_key_file_others_can_read_stops_dev_before_anything_starts() {
    // Given
    let file = key_file("open", 0o644);

    // When
    let refused = checked_keys(&[format!("openai={}", file.display())], &keyed());

    // Then
    let why = refused.expect_err("mode 644");
    assert!(why.contains("chmod 600"), "{why}");
    assert!(!why.contains("sk-test-dev-key-0001"), "{why}");
    let _ = std::fs::remove_file(file);
}

#[test]
fn checked_keys_keep_each_provider_and_file_in_order() {
    // Given
    let file = key_file("kept", 0o600);

    // When
    let keys = checked_keys(
        &[
            format!("anthropic={}", file.display()),
            format!("openai={}", file.display()),
        ],
        &keyed(),
    );

    // Then
    let providers: Vec<String> = keys
        .expect("both")
        .into_iter()
        .map(|k| k.provider)
        .collect();
    assert_eq!(providers, ["anthropic", "openai"]);
    let _ = std::fs::remove_file(file);
}

#[test]
fn a_key_typed_where_the_argument_goes_never_appears_in_the_refusal() {
    // Given: the key itself, alone, as the file, and as the provider
    let key = "sk-proj-NOTAFILE0123456789abcdef";
    for argument in [key.to_owned(), format!("openai={key}"), format!("{key}=x")] {
        // When
        let refused = checked_keys(std::slice::from_ref(&argument), &keyed());

        // Then
        let why = refused.expect_err(&argument);
        assert!(!why.contains("NOTAFILE"), "{argument:?} gave {why}");
        assert!(why.contains("<provider>=<file>"), "{why}");
    }
}

#[test]
fn a_provider_given_twice_is_refused_naming_both_files() {
    // Given
    let first = key_file("twice-a", 0o600);
    let second = key_file("twice-b", 0o600);

    // When
    let refused = checked_keys(
        &[
            format!("openai={}", first.display()),
            format!("openai={}", second.display()),
        ],
        &keyed(),
    );

    // Then
    let why = refused.expect_err("openai twice");
    assert!(why.contains(&first.display().to_string()), "{why}");
    assert!(why.contains(&second.display().to_string()), "{why}");
    let _ = std::fs::remove_file(first);
    let _ = std::fs::remove_file(second);
}

/// The keyed providers of the page's words file, as `[dev] words` gives them.
fn keyed() -> Vec<String> {
    vec!["anthropic".to_owned(), "openai".to_owned()]
}
