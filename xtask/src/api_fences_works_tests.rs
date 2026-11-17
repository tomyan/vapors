use super::{Expected, Works, checked};
use crate::api_fences::{Ran, SENTINEL};

fn said(output: &str) -> Ran {
    Ran {
        success: true,
        said: format!("{SENTINEL}\n{output}"),
    }
}

fn check(expected: Expected) -> Works {
    Works {
        name: "a check",
        script: "true".into(),
        expected,
    }
}

#[test]
fn a_check_whose_output_is_what_it_must_be_works() {
    let lines = checked(&[check(Expected::Is("pong".into()))], &|_: &str| {
        Ok(said("pong"))
    });
    assert_eq!(lines, Ok(vec!["works inside: a check".to_owned()]));
}

#[test]
fn a_check_whose_output_differs_fails_saying_what_it_got() {
    let failed = checked(&[check(Expected::Is("0 1 2".into()))], &|_: &str| {
        Ok(said("0 1 2 3"))
    });
    let failed = failed.expect_err("failed");
    assert!(
        failed.contains("a check: `true` said \"0 1 2 3\""),
        "{failed}"
    );
}

#[test]
fn a_namespace_the_host_shares_fails_its_check() {
    // Given: the host's ipc namespace, and an inside that reports it too
    let host = vec!["ipc:[1]".to_owned()];

    // When
    let shared = checked(&[check(Expected::NoneOf(host.clone()))], &|_: &str| {
        Ok(said("ipc:[1]"))
    });
    let own = checked(&[check(Expected::NoneOf(host))], &|_: &str| {
        Ok(said("ipc:[2]"))
    });

    // Then
    assert!(shared.is_err());
    assert!(own.is_ok());
}
