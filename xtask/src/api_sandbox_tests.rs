use std::path::{Path, PathBuf};

use super::{Host, arguments};
use crate::api_paths::Builder;

fn builder() -> Builder {
    Builder::new("b1", Path::new("/home/u")).expect("a plain name")
}

fn host() -> Host {
    Host {
        xtask: PathBuf::from("/home/u/projects/vapors/target/debug/xtask"),
        claude: PathBuf::from("/home/u/.local/share/claude/versions/2"),
        objects: PathBuf::from("/home/u/projects/vapors/.git/objects"),
        render_nodes: vec![PathBuf::from("/dev/dri/renderD128")],
    }
}

/// The destination of each mount of `kind`.
fn mounted(args: &[String], kind: &str) -> Vec<String> {
    let at = args.iter().enumerate().filter(|(_, arg)| *arg == kind);
    at.map(|(index, _)| args[index + 2].clone()).collect()
}

#[test]
fn the_session_has_no_network_and_sees_no_process_outside() {
    let args = arguments(&builder(), &host(), 4000);
    for flag in [
        "--unshare-net",
        "--unshare-pid",
        "--clearenv",
        "--die-with-parent",
    ] {
        assert!(
            args.iter().any(|arg| arg == flag),
            "{flag} missing: {args:?}"
        );
    }
}

#[test]
fn only_the_clone_its_target_its_cargo_home_and_sccache_are_writable() {
    // When
    let writable = mounted(&arguments(&builder(), &host(), 4000), "--bind");

    // Then
    assert_eq!(
        writable,
        [
            "/home/u/.cache/api-build/b1/cargo-home",
            "/home/u/.cache/api-build/b1/vapors",
            "/home/u/.cache/vapors-targets/api-b1",
            "/home/u/.cache/sccache-sandbox",
        ]
    );
}

#[test]
fn the_home_directory_is_hidden_before_anything_is_bound_back_into_it() {
    // Given
    let args = arguments(&builder(), &host(), 4000);

    // When
    let hidden = args
        .windows(2)
        .position(|pair| pair == ["--tmpfs", "/home/u"]);
    let first_bind = args.iter().position(|arg| arg.starts_with("/home/u/"));

    // Then
    let (hidden, first_bind) = (hidden.expect("home hidden"), first_bind.expect("a bind"));
    assert!(hidden < first_bind, "{args:?}");
}

#[test]
fn the_session_is_given_the_relay_and_a_dummy_key_never_a_key_file() {
    // When
    let args = arguments(&builder(), &host(), 4321);

    // Then
    let joined = args.join(" ");
    assert!(
        joined.contains("ANTHROPIC_BASE_URL http://127.0.0.1:4321"),
        "{joined}"
    );
    assert!(
        joined.contains("ANTHROPIC_API_KEY sk-ant-api-build-dummy"),
        "{joined}"
    );
    assert!(!joined.contains(".csvg-tok"), "{joined}");
}

#[test]
fn the_hosts_shared_sccache_directory_is_never_bound() {
    // When
    let args = arguments(&builder(), &host(), 4000);

    // Then: a cache entry written inside would be linked into a host build
    assert!(
        !args.iter().any(|arg| arg == "/home/u/.cache/sccache"),
        "{args:?}"
    );
}

#[test]
fn only_the_gpus_render_nodes_are_bound_not_its_card_nodes() {
    let devices = mounted(&arguments(&builder(), &host(), 4000), "--dev-bind");
    assert_eq!(devices, ["/dev/dri/renderD128"]);
}
