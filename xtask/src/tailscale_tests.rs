//! The tailscale pieces against canned status text and a PATH of their own.

use super::*;

#[test]
fn dns_name_comes_from_self_without_the_trailing_dot() {
    // Given
    let status = r#"{"Peer":{"a":{"DNSName":"other.ts.net."}},"Self":{"ID":"x","DNSName":"debian.tail.ts.net.","Online":true}}"#;

    // When
    let name = dns_name(status).expect("has a name");

    // Then
    assert_eq!(name, "debian.tail.ts.net");
}

#[test]
fn a_null_self_is_not_logged_in_and_a_peer_does_not_stand_in() {
    // Given
    let status = r#"{"Self":null,"Peer":{"a":{"DNSName":"other.ts.net."}}}"#;

    // When
    let error = dns_name(status).expect_err("not logged in");

    // Then
    assert!(error.contains("not logged in"), "{error}");
}

#[test]
fn a_status_without_self_is_refused() {
    // Given
    let status = r#"{"Peer":{}}"#;

    // When
    let error = dns_name(status).expect_err("no Self");

    // Then
    assert!(error.contains("no Self"), "{error}");
}

#[test]
fn text_that_is_not_json_is_refused() {
    // Given
    let status = "tailscale: command not found";

    // When
    let error = dns_name(status).expect_err("not JSON");

    // Then
    assert!(error.contains("not JSON"), "{error}");
}

#[test]
fn find_in_searches_the_given_path_in_order_for_a_file() {
    // Given: two directories, the second holding `tool`
    let base = std::env::temp_dir().join(format!("vapors-xtask-path-{}", std::process::id()));
    let (first, second) = (base.join("first"), base.join("second"));
    std::fs::create_dir_all(&first).expect("first");
    std::fs::create_dir_all(&second).expect("second");
    std::fs::write(second.join("tool"), "").expect("tool");
    let path = std::env::join_paths([&first, &second]).expect("joinable");

    // When / Then
    assert_eq!(find_in(Some(&path), "tool"), Some(second.join("tool")));
    assert_eq!(find_in(Some(&path), "no-such-program"), None);
    assert_eq!(find_in(None, "tool"), None);
    drop(std::fs::remove_dir_all(base));
}

#[test]
fn a_missing_tailscale_is_refused_with_where_to_get_it() {
    // When
    let error = require_tailscale_in(Some(OsStr::new("/nonexistent"))).expect_err("absent");

    // Then
    assert_eq!(
        error,
        "tailscale is not on PATH; install it from https://tailscale.com/download and log in"
    );
    assert!(require_tailscale_in(None).is_err());
}

#[test]
fn a_local_port_target_is_the_loopback_url_and_a_directory_its_path() {
    // When / Then
    assert_eq!(Target::LocalPort(8080).argument(), "http://127.0.0.1:8080");
    assert_eq!(
        Target::Directory(Path::new("/srv/web")).argument(),
        "/srv/web"
    );
}
